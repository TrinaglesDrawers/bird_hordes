use crate::na::Isometry3;
use crate::na::Vector3;
use crate::systems::bunny_camera::BunnyCamera;
use crate::systems::bunny_camera::BunnyCamera3Controller;
use crate::systems::bunny_camera::BunnyCameraSystem;
use crate::systems::bunny_systems::*;
use rapier3d::prelude::*;

use pathfinding;
use rand::{thread_rng, Rng};
use {arcana::*, rapier3d::na};

mod systems {
    pub mod bunnty_colliders_physics;
    pub mod bunny_camera;
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

        let mut res = cx.res;

        let params = res.get::<MapParams>().unwrap();
        let global_targets = res.get::<GlobalTargets>().unwrap();
        let mut grid = res.get::<pathfinding::grid::Grid>().unwrap().clone();

        let scales = [
            arcana::graphics::Scale(na::Vector3::new(0.5, 1.0, 0.5)),
            arcana::graphics::Scale(na::Vector3::new(0.6, 0.6, 0.6)),
            arcana::graphics::Scale(na::Vector3::new(1.0, 1.0, 1.0)),
            arcana::graphics::Scale(na::Vector3::new(1.1, 2.0, 1.1)),
            arcana::graphics::Scale(na::Vector3::new(1.5, 1.2, 1.5)),
        ];
        let mut rng = thread_rng();

        let scale_index = rng.gen_range(0..scales.len());
        let scale = scales[scale_index];
        let mut size = 1;
        if scale_index >= 3 {
            size = 3;
        } else if scale_index >= 2 {
            size = 2;
        }

        let mut can_spawn_at_position = false;

        let (xmin, xmax, ymin, ymax) = Pos::min_max_offset(size);
        let mut xcoord = 0;
        let mut ycoord = 0;

        while !can_spawn_at_position || global_targets.targets.contains(&(xcoord, ycoord)) {
            let variants = [
                (
                    rng.gen_range(0..params.tiles_dimension.0),
                    rng.gen_range(0..2),
                ),
                (
                    rng.gen_range(0..params.tiles_dimension.0),
                    rng.gen_range(params.tiles_dimension.1 - 2..params.tiles_dimension.1),
                ),
                (
                    rng.gen_range(0..2),
                    rng.gen_range(0..params.tiles_dimension.1),
                ),
                (
                    rng.gen_range(params.tiles_dimension.0 - 2..params.tiles_dimension.0),
                    rng.gen_range(0..params.tiles_dimension.1),
                ),
            ];
            let variant = rng.gen_range(0..4);
            xcoord = variants[variant].0;
            ycoord = variants[variant].1;

            can_spawn_at_position = true;
            for ix in xmin..xmax + 1 {
                for iy in ymin..ymax + 1 {
                    if !grid.has_vertex(&((xcoord + ix) as usize, (ycoord + iy) as usize)) {
                        can_spawn_at_position = false;
                        break;
                    }
                }
                if !can_spawn_at_position {
                    break;
                }
            }
        }

        let mut physics = res.get::<PhysicsData3>().unwrap();
        let mut collider_set = physics.colliders.clone();
        let mut bodies_set = physics.bodies.clone();

        let body = bodies_set.insert(
            RigidBodyBuilder::new_dynamic()
                // RigidBodyBuilder::new_kinematic_position_based()
                // .position(
                //     na::Translation3::new(
                //         params.steps.0 * xcoord as f32 - params.physical_len.0 / 2.0,
                //         0.0,
                //         params.steps.1 * ycoord as f32 - params.physical_len.1 / 2.0,
                //     )
                //     .into(),
                // )
                .additional_mass(size as f32)
                // .mass(size as f32)
                .linear_damping(0.3)
                .angular_damping(0.3)
                .build(),
        );

        collider_set.insert_with_parent(
            ColliderBuilder::capsule_y(scale.0.y, (params.steps.0 * size as f32) / 2.0)
                .active_events(ActiveEvents::all())
                .build(),
            body,
            &mut bodies_set,
        );

