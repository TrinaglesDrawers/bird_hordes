use arcana::event::DeviceEvent;
use arcana::event::ElementState;
use arcana::event::KeyboardInput;
use arcana::event::VirtualKeyCode;
use {arcana::*, rapier3d::na};

#[derive(Debug)]
pub enum BunnyCamera3Command {
    // RotateTo(na::UnitQuaternion<f32>),
    Move(na::Vector3<f32>),
}

pub struct BunnyCamera3Controller {
    // pitch: f32,
    // yaw: f32,
    forward_pressed: bool,
    backward_pressed: bool,
    left_pressed: bool,
    right_pressed: bool,
    up_pressed: bool,
    down_pressed: bool,
}

impl BunnyCamera3Controller {
    pub fn new() -> Self {
        BunnyCamera3Controller {
            // pitch: 0.0,
            // yaw: 0.0,
            forward_pressed: false,
            backward_pressed: false,
            left_pressed: false,
            right_pressed: false,
            up_pressed: false,
            down_pressed: false,
        }
    }
}

impl InputCommander for BunnyCamera3Controller {
    type Command = BunnyCamera3Command;

    fn translate(&mut self, event: DeviceEvent) -> Option<BunnyCamera3Command> {
        match event {
            // DeviceEvent::MouseMotion { delta: (x, y) } => {
            //     self.pitch -= (x * 0.001) as f32;
            //     self.yaw -= (y * 0.001) as f32;

            //     self.yaw = self.yaw.clamp(
            //         std::f32::consts::FRAC_PI_2 * (f32::EPSILON - 1.0),
            //         std::f32::consts::FRAC_PI_2 * (1.0 - f32::EPSILON),
            //     );

            //     while self.pitch < -std::f32::consts::PI {
            //         self.pitch += std::f32::consts::TAU
            //     }

            //     while self.pitch > std::f32::consts::PI {
            //         self.pitch -= std::f32::consts::TAU
            //     }

            //     Some(FreeCamera3Command::RotateTo(
            //         na::UnitQuaternion::from_euler_angles(0.0, self.pitch, 0.0)
            //             * na::UnitQuaternion::from_euler_angles(self.yaw, 0.0, 0.0),
            //     ))
            // }
            DeviceEvent::Key(KeyboardInput {
                virtual_keycode: Some(key),
                state,
                ..
            }) => {
                let pressed = matches!(state, ElementState::Pressed);

                match key {
                    VirtualKeyCode::W => self.forward_pressed = pressed,
                    VirtualKeyCode::S => self.backward_pressed = pressed,
                    VirtualKeyCode::A => self.left_pressed = pressed,
                    VirtualKeyCode::D => self.right_pressed = pressed,
                    VirtualKeyCode::LControl => self.up_pressed = pressed,
                    VirtualKeyCode::Space => self.down_pressed = pressed,
                    _ => return None,
                }

                let forward = (self.forward_pressed as u8 as f32) * -na::Vector3::z();
                let backward = (self.backward_pressed as u8 as f32) * na::Vector3::z();
                let left = (self.left_pressed as u8 as f32) * -na::Vector3::x();
                let right = (self.right_pressed as u8 as f32) * na::Vector3::x();
                let up = (self.up_pressed as u8 as f32) * -na::Vector3::y();
                let down = (self.down_pressed as u8 as f32) * na::Vector3::y();

                Some(BunnyCamera3Command::Move(
                    forward + backward + left + right + up + down,
                ))
            }
            _ => None,
        }
    }
}

pub struct BunnyCamera {
    speed: f32,
    mov: na::Vector3<f32>,
}

impl BunnyCamera {
    pub fn new(speed: f32) -> Self {
        BunnyCamera {
            speed,
            mov: na::Vector3::zeros(),
        }
    }
}

pub struct BunnyCameraSystem;

impl System for BunnyCameraSystem {
    fn name(&self) -> &str {
        "BunnyCameraSystem"
    }

    fn run(&mut self, cx: SystemContext<'_>) -> eyre::Result<()> {
        let query = cx.world.query_mut::<(
            &mut Global3,
            &mut BunnyCamera,
            &mut CommandQueue<BunnyCamera3Command>,
        )>();
        for (_, (global, camera, commands)) in query {
            for cmd in commands.drain() {
                match cmd {
                    // FreeCamera3Command::RotateTo(rot) => {
                    //     global.iso.rotation = rot;
                    // }
                    BunnyCamera3Command::Move(mov) => {
                        camera.mov = mov * camera.speed;
                    }
                }
            }

            global.iso.translation.vector += camera.mov * cx.clock.delta.as_secs_f32();
        }
        Ok(())
    }
}
