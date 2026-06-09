//! Naturalistic planet generation using layered noise fields.
//!
//! Why a new algorithm: the existing WFC generator in `build.rs` is socket-only
//! — `ground` is interchangeable with `feature`/`shallow`/`rock` at every
//! edge, so it produces balanced random texture, not a world with continents,
//! coastlines, lakes, or forests. The editor-pattern WFC in `editor.rs` is
//! even more restricted: it only re-emits patterns that were already painted
//! on the input canvas, so the output sticks to the author.
//!
//! This generator combines a low-frequency **elevation** field, a low-frequency
//! **moisture** field, and higher-frequency **detail** fields into a layered
//! terrain model:
//!
//! * Elevation decides the *band* (deep water → shallow water → beach →
//!   lowland → midland → peak).
//! * Moisture decides the *biome* within land (grass vs tall-grass/forest).
//! * A detail field scatters rocks; a separate mask scatters trees.
//!
//! The result has coherent continents, real coastlines, lakes, forests, and
//! hills — none of which the existing implementations produce.
//!
//! See `generate_natural_planet` for the public entry point.

use {crate::{entities::*, galaxy::Location, level::{Level, LocationType, Tile}},
     noise::{Fbm, MultiFractal, NoiseFn, Perlin},
     rand::{Rng, RngCore, SeedableRng, rngs::SmallRng},
     std::collections::VecDeque};

pub const SEED: u64 = 0xCAFE_F00D;
pub const PLANET_SIZE: usize = 300;

// Elevation band thresholds (0..1 normalized noise).
const ELEV_DEEP: f32 = 0.32;
const ELEV_SHALLOW: f32 = 0.36;
const ELEV_BEACH: f32 = 0.40;
const ELEV_MIDLAND: f32 = 0.58;
const ELEV_PEAK: f32 = 0.76;

// Cave sublevel tunables.
const CAVE_ENTRANCES: usize = 3;
const CAVE_FILL_CHANCE: f64 = 0.44;
const CAVE_SMOOTH_PASSES: usize = 5;

// Tile-feature scatter probabilities (per eligible tile, after the density
// mask passes). Keep them low — natural terrain is mostly empty, with a few
// strong features per area.
const ROCK_MASK_THRESHOLD: f32 = 0.83;
const ROCK_DETAIL_THRESHOLD: f32 = 0.55;
const WATER_FRAGMENT_THRESHOLD: f32 = 0.50;

const RIDGE_COUNT: usize = 3;
const RIDGE_THICKNESS: i32 = 2;
const RIDGE_GAP_CHANCE: f64 = 0.10;
// Tree density bands: in each biome, the smoothstep maps raw noise density
// [0, 1] to per-tile tree probability. Lo is where the ramp starts (0%),
// hi is where it saturates (100%). The 0.30-wide band gives a gradual
// transition between grove and plain. Regional variation shifts these
// bands up or down by up to ±0.08.
const TREE_TG_LO: f32 = 0.20;
const TREE_TG_HI: f32 = 0.50;
const TREE_GR_LO: f32 = 0.42;
const TREE_GR_HI: f32 = 0.72;
const TREE_MAX_PROB: f32 = 0.85; // peak per-tile probability at the dense end

// Starting-zone seeding: minimum features guaranteed within sight of the dock
// so the player sees something interesting on first spawn, regardless of how
// the random noise happens to fall. (Without this, the dock can land in a
// boring patch that's just grass for 30+ tiles in every direction.)
const STARTING_TREES: usize = 8;
const STARTING_ROCKS: usize = 3;
const STARTING_CREATURES: usize = 2;
const STARTING_RING_MIN: i32 = 6;
const STARTING_RING_MAX: i32 = 28;
const STARTING_CREATURE_RING_MAX: i32 = 38;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Biome {
  DeepWater,
  ShallowWater,
  Sand,
  Grass,
  TallGrass,
  Rock
}

impl Biome {
  fn tile(self) -> Tile {
    match self {
      Self::DeepWater => Tile::DeepWater,
      Self::ShallowWater => Tile::ShallowWater,
      Self::Sand => Tile::Sand,
      Self::Grass => Tile::Grass,
      Self::TallGrass => Tile::TallGrass,
      Self::Rock => Tile::CaveWall
    }
  }
}

struct NoiseField {
  base: Fbm<Perlin>
}

impl NoiseField {
  fn new(seed: u32, frequency: f64, octaves: usize, persistence: f64) -> Self {
    let base = Fbm::<Perlin>::new(seed)
      .set_frequency(frequency)
      .set_octaves(octaves)
      .set_persistence(persistence);
    Self { base }
  }

  /// Returns a noise value remapped from roughly [-1, 1] to [0, 1].
  fn sample01(&self, x: f64, y: f64) -> f64 { (self.base.get([x, y]) + 1.0) * 0.5 }
}

fn classify(e: f32, m: f32) -> Biome {
  if e < ELEV_DEEP {
    Biome::DeepWater
  } else if e < ELEV_SHALLOW {
    Biome::ShallowWater
  } else if e < ELEV_BEACH {
    Biome::Sand
  } else if e < ELEV_MIDLAND {
    Biome::Grass
  } else if e < ELEV_PEAK {
    if m > 0.50 { Biome::TallGrass } else { Biome::Grass }
  } else {
    Biome::Rock
  }
}

pub struct NaturalParams {
  pub name: &'static str,
  pub seed: u64,
  pub breathable: bool,
  /// 0..1 — scales the chance that an eligible cell grows a tree.
  pub tree_density: f32
}

