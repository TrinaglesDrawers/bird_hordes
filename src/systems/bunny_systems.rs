use arcana::bumpalo::collections::Vec as BVec;
use rand::Rng;
use {arcana::*, rapier3d::na};

use crate::Bunny;
use crate::BunnyCount;
use crate::GlobalTargets;
use crate::MapParams;
use pathfinding::prelude::{absdiff, astar};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BunnyMovementState {
    Idle,
    Moving,
    Blocked,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BunnyBehaviourState {
    Idle,
    Wandering,
    TargetLock,
    Following,
}

#[derive(Clone, Debug)]
pub struct BunnyBehaviourComponent {
    pub state: BunnyBehaviourState,
}

#[derive(Clone, Debug)]
pub struct BunnyMoveComponent {
    pub speed: f32,
    pub destination: na::Vector3<f32>,
    pub start: na::Vector3<f32>,
    pub move_lerp: f32,
    pub state: BunnyMovementState,
}

pub struct BunnyGridComponent {
    pub xcoord: i32,
    pub ycoord: i32,
    pub hops: Vec<(i32, i32)>,
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
struct Pos(i32, i32);

#[derive(Debug)]
pub struct BunnyMoveSystem;

impl System for BunnyMoveSystem {
    fn name(&self) -> &str {
        "BunnyMoveSystem"
    }

    fn run(&mut self, cx: SystemContext<'_>) -> eyre::Result<()> {
        let mut despawn = BVec::new_in(cx.bump);
        let mut grid = cx.res.get::<pathfinding::grid::Grid>().unwrap().clone();

        for (_entity, (global, movement, coords)) in cx
            .world
            .query_mut::<(
                &mut Global3,
                &mut BunnyMoveComponent,
                &mut BunnyGridComponent,
            )>()
            .with::<Bunny>()
        {
            if (movement.state != BunnyMovementState::Moving) {
                continue;
            }
            let mut v = &mut global.iso.translation.vector;
            let delta = cx.clock.delta.as_secs_f32();

            let params = cx.res.get::<MapParams>().unwrap();
            let globalTargets = cx.res.get::<GlobalTargets>().unwrap();

            movement.move_lerp += delta * movement.speed;

            let _v = movement
                .start
                .lerp(&movement.destination, movement.move_lerp);

            v.x = _v.x;
            v.y = _v.y;
            v.z = _v.z;

            if movement.move_lerp >= 1.0 {
                movement.start = na::Vector3::new(v.x, v.y, v.z);
                movement.move_lerp = 0.0;
                if !coords.hops.is_empty() {
                    // let next_coord = coords.hops.pop().unwrap();
                    let next_coord = coords.hops.remove(0);

                    if (grid.has_vertex(&(next_coord.0 as usize, next_coord.1 as usize))) {
                        movement.destination = na::Vector3::new(
                            params.steps.0 * next_coord.0 as f32 - params.physical_len.0 / 2.0,
                            0.0,
                            params.steps.1 * next_coord.1 as f32 - params.physical_len.1 / 2.0,
                        );

                        grid.add_vertex((coords.xcoord as usize, coords.ycoord as usize));

                        coords.xcoord = next_coord.0;
                        coords.ycoord = next_coord.1;

                        if !globalTargets
                            .targets
                            .contains(&(coords.xcoord, coords.ycoord))
                        {
                            grid.remove_vertex(&(next_coord.0 as usize, next_coord.1 as usize));
                        }
                    } else {
                        movement.state = BunnyMovementState::Blocked;
                    }
                } else {
                    movement.state = BunnyMovementState::Idle;

                    if globalTargets
                        .targets
                        .contains(&(coords.xcoord, coords.ycoord))
                    {
                        despawn.push((_entity, coords.xcoord, coords.ycoord));
                    }
                }
            }
            // println!("Position {}", v);
        }

        for e in despawn {
            // let mut grid = cx.res.get_mut::<pathfinding::grid::Grid>().unwrap();

            grid.add_vertex((e.1 as usize, e.2 as usize));

            let _ = cx.world.despawn(e.0);
            cx.res.with(BunnyCount::default).count -= 1;
        }

        cx.res.insert(grid);
        Ok(())
    }
}

#[derive(Debug)]
pub struct BunnyTargetingSystem;

impl System for BunnyTargetingSystem {
    fn name(&self) -> &str {
        "BunnyTargetingSystem"
    }

    fn run(&mut self, cx: SystemContext<'_>) -> eyre::Result<()> {
        let mut grid = cx.res.get::<pathfinding::grid::Grid>().unwrap().clone();
        for (_entity, (global, movement, coords, behaviour)) in cx
            .world
            .query_mut::<(
                &mut Global3,
                &mut BunnyMoveComponent,
                &mut BunnyGridComponent,
                &BunnyBehaviourComponent,
            )>()
            .with::<Bunny>()
        {
            if movement.state == BunnyMovementState::Moving {
                continue;
            }

            let params = cx.res.get::<MapParams>().unwrap();
            let globalTargets = cx.res.get::<GlobalTargets>().unwrap();

            if movement.state == BunnyMovementState::Blocked {
                if coords.hops.is_empty() {
                    movement.state = BunnyMovementState::Idle;
                    continue;
                }
                let goal = Pos(
                    coords.hops[coords.hops.len() - 1].0,
                    coords.hops[coords.hops.len() - 1].1,
                );

                grid.add_vertex((coords.xcoord as usize, coords.ycoord as usize));

                if grid
                    .neighbours(&(coords.xcoord as usize, coords.ycoord as usize))
                    .len()
                    == 0
                {
                    // println!("OOOps");
                    grid.remove_vertex(&(coords.xcoord as usize, coords.ycoord as usize));
                    // movement.state = BunnyMovementState::Idle;
                    continue;
                } else if grid.neighbours(&(goal.0 as usize, goal.1 as usize)).len() == 0 {
                    // println!("Double OOOps!");
                    grid.remove_vertex(&(coords.xcoord as usize, coords.ycoord as usize));
                    continue;
                }

                let path = astar(
                    &(coords.xcoord, coords.ycoord),
                    |p| {
                        grid.neighbours(&(p.0 as usize, p.1 as usize))
                            .into_iter()
                            .map(|p| ((p.0 as i32, p.1 as i32), 1))
                    },
                    |p| {
                        grid.distance(
                            &(p.0 as usize, p.1 as usize),
                            &(goal.0 as usize, goal.1 as usize),
                        )
                    },
                    |p| *p == (goal.0, goal.1),
                )
                .unwrap_or((Vec::<(i32, i32)>::new(), 0));
                grid.remove_vertex(&(coords.xcoord as usize, coords.ycoord as usize));

                coords.hops = path.0;
                if (coords.hops.is_empty()) {
                    // println!("Path empty1 :(");
                } else {
                    coords.hops.remove(0);
                    movement.state = BunnyMovementState::Moving;
                }
            } else if movement.state == BunnyMovementState::Idle {
                let mut xcoord: i32 = 0;
                let mut ycoord: i32 = 0;

                if behaviour.state == BunnyBehaviourState::TargetLock {
                    if globalTargets.targets.len() > 0 {
                        let mut chosen_target = globalTargets.targets[0];
                        let mut max_distance = 9999.0 as usize;

                        for _target in &globalTargets.targets {
                            let target_distance = grid.distance(
                                &(coords.xcoord as usize, coords.ycoord as usize),
                                &(_target.0 as usize, _target.1 as usize),
                            );

                            if target_distance < max_distance {
                                max_distance = target_distance;
                                chosen_target = _target.clone();
                            }
                        }

                        xcoord = chosen_target.0;
                        ycoord = chosen_target.1;
                    } else {
                        xcoord = coords.xcoord;
                        ycoord = coords.ycoord;
                    }
                } else if behaviour.state == BunnyBehaviourState::Wandering {
                    let mut rng = rand::thread_rng();

                    xcoord = rng.gen_range(0..params.tiles_dimension.0);
                    ycoord = rng.gen_range(0..params.tiles_dimension.1);

                    let max_tries = 3;

                    let mut try_count = 0;
                    while (!grid.has_vertex(&(xcoord as usize, ycoord as usize))) {
                        if try_count >= max_tries {
                            xcoord = coords.xcoord;
                            ycoord = coords.ycoord;
                            break;
                        }
                        xcoord = rng.gen_range(0..params.tiles_dimension.0);
                        ycoord = rng.gen_range(0..params.tiles_dimension.1);

                        try_count += 1;
                    }
                } else {
                    movement.state = BunnyMovementState::Idle;
                    continue;
                }

                // println!(
                //     "Trying to get path between {},{} and {},{}",
                //     coords.xcoord, coords.ycoord, xcoord, ycoord
                // );

                let goal = Pos(xcoord, ycoord);

                grid.add_vertex((coords.xcoord as usize, coords.ycoord as usize));
                let path = astar(
                    &(coords.xcoord, coords.ycoord),
                    |p| {
                        grid.neighbours(&(p.0 as usize, p.1 as usize))
                            .into_iter()
                            .map(|p| ((p.0 as i32, p.1 as i32), 0))
                    },
                    |p| {
                        grid.distance(
                            &(p.0 as usize, p.1 as usize),
                            &(goal.0 as usize, goal.1 as usize),
                        )
                    },
                    |p| *p == (goal.0, goal.1),
                )
                // .unwrap();
                .unwrap_or((Vec::<(i32, i32)>::new(), 0));

                coords.hops = path.0;

                grid.remove_vertex(&(coords.xcoord as usize, coords.ycoord as usize));
                if (coords.hops.is_empty()) {
                    // println!("Path empty2 :(");
                } else {
                    coords.hops.remove(0);
                    movement.state = BunnyMovementState::Moving;
                }
            }
        }
        cx.res.insert(grid);

        Ok(())
    }
}

pub struct BunnySpawnSystem;

impl System for BunnySpawnSystem {
    fn name(&self) -> &str {
        "BunnySpawnSystem"
    }

    fn run(&mut self, mut cx: SystemContext<'_>) -> eyre::Result<()> {
        let max_bunny = 512;
        if cx.res.get::<BunnyCount>().unwrap().count < max_bunny - 54 {
            for i in 0..54 {
                cx.res.with(BunnyCount::default).count += 1;
                Bunny.spawn(cx.task());
            }
        }
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

        for (_entity, (ttl, coords)) in cx
            .world
            .query_mut::<(&mut BunnyTTLComponent, &BunnyGridComponent)>()
            .with::<Bunny>()
        {
            let delta = cx.clock.delta.as_secs_f32();
            ttl.lived += delta;

            if ttl.lived >= ttl.ttl {
                despawn.push((_entity, coords.xcoord, coords.ycoord));
            }
        }

        for e in despawn {
            let mut grid = cx.res.get_mut::<pathfinding::grid::Grid>().unwrap();

            grid.add_vertex((e.1 as usize, e.2 as usize));

            let _ = cx.world.despawn(e.0);
            cx.res.with(BunnyCount::default).count -= 1;
        }

        Ok(())
    }
}
