use arcana::bumpalo::collections::Vec as BVec;
use arcana::*;
use rapier3d::prelude::*;
// use std::sync::mpsc::Sender;
use {
    // arcana::clocks::TimeSpan,
    approx::relative_ne,
    // arcana::system::DEFAULT_TICK_SPAN,
    flume::{unbounded, Sender},
    hecs::Entity,
    rapier3d::{
        dynamics::{CCDSolver, IntegrationParameters, IslandManager, JointSet, RigidBodySet},
        geometry::{
            BroadPhase, ColliderHandle, ColliderSet, ContactEvent, ContactPair, IntersectionEvent,
            NarrowPhase,
        },
        na,
        pipeline::{CollisionPipeline, EventHandler},
    },
};

pub struct BunnyContactQueue3 {
    contacts_started: Vec<ColliderHandle>,
    contacts_stopped: Vec<ColliderHandle>,
}

impl BunnyContactQueue3 {
    pub const fn new() -> Self {
        BunnyContactQueue3 {
            contacts_started: Vec::new(),
            contacts_stopped: Vec::new(),
        }
    }

    pub fn drain_contacts_started(&mut self) -> std::vec::Drain<'_, ColliderHandle> {
        self.contacts_started.drain(..)
    }

    pub fn drain_contacts_stopped(&mut self) -> std::vec::Drain<'_, ColliderHandle> {
        self.contacts_stopped.drain(..)
    }
}

pub struct BunnyColliderPhysics3 {
    pipeline: CollisionPipeline,
    integration_parameters: IntegrationParameters,
    broad_phase: BroadPhase,
    narrow_phase: NarrowPhase,
    ccd_solver: CCDSolver,
}

pub struct BunnyColliderPhysicsData3 {
    pub bodies: RigidBodySet,
    pub colliders: ColliderSet,
    pub islands: IslandManager,
    pub joints: JointSet,
    pub gravity: na::Vector3<f32>,
}

impl BunnyColliderPhysicsData3 {
    pub fn new() -> Self {
        BunnyColliderPhysicsData3 {
            bodies: RigidBodySet::new(),
            colliders: ColliderSet::new(),
            islands: IslandManager::new(),
            joints: JointSet::new(),
            gravity: na::Vector3::default(),
        }
    }
}

impl Default for BunnyColliderPhysicsData3 {
    fn default() -> Self {
        Self::new()
    }
}

impl BunnyColliderPhysics3 {
    pub fn new() -> Self {
        BunnyColliderPhysics3::with_tick_span(TimeSpan::from_micros(20_000))
    }

    pub fn with_tick_span(tick_span: TimeSpan) -> Self {
        BunnyColliderPhysics3 {
            pipeline: CollisionPipeline::new(),
            integration_parameters: IntegrationParameters {
                dt: tick_span.as_secs_f32(),
                ..IntegrationParameters::default()
            },
            broad_phase: BroadPhase::new(),
            narrow_phase: NarrowPhase::new(),
            ccd_solver: CCDSolver::new(),
        }
    }
}

impl System for BunnyColliderPhysics3 {
    fn name(&self) -> &str {
        "BunnyColliderPhysics"
    }

    fn run(&mut self, cx: SystemContext<'_>) -> eyre::Result<()> {
        let data = cx.res.with(BunnyColliderPhysicsData3::new);

        let mut remove_bodies = BVec::with_capacity_in(data.bodies.len(), cx.bump);
        let world = &*cx.world;
        data.bodies.iter().for_each(|(handle, body)| {
            let e = Entity::from_bits(body.user_data as u64);
            if !world.contains(e) {
                remove_bodies.push(handle);
            }
        });
        for handle in remove_bodies {
            data.bodies.remove(
                handle,
                &mut data.islands,
                &mut data.colliders,
                &mut data.joints,
            );
        }

        for (entity, collider) in cx.world.query_mut::<&ColliderHandle>() {
            let collider = data.colliders.get_mut(*collider).unwrap();

            if collider.user_data == 0 {
                collider.user_data = entity.to_bits().into();

                collider.user_data = ((0 as u128) << 64) | entity.to_bits() as u128;
                // for (index, &collider) in body.colliders().iter().enumerate() {
                // }
            }
        }

        for (_entity, (global, collider)) in cx.world.query_mut::<(&Global3, &ColliderHandle)>() {
            let collider = data.colliders.get_mut(*collider).unwrap();

            if relative_ne!(*collider.position(), global.iso) {
                // collider.set_position(global.iso);
                collider.set_translation(global.iso.translation.vector);
            }
        }

        struct SenderEventHandler {
            tx: Sender<ContactEvent>,
        }

        impl EventHandler for SenderEventHandler {
            fn handle_intersection_event(&self, _event: IntersectionEvent) {}
            fn handle_contact_event(&self, event: ContactEvent, _pair: &ContactPair) {
                // println!("2");
                self.tx.send(event).unwrap();
            }
        }

        let (tx, rx) = unbounded();

        self.pipeline.step(
            0.1,
            // &data.gravity,
            // &self.integration_parameters,
            // &mut data.islands,
            &mut self.broad_phase,
            &mut self.narrow_phase,
            &mut data.bodies,
            &mut data.colliders,
            // &mut data.joints,
            // &mut self.ccd_solver,
            &(),
            &SenderEventHandler { tx },
        );

        for (_, (global, collider)) in cx.world.query::<(&mut Global3, &ColliderHandle)>().iter() {
            let collider = data.colliders.get_mut(*collider).unwrap();
            global.iso = *collider.position();
        }

        while let Ok(event) = rx.recv() {
            println!("1");
            match event {
                ContactEvent::Started(lhs, rhs) => {
                    let bits = data.colliders.get(lhs).unwrap().user_data as u64;
                    let entity = Entity::from_bits(bits);

                    if let Ok(mut queue) = cx.world.get_mut::<BunnyContactQueue3>(entity) {
                        queue.contacts_started.push(rhs);
                    }

                    let bits = data.colliders.get(rhs).unwrap().user_data as u64;
                    let entity = Entity::from_bits(bits);

                    if let Ok(mut queue) = cx.world.get_mut::<BunnyContactQueue3>(entity) {
                        queue.contacts_started.push(lhs);
                    }
                }
                ContactEvent::Stopped(lhs, rhs) => {
                    let bits = data.colliders.get(lhs).unwrap().user_data as u64;
                    let entity = Entity::from_bits(bits);

                    if let Ok(mut queue) = cx.world.get_mut::<BunnyContactQueue3>(entity) {
                        queue.contacts_stopped.push(rhs);
                    }

                    let bits = data.colliders.get(rhs).unwrap().user_data as u64;
                    let entity = Entity::from_bits(bits);

                    if let Ok(mut queue) = cx.world.get_mut::<BunnyContactQueue3>(entity) {
                        queue.contacts_stopped.push(lhs);
                    }
                }
            }
        }

        Ok(())
    }
}