pub fn generate_natural_planet(params: &NaturalParams) -> Location {
  // Different seeds per field keep the bands uncorrelated (no bias toward
  // wet mountains / dry coasts). Frequencies are in *tile units* — 0.005
  // means one Perlin cycle per 200 tiles, so 300 tiles sees ~1.5 cycles.
  let elev = NoiseField::new(params.seed as u32, 0.0050, 4, 0.5);
  let moist = NoiseField::new((params.seed ^ 0x9E37_79B9) as u32, 0.0042, 2, 0.5);
  let detail = NoiseField::new((params.seed ^ 0xBF58_476D) as u32, 0.025, 3, 0.45);
  let rock_mask = NoiseField::new((params.seed ^ 0x94D0_49BB) as u32, 0.018, 2, 0.5);
  // Tree mask: 4 octaves of FBM at moderate frequency. The multi-octave
  // structure means the boundary between "forest" and "plain" is a fractal
  // edge with small clearings intruding into groves, not a hard line. The
  // smoothstep pass below turns the hard per-tile threshold into a soft
  // probability ramp across an 0.30-wide band of density values.
  let tree_mask = NoiseField::new((params.seed ^ 0x1234_5678) as u32, 0.020, 4, 0.5);
  // Density variation: a single very-low-freq octave that shifts the
  // per-region tree threshold up or down, so some areas of the map are
  // naturally denser than others — the edge of a forest and the heart of
  // a forest feel different, not just "inside mask" vs "outside mask".
  let density_variation = NoiseField::new((params.seed ^ 0xABCD_1234) as u32, 0.0040, 1, 0.5);
  // Dirt mask: mid-frequency patches where the ground becomes bare earth
  // (Tile::Ground) instead of grass. The threshold is high (0.72) so only
  // a small fraction of grass cells become dirt — enough to break up the
  // visual monotony of grass sprites, not so much that dirt dominates.


  let mut loc = Location::new(
    params.name,
    PLANET_SIZE,
    PLANET_SIZE,
    2,
    LocationType::PlanetSurface { breathable: params.breathable },
    Tile::Grass
  );

  let mut spawn_objects: Vec<(i32, i32, usize, Object)> = Vec::new();
  let mut rng = SmallRng::seed_from_u64(params.seed);

  // Pass 1: classify every cell by elevation+moisture.
  {
    let level = loc.level_mut(0);
    for y in 0..PLANET_SIZE {
      for x in 0..PLANET_SIZE {
        let e = elev.sample01(x as f64, y as f64) as f32;
        let m = moist.sample01(x as f64, y as f64) as f32;
        level.set(x as i32, y as i32, classify(e, m).tile());
      }
    }
  }

  // Pass 1.5: fragment big water bodies. The elev field is low-frequency
  // (~1.5 cycles across 300 tiles) so shallow water tends to form one or
  // two big connected lakes. Where the detail mask is high, we promote
  // shallow water to land — this cuts peninsulas into the water and
  // breaks big pools into smaller ones, without losing the deep water
  // mass in the center of each lake.
  {
    let level = loc.level_mut(0);
    for y in 0..PLANET_SIZE {
      for x in 0..PLANET_SIZE {
        let t = level.get(x as i32, y as i32).unwrap();
        if t != Tile::ShallowWater {
          continue;
        }
        let d = detail.sample01(x as f64, y as f64) as f32;
        if d > WATER_FRAGMENT_THRESHOLD {
          level.set(x as i32, y as i32, Tile::Grass);
        }
      }
    }
  }

  // Pass 2: beach band. For each grass/tall_grass tile that has at least one
  // water neighbor, upgrade to sand. This draws a one-tile beach ring around
  // every water body, which is what makes a coastline read as a coastline
  // rather than a hard water/grass edge.
  {
    let level = loc.level_mut(0);
    let mut to_sand = vec![false; PLANET_SIZE * PLANET_SIZE];
    for y in 1..PLANET_SIZE - 1 {
      for x in 1..PLANET_SIZE - 1 {
        let t = level.get(x as i32, y as i32).unwrap_or(Tile::Wall);
        if !matches!(t, Tile::Grass | Tile::TallGrass) {
          continue;
        }
        let water_near = [(1, 0), (-1, 0), (0, 1), (0, -1)]
          .into_iter()
          .any(|(dx, dy)| {
            level
              .get(x as i32 + dx, y as i32 + dy)
              .is_some_and(|n| matches!(n, Tile::DeepWater | Tile::ShallowWater))
          });
        if water_near {
          to_sand[y * PLANET_SIZE + x] = true;
        }
      }
    }
    for y in 0..PLANET_SIZE {
      for x in 0..PLANET_SIZE {
        if to_sand[y * PLANET_SIZE + x] {
          level.set(x as i32, y as i32, Tile::Sand);
        }
      }
    }
  }

  // Pass 3: scatter small decorative rocks in lowland/midland cells. The
  // rock mask is a sparse field, so rocks cluster naturally into small
  // boulder fields rather than appearing uniformly. Threshold is high
  // (0.83) so the base scatter is sparse — the ridge pass below
  // contributes the connected CaveWall ridges that limit movement, while
  // these Tile::SmallRocks are walkable decorative pebbles.
  {
    let level = loc.level_mut(0);
    for y in 1..PLANET_SIZE - 1 {
      for x in 1..PLANET_SIZE - 1 {
        let t = level.get(x as i32, y as i32).unwrap_or(Tile::Wall);
        if !matches!(t, Tile::Grass | Tile::TallGrass) {
          continue;
        }
        let r = rock_mask.sample01(x as f64, y as f64) as f32;
        let d = detail.sample01(x as f64, y as f64) as f32;
        if r > ROCK_MASK_THRESHOLD && d > ROCK_DETAIL_THRESHOLD {
          level.set(x as i32, y as i32, Tile::SmallRocks);
        }
      }
    }
  }

  // Pass 3.5: meandering rock-wall ridges. Instead of straight-ish lines
  // with a small sine wobble, each ridge is a noise-guided random walk that
  // snakes across the map like a real fault line or volcanic dyke. The walk
  // steers by sampling the detail mask at each step, giving long organic
  // curves with branches, tight turns, and open loops.
  {
    let level = loc.level_mut(0);
    for wall_idx in 0..RIDGE_COUNT {
      let (mut x, mut y) = {
        let sx = rng.gen_range(20_u32..(PLANET_SIZE as u32 - 20)) as f64;
        let sy = rng.gen_range(20_u32..(PLANET_SIZE as u32 - 20)) as f64;
        (sx, sy)
      };
      let mut heading = (rng.next_u64() as f64 / u64::MAX as f64) * std::f64::consts::TAU;
      for _step in 0..(PLANET_SIZE * 5) {
        let gx = x as i32;
        let gy = y as i32;
        if gx >= 1 && gy >= 1 && gx < (PLANET_SIZE as i32 - 1) && gy < (PLANET_SIZE as i32 - 1)
          && !rng.gen_bool(RIDGE_GAP_CHANCE)
        {
          // Draw a diamond-shaped footprint for thickness, with outer tiles
          // sometimes skipped to feather the edge.
          for dx in -RIDGE_THICKNESS..=RIDGE_THICKNESS {
            for dy in -RIDGE_THICKNESS..=RIDGE_THICKNESS {
              if dx.abs() + dy.abs() > RIDGE_THICKNESS {
                continue;
              }
              let nx = gx + dx;
              let ny = gy + dy;
              if nx < 1 || ny < 1 || nx >= (PLANET_SIZE as i32 - 1) || ny >= (PLANET_SIZE as i32 - 1)
              {
                continue;
              }
              let is_outer = dx.abs() + dy.abs() == RIDGE_THICKNESS;
              if is_outer && rng.gen_bool(0.4) {
                continue;
              }
              // Only place walls on walkable land so water is preserved.
              let t = level.get(nx, ny).unwrap_or(Tile::Wall);
              if matches!(t, Tile::Grass | Tile::TallGrass | Tile::Ground | Tile::SmallRocks) {
                level.set(nx, ny, Tile::CaveWall);
              }
            }
          }
        }
        // Steer: sample detail noise to pick turn amount. A different seed
        // offset per wall_idx keeps the ridges uncorrelated.
        let steer = detail.sample01(x, y + wall_idx as f64 * 137.0);
        let turn = (steer - 0.5) * 1.2;
        let jitter = (rng.next_u64() as f64 / u64::MAX as f64 - 0.5) * 0.3;
        heading += turn + jitter;
        x += heading.cos() * 0.7;
        y += heading.sin() * 0.7;
      }
    }
  }

  // Pass 3.7: scattered dirt. Use high-frequency noise so Ground patches
  // are small and irregular instead of big contiguous blobs. Two
  // thresholds from the same detail field with different offsets give a
  // mix of patch sizes: medium clusters and tiny individual spots. No
  // skirts — skirts merge separate patches into big ugly blobs.
  {
    let level = loc.level_mut(0);
    for y in 0..PLANET_SIZE {
      for x in 0..PLANET_SIZE {
        if level.get(x as i32, y as i32) != Some(Tile::Grass) {
          continue;
        }
        let d1 = detail.sample01(x as f64 + 1000.0, y as f64 + 500.0) as f32;
        if d1 > 0.82 {
          level.set(x as i32, y as i32, Tile::Ground);
        }
      }
    }
    for y in 0..PLANET_SIZE {
      for x in 0..PLANET_SIZE {
        if level.get(x as i32, y as i32) != Some(Tile::Grass) {
          continue;
        }
        let d2 = detail.sample01(x as f64 + 2500.0, y as f64 + 3700.0) as f32;
        if d2 > 0.92 {
          level.set(x as i32, y as i32, Tile::Ground);
        }
      }
    }
  }

  // Pass 4: place the ship dock on a flat clearing in the largest landmass.
  // The dock is a single tile; we clear a 3x3 around it so the player can
  // step off and trees don't spawn on it.
  let dock = place_ship_dock(loc.level_mut(0));

  // Pass 5: scatter trees using the tree density mask. Instead of a hard
  // threshold ("density > X means tree, else no tree"), the per-tile tree
  // probability ramps smoothly across a 0.30-wide band of density values
  // via smoothstep. The result: groves fade into plains over many tiles,
  // and the boundary is shaped by the FBM's fine detail so it isn't a
  // straight line. The density_variation mask shifts the band up or down
  // per region, so some parts of the map are denser than others.
  {
    let level = loc.level_mut(0);
    for y in 1..PLANET_SIZE - 1 {
      for x in 1..PLANET_SIZE - 1 {
        let t = level.get(x as i32, y as i32).unwrap_or(Tile::Wall);
        // Per-biome density band. Lo..hi is the range over which the
        // tree probability ramps from 0% to 100%.
        let (lo, hi) = match t {
          Tile::TallGrass => (TREE_TG_LO, TREE_TG_HI),
          Tile::Grass => (TREE_GR_LO, TREE_GR_HI),
          _ => continue
        };
        // Regional shift: dense groves where variation is high, sparse
        // forest edge where it's low. ±0.08 keeps the overall mix close
        // to the base biome target.
        let variation = density_variation.sample01(x as f64, y as f64) as f32;
        let lo = lo + variation * 0.08;
        let hi = hi + variation * 0.08;
        let density = tree_mask.sample01(x as f64, y as f64) as f32;
        let p = smoothstep(lo, hi, density) * TREE_MAX_PROB * params.tree_density;
        if p <= 0.0 {
          continue;
        }
        // Don't place trees next to the dock (keep the landing pad clear).
        let too_close_to_dock = (x as i32 - dock.0).abs() <= 1
          && (y as i32 - dock.1).abs() <= 1;
        if too_close_to_dock {
          continue;
        }
        if rng.gen_bool(p as f64) {
          spawn_objects.push((x as i32, y as i32, 0, Object::random_tree()));
        }
      }
    }
  }

  // Pass 5.5: starting-zone seeding. The dock is where the player spawns;
  // guarantee a few trees, rocks, and creatures in a ring around it so the
  // first screen the player sees is interesting. Walk the ring at evenly
  // spaced angles and find a walkable tile of the right biome for each
  // feature.
  seed_starting_zone(&mut loc, dock, &mut spawn_objects, &mut rng);

  // Pass 5.6: global creature scatter. Hostile fauna in plains/forest, plus
  // a few boulders as decoration in rocky areas. The starting-zone pass
  // already placed 1-2 creatures near the dock; this populates the rest of
  // the map.
  scatter_creatures(&loc, params.seed, &mut spawn_objects);

  // Pass 5.7: intersperse tall grass into plain grass for visual variety.
  // This runs *after* tree placement so it only affects the tile sprite,
  // not tree density. Small irregular patches from detail noise.
  {
    let level = loc.level_mut(0);
    for y in 0..PLANET_SIZE {
      for x in 0..PLANET_SIZE {
        if level.get(x as i32, y as i32) != Some(Tile::Grass) {
          continue;
        }
        let d = detail.sample01(x as f64 + 800.0, y as f64 + 1200.0) as f32;
        if d > 0.78 {
          level.set(x as i32, y as i32, Tile::TallGrass);
        }
      }
    }
  }

  // Pass 5.8: intersperse scattered ground into remaining grass. Same idea:
  // tiny visual patches that break up the grass monotony.
  {
    let level = loc.level_mut(0);
    for y in 0..PLANET_SIZE {
      for x in 0..PLANET_SIZE {
        if level.get(x as i32, y as i32) != Some(Tile::Grass) {
          continue;
        }
        let d = detail.sample01(x as f64 + 500.0, y as f64 + 1800.0) as f32;
        if d > 0.90 {
          level.set(x as i32, y as i32, Tile::Ground);
        }
      }
    }
  }

  // Pass 6: cave sublevel. Cellular automata carves the level, and entrances
  // are placed on the surface only where midland+highland cells exist — so
  // caves look like they emerge from hills, not from lakes.
  generate_cave_sublevel(&mut loc, params.seed, &mut spawn_objects);

  loc.spawn_objects.extend(spawn_objects);
  loc
}

