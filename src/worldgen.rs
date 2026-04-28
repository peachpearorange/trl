use std::collections::HashSet;

use noise::{Fbm, NoiseFn, Perlin};
use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;

use crate::level::{
  Tile, ZoneWorld, WORLD_COLS, WORLD_ROWS, SURFACE_Z, ZONE_HEIGHT, ZONE_WIDTH,
};

pub const WORLD_SEED: u64 = 42;

/// Returns a [0, 1] weight: 1.0 at the world center, ~0.0 at the edges.
pub fn island_mask(wx: usize, wy: usize) -> f64 {
  let cx = (WORLD_COLS * ZONE_WIDTH) as f64 / 2.0;
  let cy = (WORLD_ROWS * ZONE_HEIGHT) as f64 / 2.0;
  let dx = (wx as f64 - cx) / cx;
  let dy = (wy as f64 - cy) / cy;
  // Corners produce d ≈ 1.41; clamp so (1.0 - d) doesn't go negative before .max(0.0)
  let d = (dx * dx + dy * dy).sqrt().min(1.0);
  (1.0 - d).max(0.0).powi(2)
}

/// Map a masked noise value in [0, 1] to a surface Tile.
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

fn surface_tile_value(wx: usize, wy: usize, noise: &Fbm<Perlin>) -> f64 {
  const SCALE: f64 = 110.0;
  let raw = noise.get([wx as f64 / SCALE, wy as f64 / SCALE]);
  let normalized = (raw + 1.0) / 2.0;
  normalized * island_mask(wx, wy)
}

fn underground_tile(wx: usize, wy: usize, z: usize, noise: &Fbm<Perlin>) -> Tile {
  const SCALE: f64 = 16.0;
  let z_offset = z as f64 * 137.3;
  let v = noise.get([wx as f64 / SCALE, wy as f64 / SCALE + z_offset]);
  if v > -0.1 { Tile::CaveFloor } else { Tile::CaveWall }
}

/// Write a tile at world coordinates (wx, wy, z). Ignores out-of-bounds.
fn set_world_tile(world: &mut ZoneWorld, wx: i32, wy: i32, z: usize, tile: Tile) {
  if wx < 0 || wy < 0 { return; }
  let (wx, wy) = (wx as usize, wy as usize);
  let zx = wx / ZONE_WIDTH;
  let zy = wy / ZONE_HEIGHT;
  if zx >= WORLD_COLS || zy >= WORLD_ROWS { return; }
  world.zone_mut(zx, zy, z).tiles[wy % ZONE_HEIGHT][wx % ZONE_WIDTH] = tile;
}

fn get_world_tile(world: &ZoneWorld, wx: usize, wy: usize, z: usize) -> Option<Tile> {
  let zx = wx / ZONE_WIDTH;
  let zy = wy / ZONE_HEIGHT;
  if zx >= WORLD_COLS || zy >= WORLD_ROWS {
    return None;
  }
  Some(world.zone(zx, zy, z).tiles[wy % ZONE_HEIGHT][wx % ZONE_WIDTH])
}

fn land_tile_for_rock(t: Tile) -> bool {
  matches!(
    t,
    Tile::Grass | Tile::TallGrass | Tile::Sand | Tile::Bush | Tile::Ash
  )
}

/// Natural rock walls on land, with worm-carved gaps so massifs stay navigable.
fn place_rock_massifs(world: &mut ZoneWorld, seed: u64) {
  let rock_n: Fbm<Perlin> = Fbm::new(seed.wrapping_add(999) as u32);
  let passage_n: Fbm<Perlin> = Fbm::new(seed.wrapping_add(1001) as u32);
  let worm_n: Fbm<Perlin> = Fbm::new(seed.wrapping_add(1002) as u32);
  let w = WORLD_COLS * ZONE_WIDTH;
  let h = WORLD_ROWS * ZONE_HEIGHT;

  for wy in 0..h {
    for wx in 0..w {
      let Some(tile) = get_world_tile(world, wx, wy, SURFACE_Z) else { continue };
      if !land_tile_for_rock(tile) {
        continue;
      }
      if island_mask(wx, wy) < 0.1 {
        continue;
      }
      let r = (rock_n.get([wx as f64 / 36.0, wy as f64 / 36.0]) + 1.0) * 0.5;
      if r < 0.54 {
        continue;
      }
      let p = (passage_n.get([wx as f64 / 13.0, wy as f64 / 13.0]) + 1.0) * 0.5;
      if p < 0.17 {
        continue;
      }
      set_world_tile(world, wx as i32, wy as i32, SURFACE_Z, Tile::Wall);
    }
  }

  for wy in 0..h {
    for wx in 0..w {
      if get_world_tile(world, wx, wy, SURFACE_Z) != Some(Tile::Wall) {
        continue;
      }
      let v = (worm_n.get([wx as f64 / 9.5, wy as f64 / 9.5]) + 1.0) * 0.5;
      if v < 0.10 {
        set_world_tile(world, wx as i32, wy as i32, SURFACE_Z, Tile::Grass);
      }
    }
  }
}

