use std::{
    collections::{HashMap, HashSet, VecDeque},
    convert::TryFrom,
    sync::Arc,
};

use arcana::{graphics::Mesh, hecs::World, na, Global3, TaskContext};
use bitsetium::*;
use goods::{Asset, AssetField};
use rand::{seq::SliceRandom, Rng, SeedableRng};
use rand_xoshiro::Xoshiro128PlusPlus;

#[derive(Clone, Copy, Debug, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Side {
    North = 0,
    East = 1,
    South = 2,
    West = 3,
    Up = 4,
    Down = 5,
}

#[derive(Clone, Copy, Debug)]
pub struct SideOutOfBounds;

impl TryFrom<usize> for Side {
    type Error = SideOutOfBounds;
    fn try_from(value: usize) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Side::North),
            1 => Ok(Side::East),
            2 => Ok(Side::South),
            3 => Ok(Side::West),
            4 => Ok(Side::Up),
            5 => Ok(Side::Down),
            _ => Err(SideOutOfBounds),
        }
    }
}

#[derive(Clone, Copy, Debug, serde::Deserialize)]
pub struct Neighbour {
    pub id: usize,
    pub side: Side,
}

#[derive(Clone, Debug, AssetField)]
pub struct Tile {
    #[external]
    pub mesh: Option<Mesh>,
}

#[derive(Clone, Debug, Asset)]
pub struct TileSet {
    #[container]
    pub tiles: Arc<[Tile]>,

    pub neighbours: Arc<[[Neighbour; 2]]>,
}

type Bits = u128;
const COLUMN_HEIGHT: usize = 16;

#[derive(Clone)]
enum Column {
    Superposition { bits: [Bits; COLUMN_HEIGHT] },
    Collapsed { tiles: [usize; COLUMN_HEIGHT] },
}

impl Column {
    fn north_neighbours(
        &self,
        floor: usize,
        neighbours: &[NeighboursBits],
        memoized: &mut Memoized,
    ) -> Bits {
        match self {
            Column::Superposition { bits } => {
                *memoized.north.entry(bits[floor]).or_insert_with(|| {
                    iter_bits(&bits[floor])
                        .fold(Bits::empty(), |acc, bit| acc.union(neighbours[bit].north))
                })
            }
            Column::Collapsed { tiles } => neighbours[tiles[floor]].north,
        }
    }

    fn south_neighbours(
        &self,
        floor: usize,
        neighbours: &[NeighboursBits],
        memoized: &mut Memoized,
    ) -> Bits {
        match self {
            Column::Superposition { bits } => {
                *memoized.south.entry(bits[floor]).or_insert_with(|| {
                    iter_bits(&bits[floor])
                        .fold(Bits::empty(), |acc, bit| acc.union(neighbours[bit].south))
                })
            }
            Column::Collapsed { tiles } => neighbours[tiles[floor]].south,
        }
    }

    fn west_neighbours(
        &self,
        floor: usize,
        neighbours: &[NeighboursBits],
        memoized: &mut Memoized,
    ) -> Bits {
        match self {
            Column::Superposition { bits } => {
                *memoized.west.entry(bits[floor]).or_insert_with(|| {
                    iter_bits(&bits[floor])
                        .fold(Bits::empty(), |acc, bit| acc.union(neighbours[bit].west))
                })
            }
            Column::Collapsed { tiles } => neighbours[tiles[floor]].west,
        }
    }

    fn east_neighbours(
        &self,
        floor: usize,
        neighbours: &[NeighboursBits],
        memoized: &mut Memoized,
    ) -> Bits {
        match self {
            Column::Superposition { bits } => {
                *memoized.east.entry(bits[floor]).or_insert_with(|| {
                    iter_bits(&bits[floor])
                        .fold(Bits::empty(), |acc, bit| acc.union(neighbours[bit].east))
                })
            }
            Column::Collapsed { tiles } => neighbours[tiles[floor]].east,
        }
    }

