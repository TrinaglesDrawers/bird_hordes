use crate::hecs::Entity;
use crate::Bunny;
use crate::BunnyHealthComponent;

use arcana::bumpalo::collections::Vec as BVec;
use arcana::lifespan::LifeSpan;

use rapier3d::prelude::*;
use {arcana::*, rapier3d::na};

use crate::Tower;

#[derive(Debug)]
pub struct TowerColliderSystem;

impl System for TowerColliderSystem {
    fn name(&self) -> &str {
        "TowerColliderSystem"
    }

    fn run(&mut self, cx: SystemContext<'_>) -> eyre::Result<()> {
        // let mut despawn = BVec::new_in(cx.bump);
        let physics = cx.res.get_mut::<PhysicsData3>().unwrap();

        for (_entity, (collider_handle, intersections, attack)) in cx
            .world
            .query_mut::<(
                &ColliderHandle,
                &mut IntersectionQueue3,
                &mut TowerAttackComponent,
            )>()
            .with::<Tower>()
        {
            // println!("I am alive. Contacts length: {}", contacts.len());
            for _other_collider in intersections.drain_intersecting_started() {
                let bits = physics.colliders.get(_other_collider).unwrap().user_data as u64;
                // let bullet = cx.world.get::<Bullet>(Entity::from_bits(bits)).is_ok();
                if attack.target.is_none() {
                    attack.target = Some(bits);
                }

                // println!("I am alive");
                // println!("Collided {}!", bits);
                // if bullet {
                //     tank.alive = false;
                // }
            }

            for _other_collider in intersections.drain_intersecting_stopped() {
                if physics.colliders.get(_other_collider).is_some() {
                    let bits = physics.colliders.get(_other_collider).unwrap().user_data as u64;

                    if attack.target == Some(bits) {
                        attack.target = None;
                    }
                }

                // let bullet = cx.world.get::<Bullet>(Entity::from_bits(bits)).is_ok();

                // println!("I am alive");
                // println!("Uncollided {}!", bits);
                // if bullet {
                //     tank.alive = false;
                // }
            }

            // let mut collider = physics.colliders.get_mut(*collider_handle).unwrap();
            // collider.set_translation(global.iso.translation.vector);
        }
        Ok(())
    }
}

pub struct Bullet;

struct BulletCollider(Collider);

impl BulletCollider {
    fn new() -> Self {
        BulletCollider(
            ColliderBuilder::ball(0.1)
                .active_events(ActiveEvents::CONTACT_EVENTS)
                .density(1.3)
                .build(),
        )
    }
}

pub struct TowerAttackComponent {
    pub target: Option<u64>,
    pub power: f32,
    pub cooldown: f32,
    pub full_cooldown: f32,
}

pub struct TowerAttackSystem;

impl System for TowerAttackSystem {
    fn name(&self) -> &str {
        "TowerAttackSystem"
    }