/// Pick a random cell in a ring around `dock` that satisfies `accept`.
/// Returns None if the whole ring is rejected (e.g. dock is in a giant lake).
/// Hermite smoothstep: maps x from [edge0, edge1] to [0, 1] with zero
/// derivatives at both ends, so transitions have no visible kinks. Used
/// to convert the raw tree-density noise into a per-tile probability ramp
/// that fades gradually from "no trees" to "dense grove" over many tiles.
fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
  let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
  t * t * (3.0 - 2.0 * t)
}

fn pick_in_ring<R: Rng>(
  rng: &mut R,
  dock: (i32, i32),
  min_r: i32,
  max_r: i32,
  accept: impl Fn(i32, i32) -> bool
) -> Option<(i32, i32)> {
  // Try a few hundred candidates, jittered by ring radius and angle. This is
  // more reliable than random sampling because we ensure the result is
  // outside the dock's 3x3 and inside the planet.
  for _ in 0..200 {
    let angle = rng.gen_range(0.0..std::f64::consts::TAU);
    let radius = rng.gen_range(min_r as f64..=max_r as f64);
    let x = dock.0 + (angle.cos() * radius).round() as i32;
    let y = dock.1 + (angle.sin() * radius).round() as i32;
    if x < 2 || y < 2 || x >= PLANET_SIZE as i32 - 2 || y >= PLANET_SIZE as i32 - 2 {
      continue;
    }
    if accept(x, y) {
      return Some((x, y));
    }
  }
  None
}

