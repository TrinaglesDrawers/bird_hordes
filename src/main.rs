use crate::systems::bunny_systems::BunnyGridComponent;
use crate::systems::bunny_systems::BunnyMoveComponent;
use crate::systems::bunny_systems::BunnyTTLComponent;

use arcana::camera::FreeCamera3Controller;
use pathfinding;
use rand::Rng;
use {arcana::*, rapier3d::na};

mod systems {
    pub mod bunny_systems;
}

#[derive(Clone, Debug)]
struct Bunny;

impl Bunny {
    fn spawn(self, cx: TaskContext<'_>) -> hecs::Entity {
        let mut handle = cx.loader.load::<assets::object::Object>(
            // &"0cf76cc1-93f1-47d0-8687-45868725c4fa".parse().unwrap(),
            &"1c9762a5-26ff-40b9-a47e-b9c5621a771a".parse().unwrap(),
        );

        let mut grid = cx.res.get_mut::<pathfinding::grid::Grid>().unwrap();

        let scales = [
            arcana::graphics::Scale(na::Vector3::new(0.5, 1.0, 0.5)),
            arcana::graphics::Scale(na::Vector3::new(1.5, 1.2, 1.5)),
            arcana::graphics::Scale(na::Vector3::new(0.6, 0.6, 0.6)),
            arcana::graphics::Scale(na::Vector3::new(1.0, 1.0, 1.0)),
            arcana::graphics::Scale(na::Vector3::new(1.1, 2.0, 1.1)),
        ];
        let mut rng = rand::thread_rng();

        let mut xcoord = rng.gen_range(0..32);
        let mut ycoord = rng.gen_range(0..32);

        while (!grid.has_vertex(&(xcoord as usize, ycoord as usize))) {
            xcoord = rng.gen_range(0..32);
            ycoord = rng.gen_range(0..32);
        }

        let step = 20.0 / 32.0;

        // let _speed: f32 = rng.gen_range(0.05..0.5);
        let entity = cx.world.spawn((
            self,
            Global3::new(
                na::Translation3::new(
                    step * xcoord as f32 - 10.0,
                    0.0,
                    step * ycoord as f32 - 10.0,
                )
                .into(),
            ),
            BunnyMoveComponent {
                speed: 5.0,
                destination: na::Vector3::new(
                    step * xcoord as f32 - 10.0,
                    0.0,
                    step * ycoord as f32 - 10.0,
                ),
                start: na::Vector3::new(
                    step * xcoord as f32 - 10.0,
                    0.0,
                    step * ycoord as f32 - 10.0,
                ),
                move_lerp: 0.0,
            },
            BunnyTTLComponent {
                ttl: rng.gen_range(2.0..16.0),
                lived: 0.0,
            },
            BunnyGridComponent {
                xcoord: xcoord,
                ycoord: ycoord,
                hops: Vec::<(i32, i32)>::new(),
            },
            // object.primitives[0].mesh.clone(),
            scales[rng.gen_range(0..scales.len())],
        ));

        cx.spawner.spawn(async move {
            let mut handle = handle.await;

            let mut cx = AsyncTaskContext::new();
            let cx = cx.get();

            let object = handle.get(cx.graphics).expect(" --- ALARMA! --- ");

            cx.world
                .insert_one(entity, object.primitives[0].mesh.clone());

            Ok(())
        });

        // grid.remove_vertex(&(xcoord as usize, ycoord as usize));

        entity
    }
}

#[derive(Clone, Debug)]
struct Stone;

