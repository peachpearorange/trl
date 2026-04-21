# Island World Generation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the hand-crafted test world with a procedurally generated island surrounded by ocean, including underground caves, towns, and a world map overlay toggled with `M`.

**Architecture:** A new `src/worldgen.rs` module (no Bevy dependencies) provides `generate_world(seed: u64) -> ZoneWorld`. Surface tiles come from masked Fbm/Perlin noise; underground from 2D noise offset by z-level; towns from flat-area scanning + road/building placement. `main.rs` calls `generate_world` at startup and gains a world map sprite overlay. A separate `src/world_data.rs` holds the manual NPC injection layer.

**Tech Stack:** Rust, `noise = "0.9"` (Fbm<Perlin>), Bevy 0.18 (Image/Sprite for world map). No other new dependencies.

---

## File Map

| File | Role |
|------|------|
| `Cargo.toml` | Add `noise = "0.9"` |
| `src/worldgen.rs` | New: all generation logic — island mask, tile assignment, caves, towns, stairs |
| `src/world_data.rs` | New: `NpcPlacement` struct + `world_npcs()` fn returning manual NPC list |
| `src/main.rs` | Add `mod worldgen; mod world_data;`; replace `build_test_world()` call; update start zone to island center; remove hardcoded enemy/NPC spawns; add world map overlay (Task 7) |

---

## Task 1: Add noise dependency and worldgen scaffold

**Files:**
- Modify: `Cargo.toml`
- Create: `src/worldgen.rs`
- Modify: `src/main.rs`

- [ ] **Step 1: Add the noise crate**

In `Cargo.toml`, add to `[dependencies]`:

```toml
noise = "0.9"
```

- [ ] **Step 2: Create `src/worldgen.rs` with imports and a stub**

```rust
use noise::{Fbm, NoiseFn, Perlin};

use crate::level::{
  fill_rect, Level, Tile, ZoneWorld, WORLD_COLS, WORLD_DEPTH, WORLD_ROWS, ZONE_HEIGHT, ZONE_WIDTH,
};

pub const WORLD_SEED: u64 = 42;

/// Generate a full ZoneWorld from a deterministic seed.
pub fn generate_world(seed: u64) -> ZoneWorld {
  ZoneWorld::new(Tile::Air)
}
```

- [ ] **Step 3: Add `mod worldgen;` and `mod world_data;` to `main.rs`**

Near the top of `src/main.rs`, after the existing `mod level; mod combat; mod dialogue;` line, add:

```rust
mod worldgen;
mod world_data;
```

- [ ] **Step 4: Create stub `src/world_data.rs`**

