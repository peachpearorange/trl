use noise::{Fbm, NoiseFn, Perlin};

use crate::level::{
  fill_rect, Level, Tile, ZoneWorld, WORLD_COLS, WORLD_DEPTH, WORLD_ROWS, ZONE_HEIGHT, ZONE_WIDTH,
};

pub const WORLD_SEED: u64 = 42;

/// Generate a full ZoneWorld from a deterministic seed.
pub fn generate_world(seed: u64) -> ZoneWorld {
  ZoneWorld::new(Tile::Air)
}