impl Stone {
    fn spawn(self, cx: TaskContext<'_>) -> hecs::Entity {
        let mut handle = cx.loader.load::<assets::object::Object>(
            &"0cf76cc1-93f1-47d0-8687-45868725c4fa".parse().unwrap(),
            // &"1c9762a5-26ff-40b9-a47e-b9c5621a771a".parse().unwrap(),
        );
        let mut grid = cx.res.get_mut::<pathfinding::grid::Grid>().unwrap();

        let mut rng = rand::thread_rng();

        let mut xcoord = rng.gen_range(0..32);
        let mut ycoord = rng.gen_range(0..32);

        while (!grid.has_vertex(&(xcoord as usize, ycoord as usize))) {
            xcoord = rng.gen_range(0..32);
            ycoord = rng.gen_range(0..32);
        }

        let step = 20.0 / 32.0;

        let entity = cx.world.spawn((
            self,
            Global3::new(
                na::Translation3::new(
                    step * xcoord as f32 - 10.0,
                    0.075,
                    step * ycoord as f32 - 10.0,
                )
                .into(),
            ),
            BunnyGridComponent {
                xcoord: xcoord,
                ycoord: ycoord,
                hops: Vec::<(i32, i32)>::new(),
            },
            // object.primitives[0].mesh.clone(),
            arcana::graphics::Scale(na::Vector3::new(0.25, 0.25, 0.25)),
        ));

        grid.remove_vertex(&(xcoord as usize, ycoord as usize));

        cx.spawner.spawn(async move {
            let mut handle = handle.await;

            let mut cx = AsyncTaskContext::new();
            let cx = cx.get();

            let object = handle.get(cx.graphics).expect(" --- ALARMA! --- ");

            cx.world
                .insert_one(entity, object.primitives[0].mesh.clone());

            Ok(())
        });

        entity
    }
}

#[derive(Default)]
struct BunnyCount {
    count: u32,
}

fn main() {
    game3(|mut game| async move {
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
                (
                    Global3::new(
                        na::Translation3::new(0.0, 15.0, 20.0)
                            * na::UnitQuaternion::from_axis_angle(
                                &na::Vector3::x_axis(),
                                // std::f32::consts::FRAC_PI_6,
                                -0.66,
                            ),
                    ),
                    // camera::FreeCamera::new(10.0),
                ),
            )
            .unwrap();

        game.control.add_global_controller(controller1);

        let mut grid = pathfinding::grid::Grid::new(32, 32);
        grid.enable_diagonal_mode();
        grid.fill();

        game.res.insert(grid);

        let mut handle = game
            .loader
            .load::<assets::object::Object>(
                &"0115fcef-c92c-431a-abc6-d4522c95e15a".parse().unwrap(),
            )
            .await;
        let object = handle.get(&mut game.graphics)?;
        let step = 20.0 / 32.0;
        for i in 0..32 {
            for j in 0..32 {
                game.world.spawn((
                    object.primitives[0].mesh.clone(),
                    Global3::new(
                        na::Translation3::new(step * i as f32 - 10.0, 0.0, step * j as f32 - 10.0)
                            .into(),
                    ),
                    arcana::graphics::Scale(na::Vector3::new(0.25, 0.25, 0.25)),
                ));
            }
        }

        let start = 64;

        for _ in 0..128 {
            let stome = Stone.spawn(game.cx());
        }

        for _ in 0..start {
            game.res.with(BunnyCount::default).count = start;

            let bunny = Bunny.spawn(game.cx());
        }

        game.scheduler
            .add_system(systems::bunny_systems::BunnyMoveSystem);
        game.scheduler
            .add_system(systems::bunny_systems::BunnyTTLSystem);

        game.scheduler
            .add_fixed_system(systems::bunny_systems::BunnySpawnSystem, TimeSpan::SECOND);

        game.scheduler.add_fixed_system(
            |mut cx: SystemContext<'_>| {
                if let Some(bunny) = cx.res.get::<BunnyCount>() {
                    println!("{} bunnies", bunny.count);
                }
            },
            TimeSpan::SECOND,
        );

        // game.scheduler.add_system(camera::FreeCameraSystem);

        // let mut object = game
        //     .loader
        //     .load::<assets::object::Object>(
        //         &"42f9feac-d11a-4b2f-9c0b-358166237958".parse().unwrap(),
        //     )
        //     .await;

        // dbg!(object.get(&mut game.graphics));

        Ok(game)
    })
}
