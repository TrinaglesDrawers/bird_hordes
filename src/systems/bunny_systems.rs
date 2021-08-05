use arcana::bumpalo::collections::Vec as BVec;
use rand::Rng;
use {arcana::*, rapier3d::na};

use crate::Bunny;
use crate::BunnyCount;

#[derive(Clone, Debug)]
pub struct BunnyMoveComponent {
    pub speed: f32,
    pub destination: na::Vector3<f32>,
}

#[derive(Debug)]
pub struct BunnyMoveSystem;

impl System for BunnyMoveSystem {
    fn name(&self) -> &str {
        "BunnyMoveSystem"
    }

    fn run(&mut self, cx: SystemContext<'_>) -> eyre::Result<()> {
        for (_entity, (global, movement)) in cx
            .world
            .query_mut::<(&mut Global3, &mut BunnyMoveComponent)>()
            .with::<Bunny>()
        {
            let v = &mut global.iso.translation.vector;
            let delta = cx.clock.delta.as_secs_f32();
            let direction =
                na::Vector2::new(movement.destination.x - v.x, movement.destination.y - v.y)
                    .normalize();

            v.x = v.x + direction.x * delta * movement.speed;
            v.z = v.z + direction.y * delta * movement.speed;

            let mut rng = rand::thread_rng();

            movement.destination = na::Vector3::new(
                movement.destination.x + rng.gen_range(-0.05..0.05),
                0.0,
                movement.destination.z + rng.gen_range(-0.05..0.05),
            );
        }
        Ok(())
    }
}

pub struct BunnySpawnSystem;

impl System for BunnySpawnSystem {
    fn name(&self) -> &str {
        "BunnySpawnSystem"
    }

    fn run(&mut self, mut cx: SystemContext<'_>) -> eyre::Result<()> {
        cx.res.with(BunnyCount::default).count += 1;
        Bunny.spawn(cx.task());
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct BunnyTTLComponent {
    pub ttl: f32,
    pub lived: f32,
}

#[derive(Debug)]
pub struct BunnyTTLSystem;

impl System for BunnyTTLSystem {
    fn name(&self) -> &str {
        "BunnyTTLSystem"
    }

    fn run(&mut self, cx: SystemContext<'_>) -> eyre::Result<()> {
        let mut despawn = BVec::new_in(cx.bump);

        for (_entity, ttl) in cx
            .world
            .query_mut::<&mut BunnyTTLComponent>()
            .with::<Bunny>()
        {
            let delta = cx.clock.delta.as_secs_f32();
            ttl.lived += delta;

            if ttl.lived >= ttl.ttl {
                despawn.push(_entity);
            }
        }

        for e in despawn {
            let _ = cx.world.despawn(e);
        }

        Ok(())
    }
}
