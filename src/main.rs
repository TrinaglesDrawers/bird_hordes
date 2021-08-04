use rand::thread_rng;
use rand::Rng;
use {
    arcana::*,
    rapier2d::{
        dynamics::RigidBodyBuilder,
        geometry::{Collider, ColliderBuilder},
        na,
    },
};

#[derive(Clone, Debug)]
struct Bunny;

#[derive(Clone, Debug)]
pub struct BunnyMoveComponent {
    speed: f32,
    destination: na::Vector2<f32>,
}

impl Bunny {
    // pub fn new(speed: f32, destination: na::Vector2<f32>) -> Self {
    //     Bunny { speed, destination }
    // }
    fn spawn(self, cx: TaskContext<'_>) -> hecs::Entity {
        let cat = cx
            .loader
            .load::<assets::ImageAsset>(&"44a9ea43-7f4a-43e1-a7d9-2ee93c6aac84".parse().unwrap());

        let sampler = cx
            .graphics
            .create_sampler(graphics::SamplerInfo::default())
            .unwrap();

        let mut rng = rand::thread_rng();
        let _speed: f32 = rng.gen_range(0.01..0.5);
        let entity = cx.world.spawn((
            self,
            graphics::Sprite {
                world: graphics::Rect {
                    left: -0.015,
                    right: 0.015,
                    top: -0.02,
                    bottom: 0.02,
                },
                ..graphics::Sprite::default()
            },
            Global2::new(
                na::Translation2::new(
                    rand::random::<f32>() * 1.5 - 0.75,
                    rand::random::<f32>() * 1.5 - 0.75,
                )
                .into(),
            ),
            BunnyMoveComponent {
                speed: _speed,
                destination: na::Vector2::new(0.0, 0.0),
            }, // body,
        ));

        cx.spawner.spawn(async move {
            let mut cat = cat.await;

            let mut cx = AsyncTaskContext::new();
            let cx = cx.get();

            let cat = cat.get(cx.graphics).unwrap().clone().into_inner();

            let material = graphics::Material {
                albedo_coverage: Some(graphics::Texture {
                    image: cat,
                    sampler,
                }),
                ..Default::default()
            };

            cx.world.insert_one(entity, material);
            Ok(())
        });

        entity
    }
}

fn main() {
    game2(|mut game| async move {
        let start = 10;

        for _ in 0..start {
            game.res.with(BunnyCount::default).count = start;

            let bunny = Bunny.spawn(game.cx());
        }

        game.scheduler.add_system(move |cx: SystemContext<'_>| {
            for (_entity, (global, movement)) in cx
                .world
                .query_mut::<(&mut Global2, &mut BunnyMoveComponent)>()
                .with::<Bunny>()
            {
                let v = &mut global.iso.translation.vector;
                let delta = cx.clock.delta.as_secs_f32();
                let direction =
                    na::Vector2::new(movement.destination.x - v.x, movement.destination.y - v.y)
                        .normalize();

                v.x = v.x + direction.x * delta * movement.speed;
                v.y = v.y + direction.y * delta * movement.speed;

                let mut rng = rand::thread_rng();

                movement.destination = na::Vector2::new(
                    movement.destination.x + rng.gen_range(-0.05..0.05),
                    movement.destination.y + rng.gen_range(-0.05..0.05),
                );

                // v.y -= cx.clock.delta.as_secs_f32();
                // if v.y <= -0.75 {
                //     v.y += 1.5;
            }
        });

        #[derive(Default)]
        struct BunnyCount {
            count: u32,
        }

        game.scheduler.add_fixed_system(
            |mut cx: SystemContext<'_>| {
                cx.res.with(BunnyCount::default).count += 1;
                Bunny.spawn(cx.task());
            },
            TimeSpan::SECOND,
        );

        game.scheduler.add_fixed_system(
            |mut cx: SystemContext<'_>| {
                if let Some(bunny) = cx.res.get::<BunnyCount>() {
                    println!("{} bunnies", bunny.count);
                }
            },
            TimeSpan::SECOND,
        );

        Ok(game)
    })
}