```rust
use trl::entities::Object;

/// A named NPC to be spawned at a specific world position after generation.
pub struct NpcPlacement {
  pub wx: i32,
  pub wy: i32,
  pub z:  usize,
  pub object: fn() -> Object,
}

/// Hand-authored NPCs injected into the world after procgen.
/// Example (commented out until a suitable world position is confirmed):
/// ```
/// NpcPlacement { wx: 247, wy: 243, z: 2, object: Object::catgirl },
/// ```
pub fn world_npcs() -> Vec<NpcPlacement> {
  vec![]
}
```

- [ ] **Step 5: Verify it compiles**

```bash
cargo check 2>&1 | grep "^error"
```

Expected: no errors.

- [ ] **Step 6: Commit**

```bash
git add Cargo.toml src/worldgen.rs src/world_data.rs src/main.rs
git commit -m "feat: add worldgen scaffold and noise dependency"
```

---

## Task 2: Island mask and tile-from-value (TDD)

**Files:**
- Modify: `src/worldgen.rs`

These two functions are pure — no noise, no world state — so they're ideal TDD starting points.

- [ ] **Step 1: Write the failing tests**

Add to the bottom of `src/worldgen.rs`:

```rust
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
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cargo test 2>&1 | grep -E "^(test |error)"
```

Expected: compilation errors (`island_mask` and `tile_from_value` not defined).

- [ ] **Step 3: Implement `island_mask`**

Add to `src/worldgen.rs` (before the test module):

```rust
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
```

- [ ] **Step 4: Implement `tile_from_value`**

```rust
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
```

- [ ] **Step 5: Run tests to verify they pass**

```bash
cargo test worldgen 2>&1 | grep -E "^(test |FAILED|ok)"
```

Expected: all 7 tests pass.

- [ ] **Step 6: Commit**

```bash
git add src/worldgen.rs
git commit -m "feat: island_mask and tile_from_value with passing tests"
```

---

## Task 3: Surface generation

**Files:**
- Modify: `src/worldgen.rs`

- [ ] **Step 1: Write integration tests for surface generation**

Add to the `tests` module in `src/worldgen.rs`:

```rust
  // --- surface generation ---

  #[test]
  fn world_has_correct_zone_dimensions() {
    let world = generate_world(WORLD_SEED);
    assert_eq!(world.zones.len(), WORLD_COLS);
    assert_eq!(world.zones[0].len(), WORLD_ROWS);
    assert_eq!(world.zones[0][0].len(), WORLD_DEPTH);
  }

  #[test]
  fn island_center_is_mostly_land() {
    let world = generate_world(WORLD_SEED);
    let czx = WORLD_COLS / 2;
    let czy = WORLD_ROWS / 2;
    let zone = world.zone(czx, czy, 2);
    let land_count = zone.tiles.iter().flatten()
      .filter(|&&t| matches!(t, Tile::Grass | Tile::TallGrass | Tile::Sand | Tile::Ash | Tile::Bush | Tile::Lava))
      .count();
    let total = ZONE_WIDTH * ZONE_HEIGHT;
    assert!(
      land_count > total / 2,
      "center zone should be >50% land, got {land_count}/{total}"
    );
  }

  #[test]
  fn world_edge_is_mostly_deep_water() {
    let world = generate_world(WORLD_SEED);
    // Check corner zone (0, 0) at z=2
    let zone = world.zone(0, 0, 2);
    let water_count = zone.tiles.iter().flatten()
      .filter(|&&t| t == Tile::DeepWater)
      .count();
    let total = ZONE_WIDTH * ZONE_HEIGHT;
    assert!(
      water_count > total * 3 / 4,
      "corner zone should be >75% DeepWater, got {water_count}/{total}"
    );
  }
```

- [ ] **Step 2: Run to confirm they fail**

```bash
cargo test worldgen::tests::world_has_correct 2>&1 | tail -5
```

Expected: `FAILED` (generate_world still returns empty ZoneWorld).

- [ ] **Step 3: Implement `surface_tile_value` and surface generation in `generate_world`**

Add above the test module:

```rust
/// Sample the noise at world position (wx, wy) and apply the island mask.
/// Returns a value in [0, 1] suitable for `tile_from_value`.
fn surface_tile_value(wx: usize, wy: usize, noise: &Fbm<Perlin>) -> f64 {
  const SCALE: f64 = 110.0; // tile-scale of the noise features
  let raw = noise.get([wx as f64 / SCALE, wy as f64 / SCALE]); // ≈ [-1, 1]
  let normalized = (raw + 1.0) / 2.0; // [0, 1]
  normalized * island_mask(wx, wy)
}
```

Replace `generate_world` with:

```rust
pub fn generate_world(seed: u64) -> ZoneWorld {
  let surface_noise: Fbm<Perlin> = Fbm::new(seed as u32);
  let cave_noise:    Fbm<Perlin> = Fbm::new(seed.wrapping_add(1) as u32);

  let mut world = ZoneWorld::new(Tile::Air);

  // === z=2: surface ===
  for zx in 0..WORLD_COLS {
    for zy in 0..WORLD_ROWS {
      let zone = world.zone_mut(zx, zy, 2);
      for ty in 0..ZONE_HEIGHT {
        for tx in 0..ZONE_WIDTH {
          let wx = zx * ZONE_WIDTH + tx;
          let wy = zy * ZONE_HEIGHT + ty;
          zone.tiles[ty][tx] = tile_from_value(surface_tile_value(wx, wy, &surface_noise));
        }
      }
    }
  }

  world
}
```

- [ ] **Step 4: Run the surface tests**

```bash
cargo test worldgen 2>&1 | grep -E "^(test |FAILED|ok)"
```

Expected: `world_has_correct_zone_dimensions`, `island_center_is_mostly_land`, `world_edge_is_mostly_deep_water` all pass. The earlier mask/tile tests still pass too.

- [ ] **Step 5: Commit**

```bash
git add src/worldgen.rs
git commit -m "feat: surface noise generation with island mask"
```

---

## Task 4: Underground cave generation

**Files:**
- Modify: `src/worldgen.rs`

Caves use the same `Fbm<Perlin>` as the surface but sampled at a finer scale and offset along y to produce different shapes per z-level.

- [ ] **Step 1: Write the failing test**

Add to the `tests` module:

```rust
  #[test]
  fn underground_has_cave_floors() {
    let world = generate_world(WORLD_SEED);
    // Check the center region has a mix of CaveFloor and CaveWall at z=1
    let czx = WORLD_COLS / 2;
    let czy = WORLD_ROWS / 2;
    let zone = world.zone(czx, czy, 1);
    let floor_count = zone.tiles.iter().flatten()
      .filter(|&&t| t == Tile::CaveFloor)
      .count();
    let total = ZONE_WIDTH * ZONE_HEIGHT;
    assert!(
      floor_count > total / 4,
      "underground zone should be >25% CaveFloor, got {floor_count}/{total}"
    );
    assert!(
      floor_count < total * 3 / 4,
      "underground zone should be <75% CaveFloor (not fully open), got {floor_count}/{total}"
    );
  }