/// Seed a few trees, rocks, and creatures in a ring around the dock so the
/// player sees something interesting on first spawn.
fn seed_starting_zone(
  loc: &mut Location,
  dock: (i32, i32),
  spawn_objects: &mut Vec<(i32, i32, usize, Object)>,
  rng: &mut SmallRng
) {
  let level = loc.level_mut(0);

  // Trees: walkable land cells (grass or tall_grass), not in water.
  let mut placed_trees = 0;
  for _ in 0..STARTING_TREES * 4 {
    if placed_trees >= STARTING_TREES {
      break;
    }
    let Some((x, y)) = pick_in_ring(rng, dock, STARTING_RING_MIN, STARTING_RING_MAX, |x, y| {
      matches!(level.get(x, y), Some(Tile::Grass) | Some(Tile::TallGrass))
    }) else {
      break;
    };
    spawn_objects.push((x, y, 0, Object::random_tree()));
    placed_trees += 1;
  }

  // Small decorative rocks: replace grass/tall_grass tiles with Tile::SmallRocks
  // (walkable pebbles). We do this *after* trees so a tree and a rock don't
  // land on the same tile. The wall ridges (Pass 3.5) place the
  // impassable CaveWall barriers separately.
  let mut placed_rocks = 0;
  for _ in 0..STARTING_ROCKS * 6 {
    if placed_rocks >= STARTING_ROCKS {
      break;
    }
    let Some((x, y)) = pick_in_ring(rng, dock, STARTING_RING_MIN, STARTING_RING_MAX, |x, y| {
      matches!(level.get(x, y), Some(Tile::Grass) | Some(Tile::TallGrass))
        && !spawn_objects.iter().any(|(lx, ly, lz, _)| *lx == x && *ly == y && *lz == 0)
    }) else {
      break;
    };
    level.set(x, y, Tile::SmallRocks);
    placed_rocks += 1;
  }

  // Creatures: walkable cells (any land, not water) at slightly larger range.
  let mut placed_creatures = 0;
  for _ in 0..STARTING_CREATURES * 4 {
    if placed_creatures >= STARTING_CREATURES {
      break;
    }
    let Some((x, y)) = pick_in_ring(
      rng,
      dock,
      STARTING_RING_MIN,
      STARTING_CREATURE_RING_MAX,
      |x, y| {
        matches!(
          level.get(x, y),
          Some(Tile::Grass) | Some(Tile::TallGrass) | Some(Tile::Sand)
        )
      }
    ) else {
      break;
    };
    // Pick a creature appropriate for the local biome, weighted toward
    // aliens in this version of Vera Spera.
    let tile = level.get(x, y).unwrap();
    let creature = pick_creature_for_biome(tile, rng);
    spawn_objects.push((x, y, 0, creature));
    placed_creatures += 1;
  }
}