    fn up_neighbours(
        &self,
        floor: usize,
        neighbours: &[NeighboursBits],
        memoized: &mut Memoized,
    ) -> Bits {
        match self {
            Column::Superposition { bits } => {
                *memoized.up.entry(bits[floor]).or_insert_with(|| {
                    iter_bits(&bits[floor])
                        .fold(Bits::empty(), |acc, bit| acc.union(neighbours[bit].up))
                })
            }
            Column::Collapsed { tiles } => neighbours[tiles[floor]].up,
        }
    }

    fn down_neighbours(
        &self,
        floor: usize,
        neighbours: &[NeighboursBits],
        memoized: &mut Memoized,
    ) -> Bits {
        match self {
            Column::Superposition { bits } => {
                *memoized.down.entry(bits[floor]).or_insert_with(|| {
                    iter_bits(&bits[floor])
                        .fold(Bits::empty(), |acc, bit| acc.union(neighbours[bit].down))
                })
            }
            Column::Collapsed { tiles } => neighbours[tiles[floor]].down,
        }
    }
}

struct Chunk {
    columns: Vec<Column>,
}

impl Chunk {
    fn column(&self, chunk_extent: na::Vector2<isize>, column: na::Vector2<isize>) -> &Column {
        let x = column.x.rem_euclid(chunk_extent.x);
        let y = column.y.rem_euclid(chunk_extent.y);

        let idx = x + y * chunk_extent.x;
        &self.columns[idx as usize]
    }

    fn column_mut(
        &mut self,
        chunk_extent: na::Vector2<isize>,
        column: na::Vector2<isize>,
    ) -> &mut Column {
        let x = column.x.rem_euclid(chunk_extent.x);
        let y = column.y.rem_euclid(chunk_extent.y);

        let idx = x + y * chunk_extent.x;
        &mut self.columns[idx as usize]
    }
}

struct Chunks {
    extent: na::Vector2<isize>,
    map: HashMap<na::Vector2<isize>, Chunk>,
}

impl Chunks {
    fn get_column(&mut self, col: na::Vector2<isize>, any_tile_bits: &Bits) -> &mut Column {
        let chunk = na::Vector2::new(
            col.x.div_euclid(self.extent.x),
            col.y.div_euclid(self.extent.y),
        );
        let chunk_extent = self.extent;
        let chunk = self.map.entry(chunk).or_insert_with(|| Chunk {
            columns: vec![
                Column::Superposition {
                    bits: [*any_tile_bits; COLUMN_HEIGHT]
                };
                (chunk_extent.x * chunk_extent.y) as usize
            ],
        });

        chunk.column_mut(self.extent, col)
    }
}

struct NeighboursBits {
    north: Bits,
    south: Bits,
    west: Bits,
    east: Bits,
    up: Bits,
    down: Bits,
}

struct Memoized {
    north: HashMap<Bits, Bits>,
    south: HashMap<Bits, Bits>,
    west: HashMap<Bits, Bits>,
    east: HashMap<Bits, Bits>,
    up: HashMap<Bits, Bits>,
    down: HashMap<Bits, Bits>,
}

pub struct Terrain {
    chunks: Chunks,
    rng: Xoshiro128PlusPlus,
    tile_set: TileSet,
    problem_tile: usize,
    tile_extent: na::Vector3<f32>,

    any_tile_bits: Bits,

    neighbours: Vec<NeighboursBits>,
    memoized: Memoized,
}

