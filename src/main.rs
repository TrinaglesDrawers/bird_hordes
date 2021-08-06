use arcana::{anim::graph, camera::FreeCamera3Controller};
use {
    arcana::*,
    rapier3d::{
        dynamics::RigidBodyBuilder,
        geometry::{Collider, ColliderBuilder},
        na,
    },
};
use soundio;
extern crate hound;


use std::f64::consts::PI;
use std::io;

#[derive(Copy, Clone)]
struct SineWavePlayer {
    phase: f64, // Phase is updated each time the write callback is called.
    frequency: f64,
    amplitude: f64, // TODO: For some reason amplitude close to 1 (maybe > 0.99?) and high frequency (e.g. 18 kHz) gives weird low frequency aliasing or something.
}

impl SineWavePlayer {
    fn write_callback(&mut self, stream: &mut soundio::OutStreamWriter) {
        let mut frames_left = stream.frame_count_max();
        
        loop {
            if let Err(e) = stream.begin_write(frames_left) {
                println!("Error writing to stream: {}", e);
                return;
            }
            let phase_step = self.frequency / stream.sample_rate() as f64 * 2.0 * PI;

            for c in 0..stream.channel_count() {
                for f in 0..stream.frame_count() {
                    stream.set_sample(c, f, (self.phase.sin() * self.amplitude) as f32);
                    self.phase += phase_step;
                }
            }

            frames_left -= stream.frame_count();
            if frames_left <= 0 {
                break;
            }

            stream.end_write();
        }
    }
}



// Print sound soundio debug info and play back a sound.
fn run() -> Result<(), String> {
    println!("Soundio version: {}", soundio::version_string());

    let (major, minor, patch) = soundio::version();

    println!(
        "Major version: {}, minor version: {}, patch version: {}",
        major, minor, patch
    );

    let backend_list = [
        soundio::Backend::Jack,
        soundio::Backend::PulseAudio,
        soundio::Backend::Alsa,
        soundio::Backend::CoreAudio,
        soundio::Backend::Wasapi,
        soundio::Backend::Dummy,
    ];

    for &backend in backend_list.iter() {
        println!(
            "Backend {} available? {}",
            backend,
            soundio::have_backend(backend)
        );
    }

    println!(
        "InitAudioBackend error: {}",
        soundio::Error::InitAudioBackend
    );

    let mut ctx = soundio::Context::new();

    ctx.set_app_name("Sine Wave");

    println!("Available backends: {:?}", ctx.available_backends());

    ctx.connect()?;

    println!("Current backend: {:?}", ctx.current_backend());

    // We have to flush events so we can scan devices.
    ctx.flush_events();

    // Builtin and default layouts.

    let builtin_layouts = soundio::ChannelLayout::get_all_builtin();
    for layout in builtin_layouts {
        println!("Builtin layout: {:?}", layout);
    }

    let default_mono_layout = soundio::ChannelLayout::get_default(1);
    println!("Default mono layout: {:?}", default_mono_layout);
    let default_stereo_layout = soundio::ChannelLayout::get_default(2);
    println!("Default stereo layout: {:?}", default_stereo_layout);

    println!("Input device count: {}", ctx.input_device_count());
    println!("Output device count: {}", ctx.output_device_count());

    let output_devices = ctx
        .output_devices()
        .map_err(|_| "Error getting output devices".to_string())?;
    let input_devices = ctx
        .input_devices()
        .map_err(|_| "Error getting input devices".to_string())?;

    for dev in output_devices {
        println!(
            "Output device: {} {}",
            dev.name(),
            if dev.is_raw() { "raw" } else { "cooked" }
        );
    }

    for dev in input_devices {
        println!(
            "Input device: {} {}",
            dev.name(),
            if dev.is_raw() { "raw" } else { "cooked" }
        );
    }

    let output_dev = ctx
        .default_output_device()
        .map_err(|_| "Error getting default output device".to_string())?;

    println!(
        "Default output device: {} {}",
        output_dev.name(),
        if output_dev.is_raw() { "raw" } else { "cooked" }
    );

    let mut sine = SineWavePlayer {
        phase: 0.0,
        amplitude: 0.3,
        frequency: 200.0,
    };

    println!("Opening default output stream");
    let mut output_stream = output_dev.open_outstream(
        48000,
        soundio::Format::Float32LE,
        soundio::ChannelLayout::get_default(2),
        10.0,
        move |x| sine.write_callback(x),
        None::<fn()>,
        None::<fn(soundio::Error)>,
    )?;

    println!("Starting stream");
    output_stream.start()?;

    // Run the loop in a new thread.
    println!("Press enter to exit");
    let stdin = io::stdin();
    let input = &mut String::new();
    let _ = stdin.read_line(input);

    // Wait for key presses.
    Ok(())
}


use std::fs::File;
use std::io::BufReader;

// Maybe the best way to do this is something like:
//
// let (write_callback, wav_player) = WavPlayer::new();
//
// Internally they can use a mutex to communicate.
struct WavPlayer {
    reader: hound::WavReader<BufReader<File>>,
    finished: bool,
}