/// Pick a creature for the given biome tile, weighted so aliens dominate and
/// robot dogs are an occasional surprise in the plains.
fn pick_creature_for_biome(tile: Tile, rng: &mut SmallRng) -> Object {
  let (options, weights): (Vec<Object>, Vec<u32>) = match tile {
    Tile::TallGrass => (
      vec![Object::MUSHROOM_CREATURE, Object::ALIEN_RUNNER, Object::MANTIS_ALIEN],
      vec![40, 35, 25]
    ),
    Tile::Grass => (
      vec![
        Object::ALIEN_RUNNER,
        Object::MANTIS_ALIEN,
        Object::RAT_SOLDIER,
        Object::ROBOT_DOG,
        Object::ARMORED_RAT_SOLDIER
      ],
      vec![25, 20, 20, 20, 15]
    ),
    Tile::Sand => (vec![Object::CRAB_ALIEN, Object::MANTIS_ALIEN], vec![60, 40]),
    _ => (vec![Object::RAT_SOLDIER], vec![1])
  };
  pick_weighted(&options, &weights, rng)
}

/// Weighted random pick: `weights` are relative (don't need to sum to 1).
fn pick_weighted<T: Clone>(options: &[T], weights: &[u32], rng: &mut SmallRng) -> T {
  let total: u32 = weights.iter().sum();
  let mut roll = rng.gen_range(0..total);
  for (opt, w) in options.iter().zip(weights.iter()) {
    if roll < *w {
      return opt.clone();
    }
    roll -= w;
  }
  options.last().unwrap().clone()
}

/// Scatter creatures across the map by biome. Counts are calibrated so the
/// planet feels alive but not crowded. Each placed creature is weighted
/// randomly from a biome-appropriate pool so the fauna isn't homogeneous
/// (e.g. you don't see a single rat-soldier army).
fn scatter_creatures(loc: &Location, seed: u64, spawn_objects: &mut Vec<(i32, i32, usize, Object)>) {
  let mut rng = SmallRng::seed_from_u64(seed ^ 0xC0FF_EE42);
  let level = loc.level(0);

  let mut tall_grass_cells: Vec<(i32, i32)> = Vec::new();
  let mut grass_cells: Vec<(i32, i32)> = Vec::new();
  let mut sand_cells: Vec<(i32, i32)> = Vec::new();
  for y in 2..loc.height as i32 - 2 {
    for x in 2..loc.width as i32 - 2 {
      match level.get(x, y) {
        Some(Tile::TallGrass) => tall_grass_cells.push((x, y)),
        Some(Tile::Grass) => grass_cells.push((x, y)),
        Some(Tile::Sand) => sand_cells.push((x, y)),
        _ => ()
      }
    }
  }

  let place_n_biome = |cells: &mut Vec<(i32, i32)>,
                       count: usize,
                       taken: &[(i32, i32)],
                       tile: Tile,
                       spawn_objects: &mut Vec<(i32, i32, usize, Object)>,
                       rng: &mut SmallRng| {
    if cells.is_empty() {
      return;
    }
    let mut placed = 0;
    for _ in 0..count * 3 {
      if placed >= count {
        break;
      }
      let idx = rng.gen_range(0..cells.len());
      let (x, y) = cells[idx];
      if taken.contains(&(x, y)) {
        continue;
      }
      spawn_objects.push((x, y, 0, pick_creature_for_biome(tile, rng)));
      placed += 1;
    }
  };

  // Collect already-taken cells (from the starting-zone pass).
  let taken: Vec<(i32, i32)> = spawn_objects.iter().map(|(x, y, _, _)| (*x, *y)).collect();

  let mut cells = tall_grass_cells;
  let count = (cells.len() / 1200).clamp(3, 18);
  place_n_biome(&mut cells, count, &taken, Tile::TallGrass, spawn_objects, &mut rng);
  let mut cells = grass_cells;
  let count = (cells.len() / 2500).clamp(5, 28);
  place_n_biome(&mut cells, count, &taken, Tile::Grass, spawn_objects, &mut rng);
  let mut cells = sand_cells;
  let count = (cells.len() / 1500).clamp(1, 6);
  place_n_biome(&mut cells, count, &taken, Tile::Sand, spawn_objects, &mut rng);
}