impl Terrain {
    pub fn new(
        tile_set: TileSet,
        problem_tile: usize,
        tile_extent: na::Vector3<f32>,
        chunk_extent: na::Vector2<isize>,
        seed: u64,
    ) -> Self {
        assert!(chunk_extent.x > 0);
        assert!(chunk_extent.y > 0);
        assert!(chunk_extent.x.checked_mul(chunk_extent.y).is_some());

        assert!(tile_set.tiles.len() <= Bits::MAX_SET_INDEX / 4);

        let any_tile_bits = 1u128
            .wrapping_shl(tile_set.tiles.len() as u32 * 4)
            .wrapping_sub(1);

        let len = tile_set.tiles.len();

        let mut neighbours = (0..len * 4)
            .map(|_| NeighboursBits {
                north: Bits::empty(),
                south: Bits::empty(),
                west: Bits::empty(),
                east: Bits::empty(),
                up: Bits::empty(),
                down: Bits::empty(),
            })
            .collect::<Vec<_>>();

        let rot_id = |id: usize, rot: usize| rot + id * 4;

        for [left, right] in &*tile_set.neighbours {
            match (left.side, right.side) {
                (Side::Up, Side::Down) => {
                    for i in 0..4 {
                        for j in 0..4 {
                            neighbours[rot_id(left.id, i)].up.set(rot_id(right.id, j));
                            neighbours[rot_id(right.id, i)].down.set(rot_id(left.id, j));
                        }
                    }
                }
                (Side::Down, Side::Up) => {
                    for i in 0..4 {
                        for j in 0..4 {
                            neighbours[rot_id(right.id, i)].up.set(rot_id(left.id, j));
                            neighbours[rot_id(left.id, i)].down.set(rot_id(right.id, j));
                        }
                    }
                }
                (Side::Up | Side::Down, _) | (_, Side::Up | Side::Down) => {
                    panic!("Invalid adjustment data")
                }
                _ => {
                    neighbours[rot_id(left.id, (4 + 0 - left.side as usize) % 4)]
                        .north
                        .set(rot_id(right.id, (4 + 2 - right.side as usize) % 4));

                    neighbours[rot_id(left.id, (4 + 1 - left.side as usize) % 4)]
                        .east
                        .set(rot_id(right.id, (4 + 3 - right.side as usize) % 4));

                    neighbours[rot_id(left.id, (4 + 2 - left.side as usize) % 4)]
                        .south
                        .set(rot_id(right.id, (4 + 0 - right.side as usize) % 4));

                    neighbours[rot_id(left.id, (4 + 3 - left.side as usize) % 4)]
                        .west
                        .set(rot_id(right.id, (4 + 1 - right.side as usize) % 4));

                    //
                    //

                    neighbours[rot_id(right.id, (4 + 0 - right.side as usize) % 4)]
                        .north
                        .set(rot_id(left.id, (4 + 2 - left.side as usize) % 4));
                    neighbours[rot_id(right.id, (4 + 1 - right.side as usize) % 4)]
                        .east
                        .set(rot_id(left.id, (4 + 3 - left.side as usize) % 4));
                    neighbours[rot_id(right.id, (4 + 2 - right.side as usize) % 4)]
                        .south
                        .set(rot_id(left.id, (4 + 0 - left.side as usize) % 4));
                    neighbours[rot_id(right.id, (4 + 3 - right.side as usize) % 4)]
                        .west
                        .set(rot_id(left.id, (4 + 1 - left.side as usize) % 4));
                }
            }
        }

        // for (idx, neighbours) in neighbours.iter().enumerate() {
        //     tracing::error!(
        //         "{}.{:?} ↑ {:0x} : {}",
        //         idx / 4,
        //         idx % 4,
        //         neighbours.north,
        //         neighbours.north.count_ones(),
        //     );
        //     tracing::error!(
        //         "{}.{:?} → {:0x} : {}",
        //         idx / 4,
        //         idx % 4,
        //         neighbours.east,
        //         neighbours.east.count_ones(),
        //     );
        //     tracing::error!(
        //         "{}.{:?} ↓ {:0x} : {}",
        //         idx / 4,
        //         idx % 4,
        //         neighbours.south,
        //         neighbours.south.count_ones(),
        //     );
        //     tracing::error!(
        //         "{}.{:?} ← {:0x} : {}",
        //         idx / 4,
        //         idx % 4,
        //         neighbours.west,
        //         neighbours.west.count_ones(),
        //     );
        // }

        let is_any = iter_bits(&any_tile_bits)
            .fold(Bits::empty(), |acc, bit| acc.union(neighbours[bit].north));
        assert_eq!(any_tile_bits, is_any);

        let is_any = iter_bits(&any_tile_bits)
            .fold(Bits::empty(), |acc, bit| acc.union(neighbours[bit].east));
        assert_eq!(any_tile_bits, is_any);

        let is_any = iter_bits(&any_tile_bits)
            .fold(Bits::empty(), |acc, bit| acc.union(neighbours[bit].south));
        assert_eq!(any_tile_bits, is_any);

        let is_any = iter_bits(&any_tile_bits)
            .fold(Bits::empty(), |acc, bit| acc.union(neighbours[bit].west));
        assert_eq!(any_tile_bits, is_any);

        Terrain {
            chunks: Chunks {
                extent: chunk_extent,
                map: HashMap::new(),
            },
            rng: Xoshiro128PlusPlus::seed_from_u64(seed),
            tile_extent,
            tile_set,
            problem_tile,
            any_tile_bits,

            neighbours,

            memoized: Memoized {
                north: HashMap::new(),
                south: HashMap::new(),
                west: HashMap::new(),
                east: HashMap::new(),
                up: HashMap::new(),
                down: HashMap::new(),
            },
        }
    }

