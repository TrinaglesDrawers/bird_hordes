use arcana::bumpalo::collections::Vec as BVec;
use rand::Rng;
use rapier3d::prelude::*;
use {arcana::*, rapier3d::na};

use crate::Bunny;
use crate::BunnyCount;
use crate::GlobalTargets;
use crate::MapParams;
use pathfinding::prelude::astar;

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
    pub hops: Vec<Pos>,
    pub size: usize,
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Pos(i32, i32);

impl Pos {
    // fn distance(&self, other: &Pos) -> u32 {
    //   (absdiff(self.0, other.0) + absdiff(self.1, other.1)) as u32
    // }

    pub fn min_max_offset(size: usize) -> (i32, i32, i32, i32) {
        let mut xmin = 0;
        let mut xmax = 0;
        let mut ymin = 0;
        let mut ymax = 0;
        if size % 2 == 0 {
            xmin = -(size as f32 / 2.0) as i32 - 1;
            ymax = (size as f32 / 2.0) as i32 + 1;
        } else {
            xmin = -(size as f32 / 2.0) as i32;
            ymax = (size as f32 / 2.0) as i32;
        }

        xmax = (size as f32 / 2.0) as i32;
        ymin = -(size as f32 / 2.0) as i32;

        return (xmin, xmax, ymin, ymax);
    }

