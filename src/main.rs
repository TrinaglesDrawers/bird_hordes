use crate::systems::bunny_systems::BunnyMoveComponent;
use crate::systems::bunny_systems::BunnyTTLComponent;

use arcana::{anim::graph, camera::FreeCamera3Controller};
use rand::Rng;
use {
    arcana::*,
    rapier3d::{
        dynamics::RigidBodyBuilder,
        geometry::{Collider, ColliderBuilder},
        na,
    },
};

mod systems {
    pub mod bunny_systems;
}

#[derive(Clone, Debug)]
struct Bunny;

impl Bunny {
    // pub fn new(speed: f32, destination: na::Vector2<f32>) -> Self {
    //     Bunny { speed, destination }
    // }
    fn spawn(self, cx: TaskContext<'_>) -> hecs::Entity {
        // let cat = cx
        //     .loader
        //     .load::<assets::ImageAsset>(&"44a9ea43-7f4a-43e1-a7d9-2ee93c6aac84".parse().unwrap());

        // let sampler = cx
        //     .graphics
        //     .create_sampler(graphics::SamplerInfo::default())
        //     .unwrap();

        // // let mut handle = game
        // //     .loader
        // //     .load::<assets::object::Object>(
        // //         &"18d6f877-88e8-46f9-b65d-ae23a4b70588".parse().unwrap(),
        // //     )
        // //     .await;
        // // let object = handle.get(&mut game.graphics);
        let mut handle = cx.loader.load::<assets::object::Object>(
            &"42f9feac-d11a-4b2f-9c0b-358166237958".parse().unwrap(),
        );

        let mut rng = rand::thread_rng();
        let _speed: f32 = rng.gen_range(0.01..0.5);
        let entity = cx.world.spawn((
            self,
            Global3::new(
                na::Translation3::new(rng.gen_range(-10.0..10.0), 0.0, rng.gen_range(-10.0..10.0))
                    .into(),
                // na::Translation2::new(-1.0, -1.0).into(),
            ),
            BunnyMoveComponent {
                speed: _speed,
                destination: na::Vector3::new(0.0, 0.0, 0.0),
            },
            BunnyTTLComponent {
                ttl: rng.gen_range(2.0..16.0),
                lived: 0.0,
            },
            Scale {
                Scale: na::Vector3::new(1.0, 1.0, 1.0),
            },
        ));

        cx.spawner.spawn(async move {
            let mut handle = handle.await;

            let mut cx = AsyncTaskContext::new();
            let cx = cx.get();

            // let handle = handle.get(cx.graphics).unwrap().clone().into_inner();

            let object = handle.get(cx.graphics).unwrap();

            // let mut cx = AsyncTaskContext::new();
            // let cx = cx.get();

            // let handle = handle.get(cx.graphics).unwrap().clone().into_inner();
            println!("Mesh loaded {}", object.primitives[0].mesh.vertex_count());
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

        let start = 1000;

        for _ in 0..start {
            game.res.with(BunnyCount::default).count = start;

            let bunny = Bunny.spawn(game.cx());
        }

        // game.scheduler
        //     .add_system(systems::bunny_systems::BunnyMoveSystem);
        // game.scheduler
        //     .add_system(systems::bunny_systems::BunnyTTLSystem);

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

        // let mut handle = game
        //     .loader
        //     .load::<assets::object::Object>(
        //         &"f68f4069-fdd0-4748-9061-ce89ac8f716d".parse().unwrap(),
        //     )
        //     .await;
        // let object = handle.get(&mut game.graphics)?;

        // let mut rng = rand::thread_rng();
        // let _speed: f32 = rng.gen_range(0.01..0.5);

        // for _ in 0..start {
        //     let entity = game.world.spawn((
        //         object.primitives[0].mesh.clone(),
        //         Global3::new(
        //             na::Translation3::new(
        //                 rng.gen_range(-50.0..50.0),
        //                 0.0,
        //                 rng.gen_range(-50.0..50.0),
        //             )
        //             .into(),
        //             // na::Translation2::new(-1.0, -1.0).into(),
        //         ),
        //         BunnyMoveComponent {
        //             speed: _speed,
        //             destination: na::Vector3::new(0.0, 0.0, 0.0),
        //         },
        //         BunnyTTLComponent {
        //             ttl: rng.gen_range(2.0..16.0),
        //             lived: 0.0,
        //         },
        //     ));
        // }

        Ok(game)
    })
}