    fn run(&mut self, cx: SystemContext<'_>) -> eyre::Result<()> {
        let mut tospawn = BVec::new_in(cx.bump);

        for (_entity, (global, attack)) in cx
            .world
            .query::<(&Global3, &mut TowerAttackComponent)>()
            .with::<Tower>()
            .iter()
        {
            if attack.target.is_none() {
                continue;
            }

            let bunny = cx
                .world
                .get::<Bunny>(Entity::from_bits(attack.target.unwrap()));

            if !bunny.is_ok() {
                attack.target = None;
                continue;
            }

            let delta = cx.clock.delta.as_secs_f32();

            attack.cooldown -= delta;

            let mut query = cx
                .world
                .query_one::<&Global3>(Entity::from_bits(attack.target.unwrap()))
                .unwrap();

            let global_other = query.get();
            if !global_other.is_some() {
                continue;
            }
            let global_other = global_other.unwrap();

            if attack.cooldown <= 0.0 {
                let collider = cx.res.with(BulletCollider::new).0.clone();
                let physics = cx.res.with(PhysicsData3::new);

                let spawning_point = na::Vector3::new(
                    global.iso.translation.vector.x,
                    global.iso.translation.vector.y + 0.3,
                    global.iso.translation.vector.z,
                );

                let dir = ((global_other.iso.translation.vector + na::Vector3::new(0.0, 0.1, 0.0))
                    - spawning_point)
                    .normalize();
                // println!(
                //     "Bullet spawning: {}; target: {}",
                //     spawning_point, global_other.iso.translation.vector
                // );

                let mut rb = RigidBodyBuilder::new_dynamic()
                    .position(na::Isometry3::new(
                        spawning_point,
                        na::Vector3::y() * std::f32::consts::FRAC_1_PI,
                    ))
                    .linear_damping(0.033)
                    .ccd_enabled(true)
                    .additional_mass(0.2)
                    // .linvel(dir * 15.0)
                    .build();

                rb.apply_force(dir * 150.0, true);
                let body = physics.bodies.insert(rb);
                physics
                    .colliders
                    .insert_with_parent(collider.clone(), body, &mut physics.bodies);

                tospawn.push((spawning_point, body, attack.power, dir));
                attack.cooldown = attack.full_cooldown;
            }
        }

        for e in tospawn {
            let handle = cx.loader.load::<assets::object::Object>(
                &"69df487b-6fea-4fec-8bc4-8e91eb7b1a93".parse().unwrap(),
            );
            let bullet_entity = cx.world.spawn((
                Global3::new(na::Isometry3::new(
                    e.0,
                    na::Vector3::y() * std::f32::consts::FRAC_1_PI,
                )),
                // Global3::new(na::Translation3::from(e.0).into()),
                Bullet,
                e.1,
                ContactQueue3::new(),
                LifeSpan::new(TimeSpan::SECOND * 3),
                arcana::graphics::Scale(na::Vector3::new(0.05, 0.05, 0.05)),
                BulletPowerComponent { power: e.2 },
            ));

            cx.spawner.spawn(async move {
                let mut handle = handle.await;

                let mut cx = AsyncTaskContext::new();
                let cx = cx.get();

                let object = handle.get(cx.graphics).expect(" --- ALARMA! --- ");

                let _result = cx
                    .world
                    .insert_one(bullet_entity, object.primitives[0].mesh.clone());

                Ok(())
            });
            // let mut grid = cx.res.get_mut::<pathfinding::grid::Grid>().unwrap();

            // grid.add_vertex((e.1 as usize, e.2 as usize));
            // let (xmin, xmax, ymin, ymax) = Pos::min_max_offset(e.3);
            // for ix in xmin..xmax + 1 {
            //     for iy in ymin..ymax + 1 {
            //         grid.add_vertex(((e.1 + ix) as usize, (e.2 + iy) as usize));
            //     }
            // }

            // let _ = cx.world.despawn(e.0);
            // cx.res.with(BunnyCount::default).count -= 1;
        }

        Ok(())
    }
}

pub struct BulletPowerComponent {
    pub power: f32,
}

pub struct BulletSystem;

impl System for BulletSystem {
    fn name(&self) -> &str {
        "BulletSystem"
    }

    fn run(&mut self, cx: SystemContext<'_>) -> eyre::Result<()> {
        let mut despawn = BVec::new_in(cx.bump);
        let mut bunny_hit = BVec::new_in(cx.bump);
        let physics = cx.res.get_mut::<PhysicsData3>().unwrap();

        for (e, (queue, power)) in cx
            .world
            .query::<(&mut ContactQueue3, &BulletPowerComponent)>()
            .with::<Bullet>()
            .iter()
        {
            // if queue.drain_contacts_started().count() > 0 {
            let mut first = true;
            for _other_collider in queue.drain_contacts_started() {
                if physics.colliders.get(_other_collider).is_some() {
                    let bits = physics.colliders.get(_other_collider).unwrap().user_data as u64;

                    let bunny = cx.world.get::<Bunny>(Entity::from_bits(bits));

                    if !bunny.is_ok() {
                        continue;
                    }

                    bunny_hit.push((bits, power.power));
                }

                if first {
                    despawn.push(e);
                    first = false;
                }
                // despawn.push(e);
            }
            // }
            queue.drain_contacts_stopped();
        }

        for e in despawn {
            let _ = cx.world.despawn(e);
        }

        for b in bunny_hit {
            let mut query = cx
                .world
                .query_one_mut::<&mut BunnyHealthComponent>(Entity::from_bits(b.0))
                .unwrap();
            let mut health = query;
            health.health -= b.1;
        }

        Ok(())
    }
}