```

- [ ] **Step 2: Run to confirm it fails**

```bash
cargo test worldgen::tests::underground 2>&1 | tail -5
```

Expected: `FAILED` (z=0 and z=1 are still Air).

- [ ] **Step 3: Implement `underground_tile`**

Add above the test module:

```rust
/// Produce a cave tile for underground levels (z=0, z=1).
/// Uses a 2D noise slice; different z values get a large y-offset so cave
/// shapes differ per level while remaining deterministic and continuous
/// across zone boundaries.
fn underground_tile(wx: usize, wy: usize, z: usize, noise: &Fbm<Perlin>) -> Tile {
  const SCALE: f64 = 16.0;
  // Large prime-ish offset separates z-levels in noise space
  let z_offset = z as f64 * 137.3;
  let v = noise.get([wx as f64 / SCALE, wy as f64 / SCALE + z_offset]);
  // v ≈ [-1, 1]; threshold near 0 gives roughly 50% open caves
  if v > -0.1 { Tile::CaveFloor } else { Tile::CaveWall }
}
```

- [ ] **Step 4: Add underground generation to `generate_world`**

Inside `generate_world`, after the surface generation block, add:

```rust
  // === z=0, z=1: underground caves ===
  for zx in 0..WORLD_COLS {
    for zy in 0..WORLD_ROWS {
      for z in 0..2usize {
        let zone = world.zone_mut(zx, zy, z);
        for ty in 0..ZONE_HEIGHT {
          for tx in 0..ZONE_WIDTH {
            let wx = zx * ZONE_WIDTH + tx;
            let wy = zy * ZONE_HEIGHT + ty;
            zone.tiles[ty][tx] = underground_tile(wx, wy, z, &cave_noise);
          }
        }
      }
    }
  }
```

- [ ] **Step 5: Run tests**

```bash
cargo test worldgen 2>&1 | grep -E "^(test |FAILED|ok)"
```

Expected: all tests pass including the new underground test.

- [ ] **Step 6: Commit**

```bash
git add src/worldgen.rs
git commit -m "feat: underground cave generation via 2D noise offset"
```

---

## Task 5: Town placement

**Files:**
- Modify: `src/worldgen.rs`

Towns are placed on flat grassland areas. Each town has a road cross and up to four WoodWall buildings.

- [ ] **Step 1: Write the failing tests**

Add to the `tests` module:

```rust
  #[test]
  fn find_town_sites_returns_some_sites() {
    let world = generate_world(WORLD_SEED);
    let sites = find_town_sites(&world);
    assert!(
      !sites.is_empty(),
      "should find at least one suitable town site in the island"
    );
  }

  #[test]
  fn place_town_puts_road_tiles() {
    let mut world = generate_world(WORLD_SEED);
    let sites = find_town_sites(&world);
    if let Some(&(cx, cy)) = sites.first() {
      place_town(&mut world, cx, cy);
      // At least one Road tile should exist near the town center
      let (zx, zy) = (cx / ZONE_WIDTH, cy / ZONE_HEIGHT);
      let (lx, ly) = (cx % ZONE_WIDTH, cy % ZONE_HEIGHT);
      let zone = world.zone(zx, zy, 2);
      let road_count = zone.tiles.iter().flatten()
        .filter(|&&t| t == Tile::Road)
        .count();
      assert!(road_count > 0, "town should have road tiles");
    }
  }