fn tile_ok_for_tree(t: Tile) -> bool {
  matches!(
    t,
    Tile::Grass | Tile::TallGrass | Tile::Sand | Tile::Bush | Tile::Ash
  )
}

/// Forest clusters and scattered trees on the final surface (after towns/stairs).
fn collect_tree_sites(world: &ZoneWorld, seed: u64) -> Vec<(i32, i32)> {
  let forest_n: Fbm<Perlin> = Fbm::new(seed.wrapping_add(501) as u32);
  let sparse_n: Fbm<Perlin> = Fbm::new(seed.wrapping_add(502) as u32);
  let w = WORLD_COLS * ZONE_WIDTH;
  let h = WORLD_ROWS * ZONE_HEIGHT;
  let mut out = Vec::new();
  let mut seen = HashSet::new();

  for wy in 0..h {
    for wx in 0..w {
      let Some(t) = get_world_tile(world, wx, wy, SURFACE_Z) else { continue };
      if !tile_ok_for_tree(t) {
        continue;
      }
      let fv = (forest_n.get([wx as f64 / 26.0, wy as f64 / 26.0]) + 1.0) * 0.5;
      let sv = (sparse_n.get([wx as f64 / 3.5, wy as f64 / 3.5]) + 1.0) * 0.5;
      let hash = wx.wrapping_mul(374_761_393).wrapping_add(wy.wrapping_mul(668_265_263));
      let dense = fv > 0.66 && sv > 0.28 && hash % 5 == 0;
      let light = fv > 0.52 && fv <= 0.66 && sv > 0.55 && hash % 11 == 0;
      let grove = fv > 0.72 && hash % 4 == 0;
      if (dense || light || grove)
        && seen.insert((wx as i32, wy as i32))
        && out.len() < 3_200
      {
        out.push((wx as i32, wy as i32));
      }
    }
  }
  out
}

// ---------------------------------------------------------------------------
// Town placement
// ---------------------------------------------------------------------------

const TOWN_SEARCH_STEP: usize = 30;
const TOWN_CHECK_RADIUS: usize = 10;
const MIN_TOWN_DIST_SQ: usize = 80 * 80;
const TARGET_TOWNS: usize = 4;

fn town_suitability(world: &ZoneWorld, cx: usize, cy: usize) -> f32 {
  let r = TOWN_CHECK_RADIUS as i32;
  let mut suitable = 0u32;
  let mut total = 0u32;
  for dy in -r..=r {
    for dx in -r..=r {
      let wx = cx as i32 + dx;
      let wy = cy as i32 + dy;
      if wx < 0 || wy < 0 { continue; }
      let (wx, wy) = (wx as usize, wy as usize);
      let zx = wx / ZONE_WIDTH;
      let zy = wy / ZONE_HEIGHT;
      if zx >= WORLD_COLS || zy >= WORLD_ROWS { continue; }
      let tile = world.zone(zx, zy, SURFACE_Z).tiles[wy % ZONE_HEIGHT][wx % ZONE_WIDTH];
      total += 1;
      if matches!(tile, Tile::Grass | Tile::TallGrass | Tile::Sand) { suitable += 1; }
    }
  }
  if total == 0 { 0.0 } else { suitable as f32 / total as f32 }
}