        let _speed: f32 = (20.0 - size as f32 * 2.0) / 5.0;
        let entity = cx.world.spawn((
            self,
            Global3::new(Isometry3::new(
                na::Vector3::new(
                    params.steps.0 * xcoord as f32 - params.physical_len.0 / 2.0,
                    0.0,
                    params.steps.1 * ycoord as f32 - params.physical_len.1 / 2.0,
                ), // .into(),
                Vector3::y() * std::f32::consts::FRAC_1_PI,
            )),
            // Global3::new(
            //     na::Translation3::new(
            //         params.steps.0 * xcoord as f32 - params.physical_len.0 / 2.0,
            //         0.0,
            //         params.steps.1 * ycoord as f32 - params.physical_len.1 / 2.0,
            //     )
            //     .into(),
            //     // Vector3::y() * std::f32::consts::FRAC_PI_2,
            // ),
            BunnyMoveComponent {
                speed: _speed,
                destination: na::Vector3::new(
                    params.steps.0 * xcoord as f32 - params.physical_len.0 / 2.0,
                    0.0,
                    params.steps.1 * ycoord as f32 - params.physical_len.1 / 2.0,
                ),
                start: na::Vector3::new(
                    params.steps.0 * xcoord as f32 - params.physical_len.0 / 2.0,
                    0.0,
                    params.steps.1 * ycoord as f32 - params.physical_len.1 / 2.0,
                ),
                move_lerp: 0.0,
                state: BunnyMovementState::Idle,
            },
            BunnyGridComponent {
                xcoord: xcoord,
                ycoord: ycoord,
                hops: Vec::<Pos>::new(),
                size: 1,
            },
            body,
            ContactQueue3::new(),
            // object.primitives[0].mesh.clone(),
            scale,
        ));

        if rng.gen_range(0..3) >= 1 {
            let _ = cx.world.insert_one(
                entity,
                BunnyBehaviourComponent {
                    state: BunnyBehaviourState::TargetLock,
                },
            );
        } else {
            let _ = cx.world.insert_one(
                entity,
                BunnyBehaviourComponent {
                    state: BunnyBehaviourState::Wandering,
                },
            );
            let _ = cx.world.insert_one(
                entity,
                BunnyTTLComponent {
                    ttl: rng.gen_range(4.0..32.0),
                    lived: 0.0,
                },
            );
        }

        cx.spawner.spawn(async move {
            let mut handle = handle.await;

            let mut cx = AsyncTaskContext::new();
            let cx = cx.get();

            let object = handle.get(cx.graphics).expect(" --- ALARMA! --- ");

            let _ = cx
                .world
                .insert_one(entity, object.primitives[0].mesh.clone());

            Ok(())
        });

        let new_physics = PhysicsData3 {
            bodies: bodies_set,
            colliders: collider_set,
            // colliders: ColliderSet::new(),
            islands: physics.islands.clone(),
            joints: physics.joints.clone(),
            gravity: physics.gravity,
        };

        grid.remove_vertex(&(xcoord as usize, ycoord as usize));
        res.insert(new_physics);
        res.insert(grid);

        entity
    }
}

#[derive(Clone, Debug)]
struct Stone;