```

- [ ] **Step 2: Run to confirm they fail**

```bash
cargo test worldgen::tests::find_town 2>&1 | tail -5
cargo test worldgen::tests::place_town 2>&1 | tail -5
```

Expected: `FAILED` (functions not defined).

- [ ] **Step 3: Implement `set_world_tile` helper**

```rust
/// Write a tile at world coordinates (wx, wy, z). Silently ignores out-of-bounds positions.
fn set_world_tile(world: &mut ZoneWorld, wx: i32, wy: i32, z: usize, tile: Tile) {
  if wx < 0 || wy < 0 { return; }
  let (wx, wy) = (wx as usize, wy as usize);
  let zx = wx / ZONE_WIDTH;
  let zy = wy / ZONE_HEIGHT;
  if zx >= WORLD_COLS || zy >= WORLD_ROWS { return; }
  world.zone_mut(zx, zy, z).tiles[wy % ZONE_HEIGHT][wx % ZONE_WIDTH] = tile;
}
```

- [ ] **Step 4: Implement `town_suitability` and `find_town_sites`**

```rust
const TOWN_SEARCH_STEP: usize = 30;
const TOWN_CHECK_RADIUS: usize = 10;
const MIN_TOWN_DIST_SQ: usize = 80 * 80;
const TARGET_TOWNS: usize = 4;

/// Returns the fraction of tiles in a (2r+1)×(2r+1) area around (cx, cy) that
/// are suitable for settlement: Grass, TallGrass, or Sand.
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
      let tile = world.zone(zx, zy, 2).tiles[wy % ZONE_HEIGHT][wx % ZONE_WIDTH];
      total += 1;
      if matches!(tile, Tile::Grass | Tile::TallGrass | Tile::Sand) {
        suitable += 1;
      }
    }
  }
  if total == 0 { 0.0 } else { suitable as f32 / total as f32 }
}

/// Scan the world surface and return up to TARGET_TOWNS world-space center
/// positions suitable for a town (flat land, well-separated from each other).
pub fn find_town_sites(world: &ZoneWorld) -> Vec<(usize, usize)> {
  let world_w = WORLD_COLS * ZONE_WIDTH;
  let world_h = WORLD_ROWS * ZONE_HEIGHT;
  let margin = TOWN_CHECK_RADIUS + 5;

  // Gather all candidates above the suitability threshold
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

  // Best score first
  candidates.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));

  // Greedy spacing: keep sites that are far enough from already-chosen ones
  let mut sites: Vec<(usize, usize)> = Vec::new();
  for (cx, cy, _) in candidates {
    let too_close = sites.iter().any(|&(sx, sy)| {
      let dx = cx as i64 - sx as i64;
      let dy = cy as i64 - sy as i64;
      (dx * dx + dy * dy) as usize < MIN_TOWN_DIST_SQ
    });
    if !too_close {
      sites.push((cx, cy));
      if sites.len() >= TARGET_TOWNS { break; }
    }
  }

  sites
}
```

- [ ] **Step 5: Implement `place_building` and `place_town`**

```rust
/// Place a WoodWall-bordered building of size (w × h) with its top-left at (wx, wy).
/// Interior tiles are WoodFloor. A Door is cut into the south wall at the midpoint.
/// Does not overwrite water or lava tiles.
fn place_building(world: &mut ZoneWorld, wx: i32, wy: i32, w: i32, h: i32) {
  for dy in 0..h {
    for dx in 0..w {
      let tile = if dx == 0 || dx == w - 1 || dy == 0 || dy == h - 1 {
        Tile::WoodWall
      } else {
        Tile::WoodFloor
      };
      // Don't overwrite ocean or lava — town is only placed on land
      let ex = wx + dx;
      let ey = wy + dy;
      if ex >= 0 && ey >= 0 {
        let (uex, uey) = (ex as usize, ey as usize);
        let zx = uex / ZONE_WIDTH;
        let zy = uey / ZONE_HEIGHT;
        if zx < WORLD_COLS && zy < WORLD_ROWS {
          let existing = world.zone(zx, zy, 2).tiles[uey % ZONE_HEIGHT][uex % ZONE_WIDTH];
          if !matches!(existing, Tile::DeepWater | Tile::ShallowWater | Tile::Lava) {
            set_world_tile(world, ex, ey, 2, tile);
          }
        }
      }
    }
  }
  // Door in south wall, midpoint
  set_world_tile(world, wx + w / 2, wy + h - 1, 2, Tile::Door);
}