pub fn find_town_sites(world: &ZoneWorld) -> Vec<(usize, usize)> {
  let world_w = WORLD_COLS * ZONE_WIDTH;
  let world_h = WORLD_ROWS * ZONE_HEIGHT;
  let margin = TOWN_CHECK_RADIUS + 5;

  let mut candidates: Vec<(usize, usize, f32)> = (margin..world_h - margin)
    .step_by(TOWN_SEARCH_STEP)
    .flat_map(|cy| {
      (margin..world_w - margin)
        .step_by(TOWN_SEARCH_STEP)
        .filter_map(move |cx| {
          let score = town_suitability(world, cx, cy);
          (score >= 0.80).then_some((cx, cy, score))
        })
        .collect::<Vec<_>>()
    })
    .collect();

  candidates.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));

  let mut sites: Vec<(usize, usize)> = Vec::new();
  for (cx, cy, _) in candidates {
    let too_close = sites.iter().any(|&(sx, sy)| {
      let dx = cx as i64 - sx as i64;
      let dy = cy as i64 - sy as i64;
      ((dx * dx + dy * dy) as usize) < MIN_TOWN_DIST_SQ
    });
    if !too_close {
      sites.push((cx, cy));
      if sites.len() >= TARGET_TOWNS { break; }
    }
  }
  sites
}

fn place_building(
  world: &mut ZoneWorld,
  wx: i32,
  wy: i32,
  w: i32,
  h: i32,
  seed: u64,
  chest_sites: &mut Vec<(i32, i32, usize)>,
) {
  for dy in 0..h {
    for dx in 0..w {
      let tile = if dx == 0 || dx == w - 1 || dy == 0 || dy == h - 1 {
        Tile::WoodWall
      } else {
        Tile::WoodFloor
      };
      let (ex, ey) = (wx + dx, wy + dy);
      if ex >= 0 && ey >= 0 {
        let (uex, uey) = (ex as usize, ey as usize);
        let (zx, zy) = (uex / ZONE_WIDTH, uey / ZONE_HEIGHT);
        if zx < WORLD_COLS && zy < WORLD_ROWS {
          let existing = world.zone(zx, zy, SURFACE_Z).tiles[uey % ZONE_HEIGHT][uex % ZONE_WIDTH];
          if !matches!(existing, Tile::DeepWater | Tile::ShallowWater | Tile::Lava) {
            set_world_tile(world, ex, ey, SURFACE_Z, tile);
          }
        }
      }
    }
  }
  set_world_tile(world, wx + w / 2, wy + h - 1, SURFACE_Z, Tile::Door);

  let mut interiors = Vec::new();
  for dy in 1..h - 1 {
    for dx in 1..w - 1 {
      interiors.push((wx + dx, wy + dy));
    }
  }
  let mut rng = StdRng::seed_from_u64(seed ^ (wx as u64).rotate_left(12) ^ (wy as u64).rotate_left(24));
  if rng.random_bool(0.55) && !interiors.is_empty() {
    let i = rng.random_range(0..interiors.len());
    let (ex, ey) = interiors[i];
    chest_sites.push((ex, ey, SURFACE_Z));
  }
}

pub fn place_town(
  world: &mut ZoneWorld,
  cx: usize,
  cy: usize,
  seed: u64,
  chest_sites: &mut Vec<(i32, i32, usize)>,
) {
  const ROAD_HALF: i32 = 12;
  const BLDG_W: i32 = 7;
  const BLDG_H: i32 = 6;
  let (cx, cy) = (cx as i32, cy as i32);

  for dx in -ROAD_HALF..=ROAD_HALF { set_world_tile(world, cx + dx, cy, SURFACE_Z, Tile::Road); }
  for dy in -ROAD_HALF..=ROAD_HALF { set_world_tile(world, cx, cy + dy, SURFACE_Z, Tile::Road); }

  for (i, (ox, oy)) in [
    (2, 2),
    (-(BLDG_W + 2), 2),
    (2, -(BLDG_H + 2)),
    (-(BLDG_W + 2), -(BLDG_H + 2)),
  ]
  .into_iter()
  .enumerate()
  {
    place_building(
      world,
      cx + ox,
      cy + oy,
      BLDG_W,
      BLDG_H,
      seed.wrapping_add(i as u64 + 1),
      chest_sites,
    );
  }
}

/// Sparse chest spawn sites on cave floors (entities spawned at startup).
fn append_cave_chest_sites(world: &ZoneWorld, seed: u64, chest_sites: &mut Vec<(i32, i32, usize)>) {
  for zx in 0..WORLD_COLS {
    for zy in 0..WORLD_ROWS {
      for z in [0usize, 1] {
        for ty in 0..ZONE_HEIGHT {
          for tx in 0..ZONE_WIDTH {
            let wx = (zx * ZONE_WIDTH + tx) as i32;
            let wy = (zy * ZONE_HEIGHT + ty) as i32;
            let zone = world.zone(zx, zy, z);
            if zone.tiles[ty][tx] != Tile::CaveFloor {
              continue;
            }
            let h = chest_spawn_hash(wx, wy, z, seed);
            let thresh = if z == 0 { 7u64 } else { 9 };
            if h % 12_000 < thresh {
              chest_sites.push((wx, wy, z));
            }
          }
        }
      }
    }
  }
}