fn place_ship_dock(level: &mut Level) -> (i32, i32) {
  // Flood-fill every walkable component and find the largest one. The dock
  // must land there so the player isn't isolated in a small pocket.
  let (w, h) = (level.width as i32, level.height as i32);
  let mut visited = vec![vec![false; level.width]; level.height];
  let mut best: Vec<(i32, i32)> = Vec::new();

  for sy in 0..h {
    for sx in 0..w {
      if visited[sy as usize][sx as usize] || !level.walkable(sx, sy) {
        continue;
      }
      let mut component = Vec::new();
      let mut queue = VecDeque::new();
      visited[sy as usize][sx as usize] = true;
      queue.push_back((sx, sy));
      while let Some((x, y)) = queue.pop_front() {
        component.push((x, y));
        for (dx, dy) in [(1i32, 0), (-1, 0), (0, 1), (0, -1)] {
          let (nx, ny) = (x + dx, y + dy);
          if nx >= 0
            && ny >= 0
            && nx < w
            && ny < h
            && !visited[ny as usize][nx as usize]
            && level.walkable(nx, ny)
          {
            visited[ny as usize][nx as usize] = true;
            queue.push_back((nx, ny));
          }
        }
      }
      if component.len() > best.len() {
        best = component;
      }
    }
  }

  // Within the largest component, pick the tile closest to map center.
  let (cx, cy) = (w / 2, h / 2);
  let Some(&(sx, sy)) = best
    .iter()
    .min_by_key(|&&(x, y)| (x - cx).abs() + (y - cy).abs())
  else {
    // No walkable component at all — clear the center and place the dock.
    for dy in -1..=1 {
      for dx in -1..=1 {
        level.set(cx + dx, cy + dy, Tile::Grass);
      }
    }
    level.set(cx, cy, Tile::ShipDock);
    return (cx, cy);
  };

  // Clear a 3x3 around the dock so the player can step off.
  for dy in -1..=1 {
    for dx in -1..=1 {
      let nx = (sx + dx).clamp(0, w - 1);
      let ny = (sy + dy).clamp(0, h - 1);
      if !level.walkable(nx, ny) {
        level.set(nx, ny, Tile::Grass);
      }
    }
  }
  level.set(sx, sy, Tile::ShipDock);
  (sx, sy)
}

fn is_solid_ground(tile: Tile) -> bool {
  matches!(
    tile,
    Tile::Grass
      | Tile::TallGrass
      | Tile::Sand
      | Tile::Ground
      | Tile::CaveWall
      | Tile::CaveFloor
  )
}

fn generate_cave_sublevel(
  loc: &mut Location,
  seed: u64,
  spawn_objects: &mut Vec<(i32, i32, usize, Object)>
) {
  let size = loc.width;
  let mut rng = SmallRng::seed_from_u64(seed ^ 0xDEAD_BEEF);

  let cave = loc.level_mut(1);
  for y in 0..size {
    for x in 0..size {
      cave.set(x as i32, y as i32, Tile::CaveWall);
    }
  }

  // Cellular automata: seed random floor cells, then smooth into caves.
  let mut cells = vec![vec![false; size]; size];
  for y in 2..size - 2 {
    for x in 2..size - 2 {
      cells[y][x] = rng.gen_bool(CAVE_FILL_CHANCE);
    }
  }
  for _ in 0..CAVE_SMOOTH_PASSES {
    let prev = cells.clone();
    for y in 1..size - 1 {
      for x in 1..size - 1 {
        let neighbors = (-1..=1i32)
          .flat_map(|dy| (-1..=1i32).map(move |dx| (dx, dy)))
          .filter(|&(dx, dy)| (dx, dy) != (0, 0))
          .filter(|&(dx, dy)| prev[(y as i32 + dy) as usize][(x as i32 + dx) as usize])
          .count();
        cells[y][x] = neighbors >= 5 || (cells[y][x] && neighbors >= 4);
      }
    }
  }

  for y in 0..size {
    for x in 0..size {
      if cells[y][x] {
        cave.set(x as i32, y as i32, Tile::CaveFloor);
      }
    }
  }

  // Find the largest connected cave region.
  let largest = largest_walkable_component(cave);
  if largest.len() < 20 {
    return;
  }

  // Entrance candidates: cave tiles that are below walkable surface terrain.
  // The cave CA produces a fragmented region (typically a handful of
  // disconnected pockets), so we just accept any solid-ground surface cell
  // — midland/highland/ground are all fine places for a cave mouth. Trying
  // to *prefer* hills gives 0 candidates on a small island, so don't.
  let surface = loc.levels[0].clone();
  let mut entrance_candidates: Vec<(i32, i32)> = largest
    .iter()
    .copied()
    .filter(|&(x, y)| {
      x > 5
        && y > 5
        && x < (size as i32 - 5)
        && y < (size as i32 - 5)
        && is_solid_ground(surface.get(x, y).unwrap_or(Tile::CaveWall))
    })
    .collect();

  if entrance_candidates.is_empty() {
    return;
  }

  entrance_candidates.sort_by_key(|&(x, y)| (x, y));
  let count = CAVE_ENTRANCES.min(entrance_candidates.len());
  let step = entrance_candidates.len() / count;
  let entrances: Vec<(i32, i32)> =
    (0..count).map(|i| entrance_candidates[i * step]).collect();

  for &(ex, ey) in &entrances {
    // Clear a small area around the entrance in the cave.
    for dy in -1..=1 {
      for dx in -1..=1 {
        let cave = loc.level_mut(1);
        if !cave.walkable(ex + dx, ey + dy) {
          cave.set(ex + dx, ey + dy, Tile::CaveFloor);
        }
      }
    }
    spawn_objects.push((ex, ey, 0, Object::cave_entrance(ex, ey, ex, ey)));
    spawn_objects.push((ex, ey, 1, Object::cave_exit(ex, ey, ex, ey)));
  }

  // Scatter loot chests in the cave away from the entrances.
  let chest_candidates: Vec<(i32, i32)> = largest
    .iter()
    .copied()
    .filter(|&(x, y)| {
      !entrances.contains(&(x, y))
        && x > 3
        && y > 3
        && x < (size as i32 - 3)
        && y < (size as i32 - 3)
    })
    .collect();
  let chest_count = (chest_candidates.len() / 80).clamp(2, 6);
  let chest_step = chest_candidates.len().max(1) / chest_count.max(1);
  for i in 0..chest_count.min(chest_candidates.len()) {
    let (cx, cy) = chest_candidates[i * chest_step];
    spawn_objects.push((cx, cy, 1, Object::LOOT_CHEST));
  }
}