/// Place a town centred at world position (cx, cy):
///   - A horizontal and vertical Road cross
///   - Up to four WoodWall buildings, one per quadrant
pub fn place_town(world: &mut ZoneWorld, cx: usize, cy: usize) {
  const ROAD_HALF: i32 = 12;
  const BLDG_W: i32 = 7;
  const BLDG_H: i32 = 6;

  let (cx, cy) = (cx as i32, cy as i32);

  // Horizontal road
  for dx in -ROAD_HALF..=ROAD_HALF {
    set_world_tile(world, cx + dx, cy, 2, Tile::Road);
  }
  // Vertical road
  for dy in -ROAD_HALF..=ROAD_HALF {
    set_world_tile(world, cx, cy + dy, 2, Tile::Road);
  }

  // Buildings in each quadrant, offset from road edge
  let quadrants: [(i32, i32); 4] = [
    (2,  2),              // SE quadrant
    (-(BLDG_W + 2), 2),   // SW quadrant
    (2,  -(BLDG_H + 2)),  // NE quadrant
    (-(BLDG_W + 2), -(BLDG_H + 2)), // NW quadrant
  ];

  for (ox, oy) in quadrants {
    place_building(world, cx + ox, cy + oy, BLDG_W, BLDG_H);
  }
}
```

- [ ] **Step 6: Wire towns into `generate_world`**

At the end of `generate_world`, before `world` is returned, add:

```rust
  // === Towns ===
  let sites = find_town_sites(&world);
  for (cx, cy) in sites {
    place_town(&mut world, cx, cy);
  }
```

- [ ] **Step 7: Run tests**

```bash
cargo test worldgen 2>&1 | grep -E "^(test |FAILED|ok)"
```

Expected: all tests pass.

- [ ] **Step 8: Commit**

```bash
git add src/worldgen.rs
git commit -m "feat: town placement — flat-area scan, road cross, WoodWall buildings"
```

---

## Task 6: Stairs placement and wire into main

**Files:**
- Modify: `src/worldgen.rs`
- Modify: `src/main.rs`

Stairs connect the surface to the cave and the cave to the deep cave. Then `main.rs` switches from `build_test_world` to `worldgen::generate_world`.

- [ ] **Step 1: Write the failing test for stairs**

Add to the `tests` module:

```rust
  #[test]
  fn world_has_surface_to_cave_stairs() {
    let world = generate_world(WORLD_SEED);
    // There should be at least one StairsDown on the surface (z=2)
    let stair_count: usize = (0..WORLD_COLS)
      .flat_map(|zx| (0..WORLD_ROWS).map(move |zy| (zx, zy)))
      .map(|(zx, zy)| {
        world.zone(zx, zy, 2).tiles.iter().flatten()
          .filter(|&&t| t == Tile::StairsDown)
          .count()
      })
      .sum();
    assert!(stair_count > 0, "world should have at least one StairsDown on the surface");
  }