fn chest_spawn_hash(wx: i32, wy: i32, z: usize, seed: u64) -> u64 {
  let mut x = seed
    ^ (wx as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)
    ^ (wy as u64).wrapping_mul(0xC6BC_2796_92B5_C323);
  x ^= (z as u64).wrapping_mul(0xD6E8_FEB8_6755_73C2);
  x = x.wrapping_mul(0xBF58_476D_1CE4_E5B9);
  x ^ (x >> 32)
}

// ---------------------------------------------------------------------------
// Stairs
// ---------------------------------------------------------------------------

pub fn place_world_stairs(world: &mut ZoneWorld) {
  // Find a land zone for the surface entrance — search from island center outward
  let (czx, czy) = (WORLD_COLS / 2, WORLD_ROWS / 2);

  // Surface → shallow cave
  'outer: for zx in czx.saturating_sub(3)..=(czx + 3).min(WORLD_COLS - 1) {
    for zy in czy.saturating_sub(3)..=(czy + 3).min(WORLD_ROWS - 1) {
      for ty in 5..ZONE_HEIGHT - 5 {
        for tx in 5..ZONE_WIDTH - 5 {
          let surf  = world.zone(zx, zy, SURFACE_Z).tiles[ty][tx];
          let cave1 = world.zone(zx, zy, 1).tiles[ty][tx];
          if matches!(surf, Tile::Grass | Tile::Sand | Tile::TallGrass) && cave1 == Tile::CaveFloor {
            world.zone_mut(zx, zy, SURFACE_Z).tiles[ty][tx] = Tile::StairsDown;
            world.zone_mut(zx, zy, 1).tiles[ty][tx] = Tile::StairsUp;
            break 'outer;
          }
        }
      }
    }
  }

  // Shallow cave → deep cave (search same region)
  'outer2: for zx in czx.saturating_sub(3)..=(czx + 3).min(WORLD_COLS - 1) {
    for zy in czy.saturating_sub(3)..=(czy + 3).min(WORLD_ROWS - 1) {
      for ty in 0..ZONE_HEIGHT {
        for tx in 0..ZONE_WIDTH {
          let cave1 = world.zone(zx, zy, 1).tiles[ty][tx];
          let cave0 = world.zone(zx, zy, 0).tiles[ty][tx];
          if cave1 == Tile::CaveFloor && cave0 == Tile::CaveFloor {
            world.zone_mut(zx, zy, 1).tiles[ty][tx] = Tile::StairsDown;
            world.zone_mut(zx, zy, 0).tiles[ty][tx] = Tile::StairsUp;
            break 'outer2;
          }
        }
      }
    }
  }
}

// ---------------------------------------------------------------------------
// Main entry point
// ---------------------------------------------------------------------------