    fn successors(
        &self,
        grid: &pathfinding::grid::Grid,
        occupy: usize,
        diagonal_mode: bool,
    ) -> Vec<(Pos, u32)> {
        let &Pos(x, y) = self;

        let mut result: Vec<(Pos, u32)> = Vec::<(Pos, u32)>::new();

        let (xmin, xmax, ymin, ymax) = Pos::min_max_offset(occupy);

        // println!("Offsets: {}, {}; {}, {}", xmin, xmax, ymin, ymax);

        // let x =
        // let &(x, y) = vertex;
        // let mut candidates = Vec::with_capacity(8);
        if x + xmin > 0 {
            let mut can_move_left = true;
            for iy in ymin..ymax + 1 {
                let test_vertex = (x + xmin - 1, y + iy);
                if !grid.has_vertex(&(test_vertex.0 as usize, test_vertex.1 as usize)) {
                    can_move_left = false;
                    break;
                }
            }
            if can_move_left {
                result.push((Pos(x - 1, y), 2));
            }
            if diagonal_mode {
                if y + ymin > 0 {
                    let mut can_move_left_down = true;
                    for iy in ymin - 1..ymax {
                        let test_vertex = (x + xmin - 1, y + iy);
                        if !grid.has_vertex(&(test_vertex.0 as usize, test_vertex.1 as usize)) {
                            can_move_left_down = false;
                            break;
                        }
                    }
                    if can_move_left_down {
                        for ix in xmin - 1..xmax {
                            let test_vertex = (x + ix, y + ymin - 1);
                            if !grid.has_vertex(&(test_vertex.0 as usize, test_vertex.1 as usize)) {
                                can_move_left_down = false;
                                break;
                            }
                        }
                    }

                    if can_move_left_down {
                        result.push((Pos(x - 1, y - 1), 3));
                    }
                }
                if y + ymax + 1 < grid.height as i32 {
                    let mut can_move_left_up = true;
                    for iy in ymin + 1..ymax + 2 {
                        let test_vertex = (x + xmin - 1, y + iy);
                        if !grid.has_vertex(&(test_vertex.0 as usize, test_vertex.1 as usize)) {
                            can_move_left_up = false;
                            break;
                        }
                    }
                    if can_move_left_up {
                        for ix in xmin - 1..xmax {
                            let test_vertex = (x + ix, y + ymax + 1);
                            if !grid.has_vertex(&(test_vertex.0 as usize, test_vertex.1 as usize)) {
                                can_move_left_up = false;
                                break;
                            }
                        }
                    }

                    if can_move_left_up {
                        result.push((Pos(x - 1, y + 1), 3));
                    }
                }
            }
        }
        if x + xmax + 1 < grid.width as i32 {
            let mut can_move_right = true;
            for iy in ymin..ymax + 1 {
                let test_vertex = (x + xmax + 1, y + iy);
                if !grid.has_vertex(&(test_vertex.0 as usize, test_vertex.1 as usize)) {
                    can_move_right = false;
                    break;
                }
            }
            if can_move_right {
                result.push((Pos(x + 1, y), 2));
            }

            if diagonal_mode {
                if y + ymin > 0 {
                    let mut can_move_right_down = true;
                    for iy in ymin - 1..ymax {
                        let test_vertex = (x + xmax + 1, y + iy);
                        if !grid.has_vertex(&(test_vertex.0 as usize, test_vertex.1 as usize)) {
                            can_move_right_down = false;
                            break;
                        }
                    }
                    if can_move_right_down {
                        for ix in xmin + 1..xmax + 2 {
                            let test_vertex = (x + ix, y + ymin - 1);
                            if !grid.has_vertex(&(test_vertex.0 as usize, test_vertex.1 as usize)) {
                                can_move_right_down = false;
                                break;
                            }
                        }
                    }

                    if can_move_right_down {
                        result.push((Pos(x + 1, y - 1), 3));
                    }
                }
                if y + ymax + 1 < grid.height as i32 {
                    let mut can_move_right_up = true;
                    for iy in ymin + 1..ymax + 2 {
                        let test_vertex = (x + xmax + 1, y + iy);
                        if !grid.has_vertex(&(test_vertex.0 as usize, test_vertex.1 as usize)) {
                            can_move_right_up = false;
                            break;
                        }
                    }
                    if can_move_right_up {
                        for ix in xmin + 1..xmax + 2 {
                            let test_vertex = (x + ix, y + ymax + 1);
                            if !grid.has_vertex(&(test_vertex.0 as usize, test_vertex.1 as usize)) {
                                can_move_right_up = false;
                                break;
                            }
                        }
                    }

                    if can_move_right_up {
                        result.push((Pos(x + 1, y + 1), 3));
                    }
                }
            }
        }
        if y + ymin > 0 {
            let mut can_move_down = true;
            for ix in xmin..xmax + 1 {
                let test_vertex = (x + ix, y + ymin - 1);
                if !grid.has_vertex(&(test_vertex.0 as usize, test_vertex.1 as usize)) {
                    can_move_down = false;
                    break;
                }
            }
            if can_move_down {
                result.push((Pos(x, y - 1), 2));
            }
        }
        if y + ymax + 1 < grid.height as i32 {
            let mut can_move_up = true;
            for ix in xmin..xmax + 1 {
                let test_vertex = (x + ix, y + ymax + 1);
                if !grid.has_vertex(&(test_vertex.0 as usize, test_vertex.1 as usize)) {
                    can_move_up = false;
                    break;
                }
            }
            if can_move_up {
                result.push((Pos(x, y + 1), 2));
            }
        }
        // candidates.retain(|v| self.has_vertex(v));
        // candidates

        result

        // vec![Pos(x+1,y+2), Pos(x+1,y-2), Pos(x-1,y+2), Pos(x-1,y-2),
        //      Pos(x+2,y+1), Pos(x+2,y-1), Pos(x-2,y+1), Pos(x-2,y-1)]
        //      .into_iter().map(|p| (p, 1)).collect()
    }
}

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
            if movement.state != BunnyMovementState::Moving {
                continue;
            }
            // let v = &mut global.iso.translation.vector;

            let delta = cx.clock.delta.as_secs_f32();

            let params = cx.res.get::<MapParams>().unwrap();
            let global_targets = cx.res.get::<GlobalTargets>().unwrap();

            movement.move_lerp += delta * movement.speed;

            let _v = movement
                .start
                .lerp(&movement.destination, movement.move_lerp);

            // v.x = _v.x;
            // v.y = _v.y;
            // v.z = _v.z;

            let eye = na::Point3::new(
                global.iso.translation.vector.x,
                global.iso.translation.vector.y,
                global.iso.translation.vector.z,
            );
            let target = na::Point3::new(
                movement.destination.x,
                movement.destination.y,
                movement.destination.z,
            );
            let up = na::Vector3::y();

            // Isometry with its rotation part represented as a UnitQuaternion
            let iso = na::Isometry3::face_towards(&eye, &target, &up);
            // let r = na::UnitQuaternion::rotation_between()

            // let _r = global.iso.rotation.lerp(&iso.rotation, movement.move_lerp);

            *global = Global3::new(
                na::Isometry3::new(
                    na::Vector3::new(_v.x, _v.y, _v.z),
                    // *iso.rotation.axis().unwrap_or(global.iso.rotation.axis().unwrap().clone())
                    *iso.rotation.axis().unwrap_or(global.iso.rotation.axis().unwrap().clone())
                    // *iso.rotation.scaled_axis()
                )
                // na::Translation3::new(_v.x, _v.y, _v.z) * na::UnitQuaternion::from_quaternion(_r),
                // na::Translation3::new(_v.x, _v.y, _v.z) * iso.rotation,
            );

            let (xmin, xmax, ymin, ymax) = Pos::min_max_offset(coords.size);

            if movement.move_lerp >= 1.0 {
                movement.start = na::Vector3::new(
                    global.iso.translation.vector.x,
                    global.iso.translation.vector.y,
                    global.iso.translation.vector.z,
                );
                movement.move_lerp = 0.0;
                if !coords.hops.is_empty() {
                    // let next_coord = coords.hops.pop().unwrap();
                    let next_coord = coords.hops.remove(0);

                    let mut next_hop_reachable = true;
                    for ix in xmin..xmax + 1 {
                        for iy in ymin..ymax + 1 {
                            // grid.add_vertex(((next_coord.0 + ix) as usize, (next_coord.1 + iy) as usize));
                            if !grid.has_vertex(&(
                                (next_coord.0 + ix) as usize,
                                (next_coord.1 + iy) as usize,
                            )) {
                                next_hop_reachable = false;
                                break;
                            }
                        }
                        if !next_hop_reachable {
                            break;
                        }
                    }

                    if next_hop_reachable {
                        if coords.size % 2 == 0 {
                            movement.destination = na::Vector3::new(
                                (params.steps.0 * next_coord.0 as f32
                                    + params.steps.0 * (next_coord.0 + 1) as f32)
                                    / 2.0
                                    - params.physical_len.0 / 2.0,
                                0.0,
                                (params.steps.1 * next_coord.1 as f32
                                    + params.steps.1 * (next_coord.1 - 1) as f32)
                                    / 2.0
                                    - params.physical_len.1 / 2.0,
                            );
                        } else {
                            movement.destination = na::Vector3::new(
                                params.steps.0 * next_coord.0 as f32 - params.physical_len.0 / 2.0,
                                0.0,
                                params.steps.1 * next_coord.1 as f32 - params.physical_len.1 / 2.0,
                            );
                        }

                        for ix in xmin..xmax + 1 {
                            for iy in ymin..ymax + 1 {
                                grid.add_vertex((
                                    (coords.xcoord + ix) as usize,
                                    (coords.ycoord + iy) as usize,
                                ));
                            }
                        }
                        // grid.add_vertex((coords.xcoord as usize, coords.ycoord as usize));

                        coords.xcoord = next_coord.0;
                        coords.ycoord = next_coord.1;

                        if !global_targets
                            .targets
                            .contains(&(coords.xcoord, coords.ycoord))
                        {
                            for ix in xmin..xmax + 1 {
                                for iy in ymin..ymax + 1 {
                                    grid.remove_vertex(&(
                                        (next_coord.0 + ix) as usize,
                                        (next_coord.1 + iy) as usize,
                                    ));
                                }
                            }
                            // grid.remove_vertex(&(next_coord.0 as usize, next_coord.1 as usize));
                        }
                    } else {
                        movement.state = BunnyMovementState::Blocked;
                    }
                } else {
                    movement.state = BunnyMovementState::Idle;
                }

                let mut target_fetched = false;
                for ix in xmin..xmax + 1 {
                    for iy in ymin..ymax + 1 {
                        if global_targets
                            .targets
                            .contains(&((coords.xcoord + ix) as i32, (coords.ycoord + iy) as i32))
                        {
                            target_fetched = true;
                            break;
                        }
                    }
                    if target_fetched {
                        break;
                    }
                }
                if target_fetched {
                    despawn.push((_entity, coords.xcoord, coords.ycoord, coords.size));
                }
            }
            // println!("Position {}", v);
        }

        for e in despawn {
            // let mut grid = cx.res.get_mut::<pathfinding::grid::Grid>().unwrap();

            let (xmin, xmax, ymin, ymax) = Pos::min_max_offset(e.3);
            for ix in xmin..xmax + 1 {
                for iy in ymin..ymax + 1 {
                    grid.add_vertex(((e.1 + ix) as usize, (e.2 + iy) as usize));
                }
            }
            // grid.add_vertex((e.1 as usize, e.2 as usize));

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
        for (_entity, (movement, coords, behaviour)) in cx
            .world
            .query_mut::<(
                &mut BunnyMoveComponent,
                &mut BunnyGridComponent,
                &BunnyBehaviourComponent,
            )>()
            .with::<Bunny>()
        {
            if movement.state == BunnyMovementState::Moving {
                continue;
            }

            let (xmin, xmax, ymin, ymax) = Pos::min_max_offset(coords.size);

            let params = cx.res.get::<MapParams>().unwrap();
            let global_targets = cx.res.get::<GlobalTargets>().unwrap();

            if movement.state == BunnyMovementState::Blocked {
                if coords.hops.is_empty() {
                    movement.state = BunnyMovementState::Idle;
                    continue;
                }
                let goal = Pos(
                    coords.hops[coords.hops.len() - 1].0,
                    coords.hops[coords.hops.len() - 1].1,
                );

                // grid.add_vertex((coords.xcoord as usize, coords.ycoord as usize));
                let _neighbours_exists = false;
                for ix in xmin..xmax + 1 {
                    for iy in ymin..ymax + 1 {
                        grid.add_vertex((
                            (coords.xcoord + ix) as usize,
                            (coords.ycoord + iy) as usize,
                        ));
                    }
                }

                if grid
                    .neighbours(&(coords.xcoord as usize, coords.ycoord as usize))
                    .len()
                    == 0
                {
                    println!("OOOps grid size: {}", grid.vertices_len());
                    // grid.remove_vertex(&(coords.xcoord as usize, coords.ycoord as usize));
                    for ix in xmin..xmax + 1 {
                        for iy in ymin..ymax + 1 {
                            grid.remove_vertex(&(
                                (coords.xcoord + ix) as usize,
                                (coords.ycoord + iy) as usize,
                            ));
                        }
                    }
                    // movement.state = BunnyMovementState::Idle;
                    continue;
                } else if grid.neighbours(&(goal.0 as usize, goal.1 as usize)).len() == 0 {
                    println!("Double OOOps! grid size: {}", grid.vertices_len());
                    // grid.remove_vertex(&(coords.xcoord as usize, coords.ycoord as usize));
                    for ix in xmin..xmax + 1 {
                        for iy in ymin..ymax + 1 {
                            grid.remove_vertex(&(
                                (coords.xcoord + ix) as usize,
                                (coords.ycoord + iy) as usize,
                            ));
                        }
                    }
                    continue;
                }

                let path = astar(
                    &Pos(coords.xcoord, coords.ycoord),
                    |p| p.successors(&grid, coords.size, true),
                    // |p| {
                    //     grid.neighbours(&(p.0 as usize, p.1 as usize))
                    //         .into_iter()
                    //         .map(|p| ((p.0 as i32, p.1 as i32), 1))
                    // },
                    |p| {
                        grid.distance(
                            &(p.0 as usize, p.1 as usize),
                            &(goal.0 as usize, goal.1 as usize),
                        ) as u32
                    },
                    |p| *p == Pos(goal.0, goal.1),
                )
                .unwrap_or((Vec::<Pos>::new(), 0));
                // grid.remove_vertex(&(coords.xcoord as usize, coords.ycoord as usize));

                for ix in xmin..xmax + 1 {
                    for iy in ymin..ymax + 1 {
                        grid.remove_vertex(&(
                            (coords.xcoord + ix) as usize,
                            (coords.ycoord + iy) as usize,
                        ));
                    }
                }

                coords.hops = path.0;
                if coords.hops.is_empty() {
                    // println!("Path empty1 :(");
                } else {
                    coords.hops.remove(0);
                    movement.state = BunnyMovementState::Moving;
                }
            } else if movement.state == BunnyMovementState::Idle {
                let mut xcoord: i32 = 0;
                let mut ycoord: i32 = 0;

                if behaviour.state == BunnyBehaviourState::TargetLock {
                    if global_targets.targets.len() > 0 {
                        let mut chosen_target = global_targets.targets[0];
                        let mut max_distance = 9999.0 as usize;

                        for _target in &global_targets.targets {
                            let target_distance = grid.distance(
                                &(coords.xcoord as usize, coords.ycoord as usize),
                                &(_target.0 as usize, _target.1 as usize),
                            );

                            if target_distance < max_distance
                                && grid
                                    .neighbours(&(_target.0 as usize, _target.1 as usize))
                                    .len()
                                    > 0
                            {
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
                    while !grid.has_vertex(&(xcoord as usize, ycoord as usize)) {
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

                // grid.add_vertex((coords.xcoord as usize, coords.ycoord as usize));
                for ix in xmin..xmax + 1 {
                    for iy in ymin..ymax + 1 {
                        grid.add_vertex((
                            (coords.xcoord + ix) as usize,
                            (coords.ycoord + iy) as usize,
                        ));
                    }
                }
                let path = astar(
                    &Pos(coords.xcoord, coords.ycoord),
                    |p| p.successors(&grid, coords.size, true),
                    // |p| {
                    //     grid.neighbours(&(p.0 as usize, p.1 as usize))
                    //         .into_iter()
                    //         .map(|p| ((p.0 as i32, p.1 as i32), 0))
                    // },
                    |p| {
                        grid.distance(
                            &(p.0 as usize, p.1 as usize),
                            &(goal.0 as usize, goal.1 as usize),
                        ) as u32
                    },
                    |p| *p == Pos(goal.0, goal.1),
                )
                // .unwrap();
                .unwrap_or((Vec::<Pos>::new(), 0));

                coords.hops = path.0;

                // grid.remove_vertex(&(coords.xcoord as usize, coords.ycoord as usize));
                for ix in xmin..xmax + 1 {
                    for iy in ymin..ymax + 1 {
                        grid.remove_vertex(&(
                            (coords.xcoord + ix) as usize,
                            (coords.ycoord + iy) as usize,
                        ));
                    }
                }
                if coords.hops.is_empty() {
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
        let max_bunny = 100;
        if cx.res.get::<BunnyCount>().unwrap().count < max_bunny - 15 {
            for _ in 0..15 {
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
                despawn.push((_entity, coords.xcoord, coords.ycoord, coords.size));
            }
        }

        for e in despawn {
            let mut grid = cx.res.get_mut::<pathfinding::grid::Grid>().unwrap();

            // grid.add_vertex((e.1 as usize, e.2 as usize));
            let (xmin, xmax, ymin, ymax) = Pos::min_max_offset(e.3);
            for ix in xmin..xmax + 1 {
                for iy in ymin..ymax + 1 {
                    grid.add_vertex(((e.1 + ix) as usize, (e.2 + iy) as usize));
                }
            }

            let _ = cx.world.despawn(e.0);
            cx.res.with(BunnyCount::default).count -= 1;
        }

        Ok(())
    }
}

pub struct BunnyCollider(pub Collider);

impl BunnyCollider {
    pub fn new(height: f32, radius: f32) -> Self {
        BunnyCollider(
            ColliderBuilder::capsule_y(height, radius)
                .active_events(ActiveEvents::CONTACT_EVENTS)
                .build(),
        )
    }
}

#[derive(Debug)]
pub struct BunnyColliderSystem;

impl System for BunnyColliderSystem {
    fn name(&self) -> &str {
        "BunnyColliderSystem"
    }

    fn run(&mut self, cx: SystemContext<'_>) -> eyre::Result<()> {
        // let mut despawn = BVec::new_in(cx.bump);
        let physics = cx.res.get_mut::<PhysicsData3>().unwrap();

        for (_entity, (body_handler, global, contacts)) in cx
            .world
            .query_mut::<(&RigidBodyHandle, &Global3, &mut ContactQueue3)>()
            .with::<Bunny>()
        {
            // println!("I am alive. Contacts length: {}", contacts.len());
            for _other_collider in contacts.drain_contacts_started() {
                let bits = physics.colliders.get(_other_collider).unwrap().user_data as u64;
                // let bullet = cx.world.get::<Bullet>(Entity::from_bits(bits)).is_ok();

                // println!("I am alive");
                println!("Collided {}!", bits);
                // if bullet {
                //     tank.alive = false;
                // }
            }

            for _other_collider in contacts.drain_contacts_stopped() {
                let bits = physics.colliders.get(_other_collider).unwrap().user_data as u64;
                // let bullet = cx.world.get::<Bullet>(Entity::from_bits(bits)).is_ok();

                // println!("I am alive");
                println!("Uncollided {}!", bits);
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