```

- [ ] **Step 2: Run to confirm it fails**

```bash
cargo test worldgen::tests::world_has_surface 2>&1 | tail -5
```

Expected: `FAILED`.

- [ ] **Step 3: Implement `place_world_stairs`**

```rust
/// Scan zone (0,0) outward for a tile pair where z=2 surface is walkable land
/// and z=1 cave is CaveFloor. Place StairsDown/StairsUp there.
/// Repeat for z=1 → z=0 using a nearby CaveFloor pair.
pub fn place_world_stairs(world: &mut ZoneWorld) {
  // Surface → shallow cave (z=2 down, z=1 up)
  'outer: for ty in 5..ZONE_HEIGHT - 5 {
    for tx in 5..ZONE_WIDTH - 5 {
      let surf = world.zone(0, 0, 2).tiles[ty][tx];
      let cave1 = world.zone(0, 0, 1).tiles[ty][tx];
      if matches!(surf, Tile::Grass | Tile::Sand | Tile::TallGrass)
        && cave1 == Tile::CaveFloor
      {
        world.zone_mut(0, 0, 2).tiles[ty][tx] = Tile::StairsDown;
        world.zone_mut(0, 0, 1).tiles[ty][tx] = Tile::StairsUp;
        break 'outer;
      }
    }
  }

  // Shallow cave → deep cave (z=1 down, z=0 up)
  // Search the whole zone (0,0) since cave floors aren't at predictable locations
  'outer2: for ty in 0..ZONE_HEIGHT {
    for tx in 0..ZONE_WIDTH {
      let cave1 = world.zone(0, 0, 1).tiles[ty][tx];
      let cave0 = world.zone(0, 0, 0).tiles[ty][tx];
      if cave1 == Tile::CaveFloor && cave0 == Tile::CaveFloor {
        world.zone_mut(0, 0, 1).tiles[ty][tx] = Tile::StairsDown;
        world.zone_mut(0, 0, 0).tiles[ty][tx] = Tile::StairsUp;
        break 'outer2;
      }
    }
  }
}
```

- [ ] **Step 4: Wire `place_world_stairs` into `generate_world`**

After the town placement block, before `world` is returned:

```rust
  // === Stairs ===
  place_world_stairs(&mut world);
```

- [ ] **Step 5: Run the stairs test**

```bash
cargo test worldgen 2>&1 | grep -E "^(test |FAILED|ok)"
```

Expected: all tests pass.

- [ ] **Step 6: Update `main.rs` to use `generate_world`**

In `src/main.rs`:

**(a)** Remove the import of `build_test_world` from the `use level::` block:

```rust
// Before:
use level::{FovGrid, Tile, ZoneWorld, ZONE_WIDTH, ZONE_HEIGHT, WORLD_DEPTH, build_test_world, compute_fov};

// After:
use level::{FovGrid, Tile, ZoneWorld, ZONE_WIDTH, ZONE_HEIGHT, WORLD_DEPTH, compute_fov};
```

**(b)** In `main()`, replace `build_test_world()` with `worldgen::generate_world`:

```rust
// Before:
let world = build_test_world();

// After:
let world = worldgen::generate_world(worldgen::WORLD_SEED);
```

**(c)** In `setup()`, change the start zone to the island center and remove the hardcoded enemy/NPC spawns. The full updated start-position block (replace from `const START_ZX` down to just before the HUD spawns):

```rust
  // Start near the island center (zone 4,4 is roughly the island's middle ring)
  const START_ZX: usize = 4;
  const START_ZY: usize = 4;
  const START_Z:  usize = 2;

  let cam_entity = commands.spawn(Camera2d).id();

  spawn_level_tiles(&mut commands, &asset_server, &gw.0, START_ZX, START_ZY, START_Z);

  let level = gw.0.zone(START_ZX, START_ZY, START_Z);
  let (lx, ly) = find_walkable(level, ZONE_WIDTH / 2, ZONE_HEIGHT / 2);
  let (px, py) = (
    (START_ZX * ZONE_WIDTH) as i32 + lx as i32,
    (START_ZY * ZONE_HEIGHT) as i32 + ly as i32,
  );
  compute_fov(&mut fov.0, level, lx as i32, ly as i32, FOV_RADIUS);

  commands.spawn((
    Text2d::new("@"),
    TextFont { font_size: TILE_SIZE, ..default() },
    TextColor(Color::srgb(1.0, 1.0, 0.0)),
    Transform::from_translation(
      tile_screen_pos(lx, ly, ZONE_WIDTH, ZONE_HEIGHT) + Vec3::Z
    ),
    Player,
    PlayerPos { x: px, y: py, z: START_Z },
    Stats { hp: 20, max_hp: 20, attack: 5, move_speed: 3.0, attack_speed: 1.0 },
  ));
