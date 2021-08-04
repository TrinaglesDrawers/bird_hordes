use std::{
    collections::{HashMap, HashSet, VecDeque},
    sync::Arc,
};

use arcana::{graphics::Mesh, hecs::World, na, TaskContext};
use bitsetium::*;
use goods::{Asset, AssetField};
use rand::{Rng, SeedableRng};
use rand_xoshiro::Xoshiro128PlusPlus;

#[derive(Clone, Debug, AssetField)]
pub struct Tile {
    #[external]
    mesh: Mesh,

    north_neighbours: Bits,
    south_neighbours: Bits,
    west_neighbours: Bits,
    east_neighbours: Bits,
    up_neighbours: Bits,
    down_neighbours: Bits,
}

#[derive(Clone, Debug, Asset)]
pub struct TileSet {
    #[container]
    tiles: Arc<[Tile]>,
}

type Bits = u128;
const COLUMN_HEIGHT: usize = 4;

#[derive(Clone)]
enum Column {
    Superposition { bits: [Bits; COLUMN_HEIGHT] },
    Collapsed { tiles: [usize; COLUMN_HEIGHT] },
}

impl Column {
    fn north_neighbours(&self, floor: usize, tile_set: &TileSet) -> Bits {
        match self {
            Column::Superposition { bits } => iter_bits(&bits[floor])
                .fold(Bits::empty(), |acc, bit| {
                    acc.intersection(tile_set.tiles[bit].north_neighbours)
                }),
            Column::Collapsed { tiles } => tile_set.tiles[tiles[floor]].north_neighbours,
        }
    }

    fn south_neighbours(&self, floor: usize, tile_set: &TileSet) -> Bits {
        match self {
            Column::Superposition { bits } => iter_bits(&bits[floor])
                .fold(Bits::empty(), |acc, bit| {
                    acc.intersection(tile_set.tiles[bit].south_neighbours)
                }),
            Column::Collapsed { tiles } => tile_set.tiles[tiles[floor]].south_neighbours,
        }
    }

    fn west_neighbours(&self, floor: usize, tile_set: &TileSet) -> Bits {
        match self {
            Column::Superposition { bits } => iter_bits(&bits[floor])
                .fold(Bits::empty(), |acc, bit| {
                    acc.intersection(tile_set.tiles[bit].west_neighbours)
                }),
            Column::Collapsed { tiles } => tile_set.tiles[tiles[floor]].west_neighbours,
        }
    }

    fn east_neighbours(&self, floor: usize, tile_set: &TileSet) -> Bits {
        match self {
            Column::Superposition { bits } => iter_bits(&bits[floor])
                .fold(Bits::empty(), |acc, bit| {
                    acc.intersection(tile_set.tiles[bit].east_neighbours)
                }),
            Column::Collapsed { tiles } => tile_set.tiles[tiles[floor]].east_neighbours,
        }
    }

    fn up_neighbours(&self, floor: usize, tile_set: &TileSet) -> Bits {
        match self {
            Column::Superposition { bits } => iter_bits(&bits[floor])
                .fold(Bits::empty(), |acc, bit| {
                    acc.intersection(tile_set.tiles[bit].up_neighbours)
                }),
            Column::Collapsed { tiles } => tile_set.tiles[tiles[floor]].up_neighbours,
        }
    }