fn largest_walkable_component(level: &Level) -> Vec<(i32, i32)> {
  let (w, h) = (level.width as i32, level.height as i32);
  let mut visited = vec![vec![false; level.width]; level.height];
  let mut best = Vec::new();

  for sy in 0..h {
    for sx in 0..w {
      if visited[sy as usize][sx as usize] || !level.walkable(sx, sy) {
        continue;
      }
      let mut component = Vec::new();
      let mut queue = VecDeque::new();
      visited[sy as usize][sx as usize] = true;
      queue.push_back((sx, sy));
      while let Some((x, y)) = queue.pop_front() {
        component.push((x, y));
        for (dx, dy) in [(1i32, 0), (-1, 0), (0, 1), (0, -1)] {
          let (nx, ny) = (x + dx, y + dy);
          if nx >= 0
            && ny >= 0
            && nx < w
            && ny < h
            && !visited[ny as usize][nx as usize]
            && level.walkable(nx, ny)
          {
            visited[ny as usize][nx as usize] = true;
            queue.push_back((nx, ny));
          }
        }
      }
      if component.len() > best.len() {
        best = component;
      }
    }
  }
  best
}

#[cfg(test)]
mod tests {
  use super::*;

  fn glyph_for(tile: Tile) -> char {
    match tile {
      Tile::DeepWater => '~',
      Tile::ShallowWater => '≈',
      Tile::Sand => '.',
      Tile::Grass => ',',
      Tile::TallGrass => '"',
      Tile::Ground => ':',
      Tile::SmallRocks => 'o',
      Tile::CaveWall => '▓',
      Tile::CaveFloor => '·',
      Tile::ShipDock => 'D',
      _ => '?'
    }
  }