impl WavPlayer {
    fn write_callback(&mut self, stream: &mut soundio::OutStreamWriter) {
        let mut frames_left = stream.frame_count_max();
        let was_finished = self.finished;
        loop {
            if let Err(e) = stream.begin_write(frames_left) {
                println!("Error writing to stream: {}", e);
                return;
            }
            // Hound's sample conversion is not as awesome as mine. This will fail on floating point types.
            let mut s = self.reader.samples::<i32>();

            for f in 0..stream.frame_count() {
                for c in 0..stream.channel_count() {
                    match s.next() {
                        Some(x) => {
                            stream.set_sample(c, f, x.unwrap() * 1000);
                        }
                        None => {
                            stream.set_sample(c, f, 0);
                            self.finished = true;
                        }
                    }
                }
            }

            frames_left -= stream.frame_count();
            if frames_left <= 0 {
                break;
            }

            stream.end_write();
        }
        if self.finished != was_finished {
            //		stream.wakeup();
        }
    }
}

// TODO: I need some interior mutability and a mutex to make the write_callback work nicely.

// Print sound soundio debug info and play back a sound.
fn play(filename: &str) -> Result<(), String> {
    // Try to open the file.
    let reader = hound::WavReader::open(filename).map_err(|x| x.to_string())?;

    println!("Soundio version: {}", soundio::version_string());

    let mut ctx = soundio::Context::new();
    ctx.set_app_name("Player");
    ctx.connect()?;

    println!("Current backend: {:?}", ctx.current_backend());

    // We have to flush events so we can scan devices.
    println!("Flushing events.");
    ctx.flush_events();
    println!("Flushed");

    let channels = reader.spec().channels;
    let sample_rate = reader.spec().sample_rate;
    let int_or_float = reader.spec().sample_format;
    let bits_per_sample = reader.spec().bits_per_sample;

    // I guess these are always signed little endian?
    let soundio_format = match int_or_float {
        hound::SampleFormat::Int => match bits_per_sample {
            8 => soundio::Format::S8,
            16 => soundio::Format::S16LE,
            24 => soundio::Format::S24LE,
            32 => soundio::Format::S32LE,
            _ => return Err(format!("Unknown bit depth: {}", bits_per_sample)),
        },

        hound::SampleFormat::Float => match bits_per_sample {
            32 => soundio::Format::Float32LE,
            64 => soundio::Format::Float64LE,
            _ => return Err(format!("Unknown bit depth: {}", bits_per_sample)),
        },
    };

    let default_layout = soundio::ChannelLayout::get_default(channels as _);
    println!(
        "Default layout for {} channel(s): {:?}",
        channels, default_layout
    );

    let output_dev = ctx
        .default_output_device()
        .map_err(|_| "Error getting default output device".to_string())?;

    println!(
        "Default output device: {} {}",
        output_dev.name(),
        if output_dev.is_raw() { "raw" } else { "cooked" }
    );

    let mut player = WavPlayer {
        reader: reader,
        finished: false,
    };

    println!("Opening default output stream");
    let mut output_stream = output_dev.open_outstream(
        sample_rate as _,
        soundio_format,
        default_layout,
        2.0,
        |x| player.write_callback(x), // The trouble is this borrows &mut player, so I can't use it at all elsewhere. It's correct because player can be mutated. But I still want to read a value of it. The only solution is interior mutability.
        None::<fn()>,
        None::<fn(soundio::Error)>,
    )?;

    println!("Starting stream");
    output_stream.start()?;

    // Wait for key presses.
    println!("Press enter to stop playback");
    let stdin = io::stdin();
    let input = &mut String::new();
    let _ = stdin.read_line(input);

    Ok(())
}


fn main() {

    
    //match run() {
    //    Err(x) => println!("Error: {}", x),
    //    _ => {}
    //}
    //std::thread::spawn(||{run()});
    std::thread::spawn(||{play("/tmp/rec.wav")});


    game3(|mut game: arcana::Game| async move {
        game.renderer = Some(Box::new(renderer::vcolor::VcolorRenderer::new(
            &mut game.graphics,
        )?));

        game.scheduler.add_system(camera::FreeCameraSystem);

        let controller1 = EntityController::assume_control(
            FreeCamera3Controller::new(),
            10,
            game.viewport.camera(),
            &mut game.world,
        )?;

        game.world
            .insert(
                game.viewport.camera(),
                (Global3::identity(), camera::FreeCamera::new()),
            )
            .unwrap();

        game.control.add_global_controller(controller1);

        let mut handle = game
            .loader
            .load::<assets::object::Object>(
                &"18d6f877-88e8-46f9-b65d-ae23a4b70588".parse().unwrap(),
            )
            .await;
        let object = handle.get(&mut game.graphics)?;

        for i in -50..=50 {
            for j in -50..=50 {
                game.world.spawn((
                    object.primitives[0].mesh.clone(),
                    Global3::new(na::Translation3::new(i as f32, 0.0, j as f32).into()),
                ));
            }
        }

        Ok(game)
    });
    match run() {
        Err(x) => println!("Error: {}", x),
        _ => {}
    }
}