    pub fn spawn_around(&mut self, point: na::Point3<f32>, radius: f32, mut cx: TaskContext<'_>) {
        // Find which columns to spawn
        let west = ((point.x - radius) / (self.tile_extent.x)).floor() as isize;
        let east = ((point.x + radius) / (self.tile_extent.x)).ceil() as isize;
        let north = ((point.z + radius) / (self.tile_extent.z)).ceil() as isize;
        let south = ((point.z - radius) / (self.tile_extent.z)).floor() as isize;

        let r2 = radius * radius;

        let mut cols = Vec::new();

        for x in west..=east {
            for y in south..=north {
                let d2 = na::Vector2::new(
                    x as f32 * self.tile_extent.x - point.x,
                    y as f32 * self.tile_extent.z - point.z,
                )
                .magnitude_squared();
                if d2 <= r2 {
                    let col = na::Vector2::new(x, y);
                    if self.column_order(col).is_some() {
                        cols.push(col);
                    }
                }
            }
        }

        // Sort columns before collapsing
        cols.sort_by_key(|&col| self.column_order(col).unwrap());
        cols.dedup();
        let cols = cols;

        for col in &cols {
            self.collapse_column(*col);
        }

        // tracing::error!("Spawning {} columns", cols.len());
        for col in &cols {
            // tracing::error!("Spawning {}", col);
            self.spawn_column(*col, cx.reborrow());
        }
    }

    fn column_order(&mut self, col: na::Vector2<isize>) -> Option<usize> {
        match self.chunks.get_column(col, &self.any_tile_bits) {
            Column::Superposition { bits } => {
                Some(bits.iter().map(|p| p.count_ones() as usize).sum())
            }
            Column::Collapsed { .. } => None,
        }
    }

    fn collapse_column(&mut self, col: na::Vector2<isize>) {
        let mut queue = VecDeque::new();

        for floor in 0..COLUMN_HEIGHT {
            match self.chunks.get_column(col, &self.any_tile_bits) {
                Column::Collapsed { .. } => unreachable!(),
                Column::Superposition { bits } => match iter_bits(&bits[floor]).count() {
                    0 | 1 => {}
                    count => {
                        let nth = self.rng.gen_range(0..count);

                        let bit = iter_bits(&bits[floor]).nth(nth).unwrap();

                        bits[floor] = Bits::empty();
                        bits[floor].set(bit);

                        if floor < COLUMN_HEIGHT - 1 {
                            queue.push_back((col, floor + 1));
                        }

                        // if floor > 0 {
                        //     queue.push_back((col, floor - 1));
                        // }

                        queue.push_back((na::Vector2::new(col.x, col.y + 1), floor));
                        queue.push_back((na::Vector2::new(col.x + 1, col.y), floor));
                        queue.push_back((na::Vector2::new(col.x, col.y - 1), floor));
                        queue.push_back((na::Vector2::new(col.x - 1, col.y), floor));

                        self.propagate(&mut queue);
                    }
                },
            }
        }
    }