```

(The three `Object::rat_soldier`, `Object::armored_rat_soldier`, `Object::catgirl` spawns are removed. They will be re-added via `world_data.rs` in a future task.)

- [ ] **Step 7: Spawn world_data NPCs after generation**

At the end of `setup`, before the HUD spawns, add:

```rust
  // Spawn hand-authored NPCs from the world data layer
  for npc in world_data::world_npcs() {
    (npc.object)().spawn_at(&mut commands, npc.wx, npc.wy, npc.z);
  }
```

- [ ] **Step 8: Build and run**

```bash
cargo build 2>&1 | grep "^error"
```

Expected: no errors.

```bash
cargo run
```

Expected: the game launches showing a noise-generated island — grass/sand near the center zone, ocean tiles at the world edges (visible when walking to zone boundaries). The player starts in zone [4,4] at z=2.

- [ ] **Step 9: Commit**

```bash
git add src/worldgen.rs src/main.rs
git commit -m "feat: wire generate_world into main, start player on island center"
```

---

## Task 7: World map overlay (M key)

**Files:**
- Modify: `src/main.rs`

Press `M` to open a fullscreen sprite showing the entire 480×480 world as a colour-coded tile image. Press `M` or `Escape` to close.

- [ ] **Step 1: Add `WorldMapState` resource and marker**

In `src/main.rs`, add alongside the other resource/component definitions:

```rust
#[derive(Resource, Default, PartialEq, Eq)]
enum WorldMapState {
  #[default]
  Closed,
  Open,
}

#[derive(Component)]
struct WorldMapOverlay;
```

- [ ] **Step 2: Add `WorldMapImage` resource**

```rust
#[derive(Resource)]
struct WorldMapImage(Handle<Image>);
```

- [ ] **Step 3: Implement `generate_world_map_image`**

Add this free function (no Bevy systems, just pure image building):

```rust
fn generate_world_map_image(world: &ZoneWorld, images: &mut Assets<Image>) -> Handle<Image> {
  use bevy::render::{
    render_asset::RenderAssetUsages,
    render_resource::{Extent3d, TextureDimension, TextureFormat},
  };

  let w = WORLD_COLS * ZONE_WIDTH;
  let h = WORLD_ROWS * ZONE_HEIGHT;
  let mut data = vec![0u8; w * h * 4]; // RGBA8

  for zy in 0..WORLD_ROWS {
    for zx in 0..WORLD_COLS {
      let zone = world.zone(zx, zy, 2);
      for ty in 0..ZONE_HEIGHT {
        for tx in 0..ZONE_WIDTH {
          let wx = zx * ZONE_WIDTH + tx;
          let wy = zy * ZONE_HEIGHT + ty;
          let [r, g, b] = zone.tiles[ty][tx].color();
          let idx = (wy * w + wx) * 4;
          data[idx]     = (r * 255.0) as u8;
          data[idx + 1] = (g * 255.0) as u8;
          data[idx + 2] = (b * 255.0) as u8;
          data[idx + 3] = 255;
        }
      }
    }
  }

  let image = Image::new(
    Extent3d { width: w as u32, height: h as u32, depth_or_array_layers: 1 },
    TextureDimension::D2,
    data,
    TextureFormat::Rgba8UnormSrgb,
    RenderAssetUsages::RENDER_WORLD,
  );
  images.add(image)
}
```

- [ ] **Step 4: Generate the image in `setup` and insert as resource**

Add `mut images: ResMut<Assets<Image>>` to the `setup` parameters. At the end of `setup` (before returning), add:

```rust
  let map_handle = generate_world_map_image(&gw.0, &mut images);
  commands.insert_resource(WorldMapImage(map_handle));
