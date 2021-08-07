use arcana::bumpalo::collections::Vec as BVec;
use rand::Rng;
use {arcana::*, rapier3d::na};

use crate::Bunny;
use crate::BunnyCount;
use crate::MapParams;
use pathfinding::prelude::{absdiff, astar};

#[derive(Clone, Debug)]
pub struct BunnyMoveComponent {
    pub speed: f32,
    pub destination: na::Vector3<f32>,
    pub start: na::Vector3<f32>,
    pub move_lerp: f32,
}

pub struct BunnyGridComponent {
    pub xcoord: i32,
    pub ycoord: i32,
    pub hops: Vec<(i32, i32)>,
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
struct Pos(i32, i32);

// impl Pos {
//   fn distance(&self, other: &Pos) -> u32 {
//     (absdiff(self.0, other.0) + absdiff(self.1, other.1)) as u32
//   }

//   fn successors(&self) -> Vec<(Pos, u32)> {
//     let &Pos(x, y) = self;
//     vec![Pos(x+1,y+2), Pos(x+1,y-2), Pos(x-1,y+2), Pos(x-1,y-2),
//          Pos(x+2,y+1), Pos(x+2,y-1), Pos(x-2,y+1), Pos(x-2,y-1)]
//          .into_iter().map(|p| (p, 1)).collect()
//   }
// }

#[derive(Debug)]
pub struct BunnyMoveSystem;

impl System for BunnyMoveSystem {
    fn name(&self) -> &str {
        "BunnyMoveSystem"
    }

    fn run(&mut self, cx: SystemContext<'_>) -> eyre::Result<()> {
        for (_entity, (global, movement, coords)) in cx
            .world
            .query_mut::<(
                &mut Global3,
                &mut BunnyMoveComponent,
                &mut BunnyGridComponent,
            )>()
            .with::<Bunny>()
        {
            let mut v = &mut global.iso.translation.vector;
            let delta = cx.clock.delta.as_secs_f32();

            let params = cx.res.get::<MapParams>().unwrap();

            movement.move_lerp += delta * movement.speed;

            // let _v = na::Vector3::new(
            //     step * coords.xcoord as f32 - 10.0,
            //     0.0,
            //     step * coords.ycoord as f32 - 10.0,
            // );
            let _v = movement
                .start
                .lerp(&movement.destination, movement.move_lerp);
            // println!("Move lerp = {}", movement.move_lerp);

            // let direction =
            //     na::Vector2::new(movement.destination.x - v.x, movement.destination.z - v.z)
            //         .normalize();

            // if (direction != na::Vector2::new(0.0, 0.0)) {
            //     v.x = v.x + direction.x * delta * movement.speed;
            //     v.z = v.z + direction.y * delta * movement.speed;
            // }

            v.x = _v.x;
            v.y = _v.y;
            v.z = _v.z;

            // if na::distance(
            //     &na::Point3::new(v.x, v.y, v.z),
            //     &na::Point3::new(
            //         movement.destination.x,
            //         movement.destination.y,
            //         movement.destination.z,
            //     ),
            // ) <= 0.0000001
            // {
            if movement.move_lerp >= 1.0 {
                let mut grid = cx.res.get::<pathfinding::grid::Grid>().unwrap().clone();

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

                        grid.remove_vertex(&(next_coord.0 as usize, next_coord.1 as usize));
                    } else {
                        if coords.hops.is_empty() {
                            continue;
                        }
                        let goal = Pos(
                            coords.hops[coords.hops.len() - 1].0,
                            coords.hops[coords.hops.len() - 1].1,
                        );

                        grid.add_vertex((coords.xcoord as usize, coords.ycoord as usize));
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
                        }
                    }
                } else {
                    let mut rng = rand::thread_rng();

                    let mut xcoord = rng.gen_range(0..params.tiles_dimension.0);
                    let mut ycoord = rng.gen_range(0..params.tiles_dimension.1);

                    while (!grid.has_vertex(&(xcoord as usize, ycoord as usize))) {
                        xcoord = rng.gen_range(0..params.tiles_dimension.0);
                        ycoord = rng.gen_range(0..params.tiles_dimension.1);
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
                    }
                    // for _p in &coords.hops {
                    //     println!("Path: {}, {}", _p.0, _p.1);
                    // }
                }

                cx.res.insert(grid);
            }
            // println!("Position {}", v);
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
        for i in 0..24 {
            cx.res.with(BunnyCount::default).count += 1;
            Bunny.spawn(cx.task());
        }
        // Bunny.spawn(cx.task());
        // Bunny.spawn(cx.task());
        // Bunny.spawn(cx.task());
        // Bunny.spawn(cx.task());
        // Bunny.spawn(cx.task());
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