impl Stone {
    fn spawn(self, cx: TaskContext<'_>) -> hecs::Entity {
        let handle = cx.loader.load::<assets::object::Object>(
            &"0cf76cc1-93f1-47d0-8687-45868725c4fa".parse().unwrap(),
            // &"1c9762a5-26ff-40b9-a47e-b9c5621a771a".parse().unwrap(),
        );

        let res = cx.res;
        let mut grid = res.get::<pathfinding::grid::Grid>().unwrap().clone();
        let params = res.get::<MapParams>().unwrap();
        let global_targets = res.get::<GlobalTargets>().unwrap();

        let mut rng = rand::thread_rng();

        let mut xcoord = 0; // = rng.gen_range(0..params.tiles_dimension.0);
        let mut ycoord = 0; // = rng.gen_range(0..params.tiles_dimension.1);

        let mut can_spawn_at_position = false;
        let (xmin, xmax, ymin, ymax) = Pos::min_max_offset(2);

        while !can_spawn_at_position || global_targets.targets.contains(&(xcoord, ycoord)) {
            xcoord = rng.gen_range(0..params.tiles_dimension.0);
            ycoord = rng.gen_range(0..params.tiles_dimension.1);

            can_spawn_at_position = true;
            for ix in xmin..xmax + 1 {
                for iy in ymin..ymax + 1 {
                    if !grid.has_vertex(&((xcoord + ix) as usize, (ycoord + iy) as usize)) {
                        can_spawn_at_position = false;
                        break;
                    }
                }
                if !can_spawn_at_position {
                    break;
                }
            }
        }

        let entity = cx.world.spawn((
            self,
            Global3::new(
                na::Translation3::new(
                    (params.steps.0 * xcoord as f32 + params.steps.0 * (xcoord + 1) as f32) / 2.0
                        - params.physical_len.0 / 2.0,
                    0.075,
                    (params.steps.1 * ycoord as f32 + params.steps.1 * (ycoord - 1) as f32) / 2.0
                        - params.physical_len.1 / 2.0,
                )
                .into(),
            ),
            BunnyGridComponent {
                xcoord: xcoord,
                ycoord: ycoord,
                hops: Vec::<Pos>::new(),
                size: 2,
            },
            // object.primitives[0].mesh.clone(),
            arcana::graphics::Scale(na::Vector3::new(0.25, 0.25, 0.25)),
        ));

        for ix in xmin..xmax + 1 {
            for iy in ymin..ymax + 1 {
                grid.remove_vertex(&((xcoord + ix) as usize, (ycoord + iy) as usize));
            }
        }

        res.insert(grid);

        cx.spawner.spawn(async move {
            let mut handle = handle.await;

            let mut cx = AsyncTaskContext::new();
            let cx = cx.get();

            let object = handle.get(cx.graphics).expect(" --- ALARMA! --- ");

            let _result = cx
                .world
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

#[derive(Clone, Debug)]
struct GlobalTargets {
    targets: Vec<(i32, i32)>,
}

#[derive(Clone, Debug)]
struct MapParams {
    tiles_dimension: (i32, i32),
    physical_min: (f32, f32),
    physical_max: (f32, f32),
    physical_len: (f32, f32),
    steps: (f32, f32),
}

fn main() {
    game3(|mut game| async move {
        game.renderer = Some(Box::new(renderer::vcolor::VcolorRenderer::new(
            &mut game.graphics,
        )?));

        game.scheduler.add_system(camera::FreeCameraSystem);

        let controller1 = EntityController::assume_control(
            BunnyCamera3Controller::new(),
            5,
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
                    BunnyCamera::new(5.0),
                ),
            )
            .unwrap();

        game.control.add_global_controller(controller1);

        game.scheduler
            .add_fixed_system(Physics3::new(), TimeSpan::MILLISECOND * 20);

        let mut grid = pathfinding::grid::Grid::new(128, 128);
        grid.enable_diagonal_mode();
        grid.fill();

        game.res.insert(grid);

        let mut params = MapParams {
            tiles_dimension: (128, 128),
            physical_min: (-20.0, -20.0),
            physical_max: (20.0, 20.0),
            physical_len: (0.0, 0.0),
            steps: (0.0, 0.0),
        };

        let physical_len = (
            params.physical_max.0 - params.physical_min.0,
            params.physical_max.1 - params.physical_min.1,
        );

        let steps = (
            physical_len.0 / params.tiles_dimension.0 as f32,
            physical_len.1 / params.tiles_dimension.1 as f32,
        );

        params.physical_len = physical_len;
        params.steps = steps;

        // let mut handle = game
        //     .loader
        //     .load::<assets::object::Object>(
        //         &"0115fcef-c92c-431a-abc6-d4522c95e15a".parse().unwrap(),
        //     )
        //     .await;
        // let object = handle.get(&mut game.graphics)?;
        // // let step = (params.physical_max.0 - params.physical_min.0) / params.tiles_dimension.0;
        // for i in 0..params.tiles_dimension.0 {
        //     for j in 0..params.tiles_dimension.1 {
        //         game.world.spawn((
        //             object.primitives[0].mesh.clone(),
        //             Global3::new(
        //                 na::Translation3::new(
        //                     steps.0 * i as f32 - physical_len.0 / 2.0,
        //                     0.0,
        //                     steps.1 * j as f32 - physical_len.1 / 2.0,
        //                 )
        //                 .into(),
        //             ),
        //             arcana::graphics::Scale(na::Vector3::new(0.25, 0.25, 0.25)),
        //         ));
        //     }
        // }

        let mut global_targets = GlobalTargets {
            targets: Vec::<(i32, i32)>::new(),
        };

        let targets_max_count = 5;

        let mut handle = game
            .loader
            .load::<assets::object::Object>(
                &"b42375dc-577d-4ec4-9006-44a1ee8850cd".parse().unwrap(),
            )
            .await;
        let object = handle.get(&mut game.graphics)?;

        let mut rng = rand::thread_rng();

        for _ in 0..rng.gen_range(1..targets_max_count + 1) {
            let xcoord =
                rng.gen_range(params.tiles_dimension.0 / 2 - 2..params.tiles_dimension.0 / 2 + 2);
            let ycoord =
                rng.gen_range(params.tiles_dimension.1 / 2 - 2..params.tiles_dimension.1 / 2 + 2);

            // while (!grid.has_vertex(&(xcoord as usize, ycoord as usize))) {
            //     xcoord = rng.gen_range(0..params.tiles_dimension.0);
            //     ycoord = rng.gen_range(0..params.tiles_dimension.1);
            // }

            game.world.spawn((
                object.primitives[0].mesh.clone(),
                Global3::new(
                    na::Translation3::new(
                        steps.0 * xcoord as f32 - physical_len.0 / 2.0,
                        0.0,
                        steps.1 * ycoord as f32 - physical_len.1 / 2.0,
                    )
                    .into(),
                ),
                arcana::graphics::Scale(na::Vector3::new(0.25, 0.25, 0.25)),
            ));

            global_targets.targets.push((xcoord, ycoord));
        }

        game.res.insert(PhysicsData3::new());
        game.res.insert(global_targets);

        game.res.insert(params);

        // let start = 1;

        for _ in 0..128 {
            let _stone = Stone.spawn(game.cx());
        }

        game.res.with(BunnyCount::default).count = 0;
        // for _ in 0..start {
        //     game.res.with(BunnyCount::default).count = start;

        //     let bunny = Bunny.spawn(game.cx());
        // }

        game.scheduler
            .add_system(systems::bunny_systems::BunnyMoveSystem);
        game.scheduler.add_fixed_system(
            systems::bunny_systems::BunnyTargetingSystem,
            TimeSpan::from_millis(333),
        );
        game.scheduler
            .add_system(systems::bunny_systems::BunnyTTLSystem);

        game.scheduler
            .add_fixed_system(systems::bunny_systems::BunnySpawnSystem, TimeSpan::SECOND);

        // let mut collider_set = ColliderSet::new();

        // /* Create the ground. */
        // let collider = ColliderBuilder::cuboid(100.0, 0.1, 100.0).build();
        // collider_set.insert(collider);

        game.scheduler.add_fixed_system(
            |cx: SystemContext<'_>| {
                if let Some(bunny) = cx.res.get::<BunnyCount>() {
                    println!("{} bunnies", bunny.count);
                }
            },
            TimeSpan::SECOND,
        );
        game.scheduler.add_system(BunnyColliderSystem);

        game.scheduler.add_system(BunnyCameraSystem);

        // arcana::game::MainWindow
        //     .window
        //     .set_cursor_grab(true)
        //     .unwrap();

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