```

- [ ] **Step 5: Register resources in `main()`**

In the `App` builder, add:

```rust
.insert_resource(WorldMapState::default())
```

- [ ] **Step 6: Add the world map system to the Update chain**

In the `add_systems(Update, (...).chain())` call, add `handle_world_map` to the chain, just before `handle_menus`:

```rust
handle_world_map,
handle_menus,
```

- [ ] **Step 7: Implement `handle_world_map`**

```rust
fn handle_world_map(
  keys: Res<ButtonInput<KeyCode>>,
  mut state: ResMut<WorldMapState>,
  map_image: Res<WorldMapImage>,
  mut commands: Commands,
  overlay_q: Query<Entity, With<WorldMapOverlay>>,
  pause: Res<PauseMenu>,
  dialogue_state: Res<DialogueState>,
) {
  // Don't open world map while another overlay is active
  if *pause != PauseMenu::Closed || matches!(*dialogue_state, DialogueState::Open { .. }) {
    return;
  }

  let m_pressed = keys.just_pressed(KeyCode::KeyM);
  let esc_pressed = keys.just_pressed(KeyCode::Escape);

  match *state {
    WorldMapState::Closed => {
      if m_pressed {
        *state = WorldMapState::Open;
        // Spawn fullscreen map sprite at high z so it covers everything
        let scale = Vec2::new(
          (WORLD_COLS * ZONE_WIDTH) as f32,
          (WORLD_ROWS * ZONE_HEIGHT) as f32,
        );
        commands.spawn((
          Sprite {
            image: map_image.0.clone(),
            custom_size: Some(scale),
            ..default()
          },
          Transform::from_xyz(0.0, 0.0, 50.0),
          WorldMapOverlay,
        ));
      }
    }
    WorldMapState::Open => {
      if m_pressed || esc_pressed {
        *state = WorldMapState::Closed;
        for e in overlay_q.iter() {
          commands.entity(e).despawn();
        }
      }
    }
  }
}
```

- [ ] **Step 8: Add `M` key to the controls screen**

In `spawn_pause_overlay`, update the controls text string to include:

```rust
"M               world map\n\
```

(Insert this line after the existing controls.)

- [ ] **Step 9: Build and run**

```bash
cargo build 2>&1 | grep "^error"
```

Expected: no errors.

```bash
cargo run
```

Expected: press `M` in-game — a colour-coded top-down world map fills the screen (blue ocean around a green/tan/brown island). Press `M` or `Esc` to close it. Camera position has no effect on the map (it's in world-space at z=50, but the camera should centre on it — if the view is offset, see the note below).

> **Note:** The `WorldMapOverlay` sprite is placed at `(0, 0, 50)` in world space. Since the camera follows the player and the map sprite is not a child of the camera, the map will only appear centred when the player is near world origin. Fix: either make the sprite a child of the camera entity, or use a separate camera for the UI. For MVP this is acceptable; a future polish task can attach the map sprite to the camera as a child with appropriate z ordering.

- [ ] **Step 10: Commit**

```bash
git add src/main.rs
git commit -m "feat: world map overlay — press M to view full island map"
```

---

## Self-Review

**Spec coverage check:**

| Spec requirement | Task |
|-----------------|------|
| Continuous Perlin/simplex noise at world-space coords | Task 3 (`surface_tile_value`) |
| Island mask — radial falloff from center | Task 2 (`island_mask`) |
| Surface tile assignment by noise threshold | Task 2 (`tile_from_value`) |
| Underground caves across full world space | Task 4 (`underground_tile`) |
| Town placement — flat areas, road grid, buildings | Task 5 |
| Manual NPC injection layer (`world_data.rs`) | Task 1 + Task 6 (stub) |
| World map image overlay | Task 7 |
| Vegetation entities (Tree, Boulder) | ⚠ Not in this plan — deferred. Spec lists them as a stretch; the tile types exist; entity spawning from worldgen is a follow-up. |
| A second noise octave for local variation | ⚠ `Fbm` already uses multiple internal octaves. Explicit second-pass variation (boulders, cave entrances) is deferred. |
| 3–5 towns | Task 5 (`TARGET_TOWNS = 4`) ✓ |

**Placeholder scan:** None found.

**Type consistency:**
- `generate_world(seed: u64) -> ZoneWorld` — called in Task 6 Step 6 ✓
- `find_town_sites(world: &ZoneWorld) -> Vec<(usize, usize)>` — called in Task 5 Step 6 ✓
- `place_town(world: &mut ZoneWorld, cx: usize, cy: usize)` — called in Task 5 Step 6 ✓
- `place_world_stairs(world: &mut ZoneWorld)` — called in Task 6 Step 4 ✓
- `WorldMapImage`, `WorldMapState`, `WorldMapOverlay` — all defined in Task 7 before use ✓