    fn down_neighbours(&self, floor: usize, tile_set: &TileSet) -> Bits {
        match self {
            Column::Superposition { bits } => iter_bits(&bits[floor])
                .fold(Bits::empty(), |acc, bit| {
                    acc.union(tile_set.tiles[bit].down_neighbours)
                }),
            Column::Collapsed { tiles } => tile_set.tiles[tiles[floor]].down_neighbours,
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

pub struct Terrain {
    chunks: Chunks,
    rng: Xoshiro128PlusPlus,
    tile_set: TileSet,
    problem_tile: usize,
    tile_extent: na::Vector3<f32>,
    any_tile_bits: Bits,
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

        assert!(tile_set.tiles.len() <= Bits::MAX_SET_INDEX);

        let any_tile_bits = 1u128
            .wrapping_shl(tile_set.tiles.len() as u32)
            .wrapping_sub(1);

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
        }
    }

    pub fn spawn_around(&mut self, point: na::Point3<f32>, radius: f32, cx: TaskContext) {
        // Find which columns to spawn
        let west = ((point.x - radius) / (self.tile_extent.x)).floor() as isize;
        let east = ((point.x + radius) / (self.tile_extent.x)).ceil() as isize;
        let north = ((point.z - radius) / (self.tile_extent.z)).floor() as isize;
        let south = ((point.z + radius) / (self.tile_extent.z)).ceil() as isize;

        let r2 = radius * radius;

        let mut cols = Vec::new();

        for x in east..=west {
            for y in south..=north {
                let d2 = na::Vector2::new(
                    (x * self.chunks.extent.x) as f32 * self.tile_extent.x - point.x,
                    (y * self.chunks.extent.y) as f32 * self.tile_extent.z - point.z,
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

        for col in &cols {
            self.collapse_column(*col);
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

                        queue.push_back((na::Vector2::new(col.x, col.y - 1), floor));
                        queue.push_back((na::Vector2::new(col.x, col.y + 1), floor));
                        queue.push_back((na::Vector2::new(col.x + 1, col.y), floor));
                        queue.push_back((na::Vector2::new(col.x - 1, col.y), floor));

                        self.propagate(&mut queue);
                    }
                },
            }
        }

        let column = self.chunks.get_column(col, &self.any_tile_bits);
        match column {
            Column::Collapsed { .. } => unreachable!(),
            Column::Superposition { bits } => {
                let problem_tile = self.problem_tile;

                let mut tiles = [0; COLUMN_HEIGHT];
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
    }

    fn propagate(&mut self, queue: &mut VecDeque<(na::Vector2<isize>, usize)>) {
        while let Some((col, floor)) = queue.pop_front() {
            match *self.chunks.get_column(col, &self.any_tile_bits) {
                Column::Collapsed { .. } => {}
                Column::Superposition { .. } => {
                    let west = self
                        .chunks
                        .get_column(na::Vector2::new(col.x - 1, col.y), &self.any_tile_bits)
                        .east_neighbours(floor, &self.tile_set);
                    let east = self
                        .chunks
                        .get_column(na::Vector2::new(col.x + 1, col.y), &self.any_tile_bits)
                        .west_neighbours(floor, &self.tile_set);

                    let north = self
                        .chunks
                        .get_column(na::Vector2::new(col.x, col.y - 1), &self.any_tile_bits)
                        .south_neighbours(floor, &self.tile_set);
                    let south = self
                        .chunks
                        .get_column(na::Vector2::new(col.x, col.y + 1), &self.any_tile_bits)
                        .north_neighbours(floor, &self.tile_set);

                    let mut possible = west
                        .intersection(east)
                        .intersection(north)
                        .intersection(south);

                    let column = self.chunks.get_column(col, &self.any_tile_bits);

                    if floor < COLUMN_HEIGHT - 1 {
                        let up = column.down_neighbours(floor + 1, &self.tile_set);
                        possible = possible.intersection(up);
                    }

                    if floor > 0 {
                        let down = column.up_neighbours(floor - 1, &self.tile_set);
                        possible = possible.intersection(down);
                    }

                    match column {
                        Column::Collapsed { .. } => unreachable!(),
                        Column::Superposition { bits } => {
                            if bits[floor] != possible {
                                bits[floor] = possible;

                                if floor < COLUMN_HEIGHT - 1 {
                                    queue.push_back((col, floor + 1));
                                }

                                if floor > 0 {
                                    queue.push_back((col, floor - 1));
                                }

                                queue.push_back((na::Vector2::new(col.x, col.y - 1), floor));
                                queue.push_back((na::Vector2::new(col.x, col.y + 1), floor));
                                queue.push_back((na::Vector2::new(col.x + 1, col.y), floor));
                                queue.push_back((na::Vector2::new(col.x - 1, col.y), floor));
                            }
                        }
                    }
                }
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
