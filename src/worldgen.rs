use noise::{Fbm, NoiseFn, Perlin};

use crate::level::{
  fill_rect, Level, Tile, ZoneWorld, WORLD_COLS, WORLD_DEPTH, WORLD_ROWS, ZONE_HEIGHT, ZONE_WIDTH,
};

pub const WORLD_SEED: u64 = 42;

/// Returns a [0, 1] weight: 1.0 at the world center, ~0.0 at the edges.
/// Used to multiply noise values so the world forms an island surrounded by ocean.
pub fn island_mask(wx: usize, wy: usize) -> f64 {
  let cx = (WORLD_COLS * ZONE_WIDTH) as f64 / 2.0;
  let cy = (WORLD_ROWS * ZONE_HEIGHT) as f64 / 2.0;
  // Normalise to [-1, 1] range; corners land at ≈ 1.41
  let dx = (wx as f64 - cx) / cx;
  let dy = (wy as f64 - cy) / cy;
  // Clamp to 1.0 so corners don't go above 1 after sqrt
  let d = (dx * dx + dy * dy).sqrt().min(1.0);
  // Smooth quadratic falloff: 1 at center, 0 at edge
  (1.0 - d).max(0.0).powi(2)
}

/// Map a masked noise value in [0, 1] to a surface Tile.
/// Lower values are ocean; higher values are inland terrain.
pub fn tile_from_value(v: f64) -> Tile {
  match v {
    v if v < 0.12 => Tile::DeepWater,
    v if v < 0.20 => Tile::ShallowWater,
    v if v < 0.26 => Tile::Sand,
    v if v < 0.58 => Tile::Grass,
    v if v < 0.66 => Tile::TallGrass,
    v if v < 0.73 => Tile::Bush,
    v if v < 0.83 => Tile::Ash,
    _             => Tile::Lava,
  }
}

/// Generate a full ZoneWorld from a deterministic seed.
pub fn generate_world(_seed: u64) -> ZoneWorld {
  ZoneWorld::new(Tile::Air)
}

#[cfg(test)]
mod tests {
  use super::*;

  // --- island_mask ---

  #[test]
  fn island_mask_center_is_one() {
    let cx = WORLD_COLS * ZONE_WIDTH / 2;
    let cy = WORLD_ROWS * ZONE_HEIGHT / 2;
    let m = island_mask(cx, cy);
    assert!(m > 0.95, "center mask should be near 1.0, got {m}");
  }

  #[test]
  fn island_mask_corner_is_near_zero() {
    let m = island_mask(0, 0);
    assert!(m < 0.05, "corner mask should be near 0.0, got {m}");
  }

  #[test]
  fn island_mask_monotone_along_x() {
    let cy = WORLD_ROWS * ZONE_HEIGHT / 2;
    let cx = WORLD_COLS * ZONE_WIDTH / 2;
    // mask should be larger closer to center on horizontal axis
    assert!(island_mask(cx, cy) > island_mask(cx / 2, cy));
    assert!(island_mask(cx / 2, cy) > island_mask(0, cy));
  }

  // --- tile_from_value ---

  #[test]
  fn tile_from_zero_is_deep_water() {
    assert_eq!(tile_from_value(0.0), Tile::DeepWater);
  }

  #[test]
  fn tile_from_mid_is_grass() {
    assert_eq!(tile_from_value(0.45), Tile::Grass);
  }

  #[test]
  fn tile_from_high_is_lava() {
    assert_eq!(tile_from_value(1.0), Tile::Lava);
  }

  #[test]
  fn tile_from_sand_range() {
    let t = tile_from_value(0.24);
    assert_eq!(t, Tile::Sand);
  }
}