  #[test]
  fn renders_ascii() {
    let loc = generate_natural_planet(&NaturalParams {
      name: "Vera Spera",
      seed: SEED,
      breathable: true,
      tree_density: 0.6
    });

    // Count tiles on the surface so we can sanity-check the biome mix.
    let mut deep = 0;
    let mut shallow = 0;
    let mut sand = 0;
    let mut grass = 0;
    let mut tall_grass = 0;
    let mut cave_wall = 0;
    let mut small_rocks = 0;
    let mut ground = 0;
    let mut dock = 0;
    for y in 0..loc.height as i32 {
      for x in 0..loc.width as i32 {
        match loc.level(0).get(x, y).unwrap() {
          Tile::DeepWater => deep += 1,
          Tile::ShallowWater => shallow += 1,
          Tile::Sand => sand += 1,
          Tile::Grass => grass += 1,
          Tile::TallGrass => tall_grass += 1,
          Tile::CaveWall => cave_wall += 1,
          Tile::SmallRocks => small_rocks += 1,
          Tile::Ground => ground += 1,
          Tile::ShipDock => dock += 1,
          _ => {}
        }
      }
    }
    let total = (loc.width * loc.height) as i32;
    eprintln!(
      "Vera Spera ({}x{}):\n  deep water {:>4} ({:>4.1}%)\n  shallow   {:>4} ({:>4.1}%)\n  sand      {:>4} ({:>4.1}%)\n  grass     {:>4} ({:>4.1}%)\n  tall grass{:>4} ({:>4.1}%)\n  rock wall {:>4} ({:>4.1}%)\n  small rocks{:>4} ({:>4.1}%)\n  ground    {:>4} ({:>4.1}%)\n  dock      {:>4}",
      loc.width,
      loc.height,
      deep,
      100.0 * deep as f32 / total as f32,
      shallow,
      100.0 * shallow as f32 / total as f32,
      sand,
      100.0 * sand as f32 / total as f32,
      grass,
      100.0 * grass as f32 / total as f32,
      tall_grass,
      100.0 * tall_grass as f32 / total as f32,
      cave_wall,
      100.0 * cave_wall as f32 / total as f32,
      small_rocks,
      100.0 * small_rocks as f32 / total as f32,
      ground,
      100.0 * ground as f32 / total as f32,
      dock
    );

    // Sanity: the planet should have water, land, and the dock placed.
    assert!(deep + shallow > 0, "expected some water tiles");
    assert!(grass + tall_grass > 0, "expected some grass tiles");
    assert_eq!(dock, 1, "expected exactly one ship dock");

    // Count objects.
    let mut trees = 0;
    let mut cave_entrances = 0;
    let mut cave_exits = 0;
    let mut chests = 0;
    let mut creatures = 0;
    let mut rat_soldiers = 0;
    let mut armored_rats = 0;
    let mut alien_runners = 0;
    let mut mantis_aliens = 0;
    let mut mushroom_creatures = 0;
    let mut crab_aliens = 0;
    let mut robot_dogs = 0;
    for (_, _, _, obj) in &loc.spawn_objects {
      if Has::<Tree>::get(obj).is_some() {
        trees += 1;
      } else if Has::<LootChest>::get(obj).is_some() {
        chests += 1;
      } else if Has::<Elevator>::get(obj).is_some() {
        cave_entrances += 1;
      } else if Has::<Enemy>::get(obj).is_some() {
        creatures += 1;
        // Use the creature's display name to bucket the kind.
        if let Some(named) = Has::<Named>::get(obj) {
          match named.name {
            "Rat Soldier" => rat_soldiers += 1,
            "Armored Rat Soldier" => armored_rats += 1,
            "Xel-Naran Hunter" => alien_runners += 1,
            "Crystal Mantis" => mantis_aliens += 1,
            "Mycelid" => mushroom_creatures += 1,
            "Xel-Naran Crawler" => crab_aliens += 1,
            "Guard Dog" => robot_dogs += 1,
            _ => ()
          }
        }
      }
    }
    cave_exits = cave_entrances;
    eprintln!(
      "  trees {trees}  cave entrances {cave_entrances}  cave exits {cave_exits}  chests {chests}  creatures {creatures}"
    );
    eprintln!(
      "  creatures by type: rat {rat_soldiers}  armored {armored_rats}  alien_runner {alien_runners}  mantis {mantis_aliens}  mushroom {mushroom_creatures}  crab {crab_aliens}  robot_dog {robot_dogs}"
    );
    assert!(trees > 100, "expected hundreds of trees, got {trees}");
    assert!(cave_entrances >= 2, "expected cave elevators, got {cave_entrances}");
    assert!(chests > 0, "expected at least one chest, got {chests}");

    // Report how many features are within 30 tiles of the dock, so we know
    // the user will see something interesting on first spawn.
    let dock = {
      let mut found = (loc.width as i32 / 2, loc.height as i32 / 2);
      for y in 0..loc.height as i32 {
        for x in 0..loc.width as i32 {
          if loc.level(0).get(x, y) == Some(Tile::ShipDock) {
            found = (x, y);
          }
        }
      }
      found
    };
    let (dx, dy) = dock;
    let mut near_trees = 0;
    let mut near_cave_walls = 0;
    let mut near_small_rocks = 0;
    let mut near_creatures = 0;
    let mut near_ground = 0;
    for y in 0..loc.height as i32 {
      for x in 0..loc.width as i32 {
        let t = loc.level(0).get(x, y).unwrap();
        let dist = (x - dx).abs().max((y - dy).abs());
        if dist <= 30 {
          if matches!(t, Tile::CaveWall) {
            near_cave_walls += 1;
          }
          if matches!(t, Tile::SmallRocks) {
            near_small_rocks += 1;
          }
          if matches!(t, Tile::Ground) {
            near_ground += 1;
          }
        }
      }
    }
    for (lx, ly, _, obj) in &loc.spawn_objects {
      let dist = (*lx - dx).abs().max(*ly - dy).abs();
      if dist > 30 {
        continue;
      }
      if Has::<Tree>::get(obj).is_some() {
        near_trees += 1;
      } else if Has::<Enemy>::get(obj).is_some() {
        near_creatures += 1;
      }
    }
    eprintln!(
      "  within 30 tiles of dock @ ({dx},{dy}): trees {near_trees}  cave walls {near_cave_walls}  small rocks {near_small_rocks}  ground tiles {near_ground}  creatures {near_creatures}"
    );

    // Tree density histogram across X (10-tile bands). Reveals whether the
    // tree distribution has sharp boundaries or gradual transitions.
    // Each band shows (tree_count) followed by a 0-9 density bar.
    let mut trees_in_band = [0usize; 30];
    let mut grass_in_band = [0usize; 30];
    let mut ground_in_band = [0usize; 30];
    for (lx, ly, _, obj) in &loc.spawn_objects {
      if Has::<Tree>::get(obj).is_some() {
        let band = ((*lx).clamp(0, 299) / 10) as usize;
        if band < 30 {
          trees_in_band[band] += 1;
        }
      }
    }
    for y in 0..loc.height as i32 {
      for x in 0..loc.width as i32 {
        let t = loc.level(0).get(x, y);
        let band = (x.clamp(0, 299) / 10) as usize;
        if band < 30 {
          if t == Some(Tile::Grass) {
            grass_in_band[band] += 1;
          } else if t == Some(Tile::Ground) {
            ground_in_band[band] += 1;
          }
        }
      }
    }
    let mut line = String::from("  trees per 10-tile X band: ");
    for i in 0..30 {
      let pct = if grass_in_band[i] > 0 {
        (trees_in_band[i] as f32 / grass_in_band[i] as f32 * 100.0) as u32
      } else {
        0
      };
      let bar = ".".repeat(pct.min(40) as usize);
      line.push_str(&format!("{pct:>3}|{bar} "));
    }
    eprintln!("{line}");
    let mut line = String::from("  ground% per 10-tile X band: ");
    for i in 0..30 {
      let total = (grass_in_band[i] + ground_in_band[i]) as f32;
      let pct = if total > 0.0 {
        (ground_in_band[i] as f32 / total * 100.0) as u32
      } else {
        0
      };
      let bar = ".".repeat(pct.min(40) as usize);
      line.push_str(&format!("{pct:>3}|{bar} "));
    }
    eprintln!("{line}");

    // Render a downsampled ASCII view (60x60) so it fits in test output.
    let mut canvas = String::new();
    let level = loc.level(0);
    for y in (0..loc.height as i32).step_by(5) {
      for x in (0..loc.width as i32).step_by(5) {
        canvas.push(glyph_for(level.get(x, y).unwrap()));
      }
      canvas.push('\n');
    }
    eprintln!("\n{}\n", canvas);

    // Print a tiny cave slice so we can see the sublevel is real.
    let mut cave_slice = String::new();
    let cave = loc.level(1);
    for y in (0..60).step_by(2) {
      for x in (0..60).step_by(2) {
        cave_slice.push(glyph_for(cave.get(x, y).unwrap()));
      }
      cave_slice.push('\n');
    }
    eprintln!("cave slice (top-left 60x60):\n{}", cave_slice);
  }
}