pub fn generate_world(seed: u64) -> ZoneWorld {
  let surface_noise: Fbm<Perlin> = Fbm::new(seed as u32);
  let cave_noise:    Fbm<Perlin> = Fbm::new(seed.wrapping_add(1) as u32);

  let mut world = ZoneWorld::new(Tile::Air);
  let mut chest_sites = Vec::new();

  // Surface
  for zx in 0..WORLD_COLS {
    for zy in 0..WORLD_ROWS {
      let zone = world.zone_mut(zx, zy, SURFACE_Z);
      for ty in 0..ZONE_HEIGHT {
        for tx in 0..ZONE_WIDTH {
          let (wx, wy) = (zx * ZONE_WIDTH + tx, zy * ZONE_HEIGHT + ty);
          zone.tiles[ty][tx] = tile_from_value(surface_tile_value(wx, wy, &surface_noise));
        }
      }
    }
  }

  place_rock_massifs(&mut world, seed);

  // z=0, z=1: underground caves
  for zx in 0..WORLD_COLS {
    for zy in 0..WORLD_ROWS {
      for z in 0..2usize {
        let zone = world.zone_mut(zx, zy, z);
        for ty in 0..ZONE_HEIGHT {
          for tx in 0..ZONE_WIDTH {
            let (wx, wy) = (zx * ZONE_WIDTH + tx, zy * ZONE_HEIGHT + ty);
            zone.tiles[ty][tx] = underground_tile(wx, wy, z, &cave_noise);
          }
        }
      }
    }
  }

  // Towns
  let sites = find_town_sites(&world);
  for (cx, cy) in sites {
    place_town(&mut world, cx, cy, seed, &mut chest_sites);
  }

  // Stairs
  place_world_stairs(&mut world);

  append_cave_chest_sites(&world, seed, &mut chest_sites);

  world.tree_sites = collect_tree_sites(&world, seed);
  world.chest_sites = chest_sites;

  world
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn island_mask_center_is_one() {
    let m = island_mask(WORLD_COLS * ZONE_WIDTH / 2, WORLD_ROWS * ZONE_HEIGHT / 2);
    assert!(m > 0.95, "center mask should be near 1.0, got {m}");
  }

  #[test]
  fn island_mask_corner_is_near_zero() {
    assert!(island_mask(0, 0) < 0.05);
  }

  #[test]
  fn island_mask_monotone_along_x() {
    let cy = WORLD_ROWS * ZONE_HEIGHT / 2;
    let cx = WORLD_COLS * ZONE_WIDTH / 2;
    assert!(island_mask(cx, cy) > island_mask(cx / 2, cy));
    assert!(island_mask(cx / 2, cy) > island_mask(0, cy));
  }

  #[test]
  fn tile_from_zero_is_deep_water() { assert_eq!(tile_from_value(0.0), Tile::DeepWater); }

  #[test]
  fn tile_from_mid_is_grass() { assert_eq!(tile_from_value(0.45), Tile::Grass); }

  #[test]
  fn tile_from_high_is_lava() { assert_eq!(tile_from_value(1.0), Tile::Lava); }

  #[test]
  fn tile_from_sand_range() { assert_eq!(tile_from_value(0.24), Tile::Sand); }

  #[test]
  fn tile_from_value_boundaries() {
    assert_eq!(tile_from_value(0.119), Tile::DeepWater);
    assert_eq!(tile_from_value(0.120), Tile::ShallowWater);
    assert_eq!(tile_from_value(0.199), Tile::ShallowWater);
    assert_eq!(tile_from_value(0.200), Tile::Sand);
    assert_eq!(tile_from_value(0.259), Tile::Sand);
    assert_eq!(tile_from_value(0.260), Tile::Grass);
    assert_eq!(tile_from_value(0.579), Tile::Grass);
    assert_eq!(tile_from_value(0.580), Tile::TallGrass);
    assert_eq!(tile_from_value(0.659), Tile::TallGrass);
    assert_eq!(tile_from_value(0.660), Tile::Bush);
    assert_eq!(tile_from_value(0.729), Tile::Bush);
    assert_eq!(tile_from_value(0.730), Tile::Ash);
    assert_eq!(tile_from_value(0.829), Tile::Ash);
    assert_eq!(tile_from_value(0.830), Tile::Lava);
  }

  #[test]
  fn surface_center_is_mostly_land() {
    let world = generate_world(WORLD_SEED);
    let zone = world.zone(WORLD_COLS / 2, WORLD_ROWS / 2, SURFACE_Z);
    let land = zone.tiles.iter().flatten()
      .filter(|&&t| matches!(t, Tile::Grass | Tile::TallGrass | Tile::Sand | Tile::Ash | Tile::Bush | Tile::Lava))
      .count();
    assert!(land > ZONE_WIDTH * ZONE_HEIGHT / 2, "center should be >50% land, got {land}");
  }

  #[test]
  fn underground_has_cave_floors() {
    let world = generate_world(WORLD_SEED);
    let zone = world.zone(WORLD_COLS / 2, WORLD_ROWS / 2, 1);
    let floors = zone.tiles.iter().flatten().filter(|&&t| t == Tile::CaveFloor).count();
    let total = ZONE_WIDTH * ZONE_HEIGHT;
    assert!(floors > total / 4, "underground should be >25% floor, got {floors}/{total}");
    assert!(floors < total * 3 / 4, "underground should be <75% floor, got {floors}/{total}");
  }

  #[test]
  fn world_has_stairs_down() {
    let world = generate_world(WORLD_SEED);
    let count: usize = (0..WORLD_COLS)
      .flat_map(|zx| (0..WORLD_ROWS).map(move |zy| (zx, zy)))
      .map(|(zx, zy)| world.zone(zx, zy, SURFACE_Z).tiles.iter().flatten().filter(|&&t| t == Tile::StairsDown).count())
      .sum();
    assert!(count > 0, "world should have at least one StairsDown");
  }
}