    fn propagate(&mut self, queue: &mut VecDeque<(na::Vector2<isize>, usize)>) {
        let mut countdown = 1000;
        loop {
            countdown -= 1;
            if countdown == 0 {
                return;
            }
            match queue.pop_front() {
                None => break,
                Some((col, floor)) => match *self.chunks.get_column(col, &self.any_tile_bits) {
                    Column::Collapsed { .. } => {}
                    Column::Superposition { .. } => {
                        let north = self
                            .chunks
                            .get_column(na::Vector2::new(col.x, col.y + 1), &self.any_tile_bits)
                            .south_neighbours(floor, &self.neighbours, &mut self.memoized);
                        let east = self
                            .chunks
                            .get_column(na::Vector2::new(col.x + 1, col.y), &self.any_tile_bits)
                            .west_neighbours(floor, &self.neighbours, &mut self.memoized);
                        let south = self
                            .chunks
                            .get_column(na::Vector2::new(col.x, col.y - 1), &self.any_tile_bits)
                            .north_neighbours(floor, &self.neighbours, &mut self.memoized);
                        let west = self
                            .chunks
                            .get_column(na::Vector2::new(col.x - 1, col.y), &self.any_tile_bits)
                            .east_neighbours(floor, &self.neighbours, &mut self.memoized);

                        let mut possible = west
                            .intersection(east)
                            .intersection(north)
                            .intersection(south);

                        let column = self.chunks.get_column(col, &self.any_tile_bits);

                        if floor < COLUMN_HEIGHT - 1 {
                            let up = column.down_neighbours(
                                floor + 1,
                                &self.neighbours,
                                &mut self.memoized,
                            );
                            possible = possible.intersection(up);
                        }

                        if floor > 0 {
                            let down = column.up_neighbours(
                                floor - 1,
                                &self.neighbours,
                                &mut self.memoized,
                            );
                            possible = possible.intersection(down);
                        }

                        if possible.test_none() {
                            tracing::error!("Problem");
                            possible.set(self.problem_tile);
                        }

                        match column {
                            Column::Collapsed { .. } => unreachable!(),
                            Column::Superposition { bits } => {
                                if bits[floor] != possible {
                                    // tracing::error!(
                                    //     "Reduction from {:0x} => {:0x}",
                                    //     bits[floor],
                                    //     possible
                                    // );

                                    bits[floor] = possible;

                                    if floor < COLUMN_HEIGHT - 1 {
                                        queue.push_back((col, floor + 1));
                                    }

                                    if floor > 0 {
                                        queue.push_back((col, floor - 1));
                                    }

                                    queue.push_back((na::Vector2::new(col.x, col.y + 1), floor));
                                    queue.push_back((na::Vector2::new(col.x + 1, col.y), floor));
                                    queue.push_back((na::Vector2::new(col.x, col.y - 1), floor));
                                    queue.push_back((na::Vector2::new(col.x - 1, col.y), floor));
                                }
                            }
                        }
                    }
                },
            }
        }
    }

    fn spawn_column(&mut self, col: na::Vector2<isize>, tx: TaskContext<'_>) {
        let mut tiles = [0; COLUMN_HEIGHT];

        let column = self.chunks.get_column(col, &self.any_tile_bits);
        match column {
            Column::Collapsed { .. } => unreachable!(),
            Column::Superposition { bits } => {
                let problem_tile = self.problem_tile;

                for floor in 0..COLUMN_HEIGHT {
                    let bit = iter_bits(&bits[floor]).next();
                    tiles[floor] = match bit {
                        None => problem_tile,
                        Some(tile) => tile,
                    };
                }

                *column = Column::Collapsed { tiles };
            }
        }

        for (floor, &tile) in tiles.iter().enumerate() {
            let id = tile / 4;
            let rot = tile % 4;

            if let Some(mesh) = &self.tile_set.tiles[id].mesh {
                let entity = tx.world.spawn((
                    Global3::new(
                        na::Translation3::new(
                            col.x as f32 * self.tile_extent.x,
                            floor as f32 * self.tile_extent.y,
                            col.y as f32 * self.tile_extent.z,
                        ) * na::UnitQuaternion::from_axis_angle(
                            &na::Unit::new_normalize(na::Vector3::y()),
                            rot as f32 * std::f32::consts::FRAC_PI_2,
                        ),
                    ),
                    mesh.clone(),
                ));
            }
        }
    }
}

fn iter_bits<T: BitSearch>(bits: &T) -> impl Iterator<Item = usize> + '_ {
    let mut next = Some(0);
    std::iter::from_fn(move || {
        let bit = bits.find_first_set(next?)?;
        if bit == usize::MAX {
            next = None;
        } else {
            next = Some(next? + 1);
        }
        Some(bit)
    })
}
