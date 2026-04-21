# Zone Infrastructure Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Refactor the game's data model to support a 10×10×4 zone grid, wire seamless zone transitions when the player walks off a zone edge, and update all simulation systems to ignore entities outside the current zone.

**Architecture:** Replace `World` with `ZoneWorld` (a 3D grid of `Level`s indexed `[zx][zy][z]`). Extend `Location::Coords` and `PlayerPos` with zone coordinates `zx, zy`. Zone transitions are handled in `player_input` by detecting off-edge movement, updating `PlayerPos`, and rebuilding tile visuals. All simulation systems filter entities by matching zone coords. The existing hand-crafted test world lives in zone (0, 0).

**Tech Stack:** Bevy 0.18, Rust 2024 edition. No new dependencies required.

---

## File Map

| File | What changes |
|------|-------------|
| `src/level.rs` | Add `ZoneWorld` struct + constants; add new `Tile` variants with all match arms |
| `src/entities.rs` | `Location::Coords` gains `zx, zy`; `Object::spawn_at` gains `zx, zy` |
| `src/main.rs` | `PlayerPos` gains `zx, zy, z`; remove `CurrentZ`; zone transition logic in `player_input`; update all systems |
| `src/combat.rs` | `enemy_ai` and `maintain_tile_index` filter by current zone |

---

## Task 1: Add new Tile variants

**Files:**
- Modify: `src/level.rs`

- [ ] **Step 1: Add variants to the Tile enum**

In `src/level.rs`, extend the `Tile` enum (currently ends at `Door`):

```rust
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tile {
  Air,
  Floor,
  Wall,
  CobblestoneWall,
  BrickWall,
  Grass,
  Water,
  Sand,
  StairsUp,
  StairsDown,
  Door,
  // --- new ---
  TallGrass,
  Bush,
  Ash,
  Lava,
  ShallowWater,
  DeepWater,
  Road,
  WoodWall,
  WoodFloor,
  Fence,
  CaveWall,
  CaveFloor,
  CrystalFormation,
}
```

- [ ] **Step 2: Add `glyph` arms**

Add to the `glyph` match in `impl Tile`:

```rust
Tile::TallGrass       => "\"",
Tile::Bush            => "%",
Tile::Ash             => ".",
Tile::Lava            => "~",
Tile::ShallowWater    => "~",
Tile::DeepWater       => "≈",
Tile::Road            => "·",
Tile::WoodWall        => "#",
Tile::WoodFloor       => ".",
Tile::Fence           => "+",
Tile::CaveWall        => "#",
Tile::CaveFloor       => ".",
Tile::CrystalFormation => "*",
```

- [ ] **Step 3: Add `color` arms**

```rust
Tile::TallGrass        => [0.25, 0.65, 0.25],
Tile::Bush             => [0.15, 0.45, 0.15],
Tile::Ash              => [0.55, 0.53, 0.5],
Tile::Lava             => [0.9,  0.3,  0.05],
Tile::ShallowWater     => [0.3,  0.5,  0.85],
Tile::DeepWater        => [0.1,  0.15, 0.6],
Tile::Road             => [0.45, 0.4,  0.35],
Tile::WoodWall         => [0.45, 0.3,  0.15],
Tile::WoodFloor        => [0.55, 0.4,  0.25],
Tile::Fence            => [0.5,  0.35, 0.2],
Tile::CaveWall         => [0.3,  0.28, 0.25],
Tile::CaveFloor        => [0.4,  0.38, 0.35],
Tile::CrystalFormation => [0.5,  0.8,  0.95],
```

- [ ] **Step 4: Add `walkable` arms**

In the `walkable` matches list, add these (non-walkable tiles are the default — only add the walkable ones):

```rust
Tile::TallGrass | Tile::Ash | Tile::Road | Tile::WoodFloor | Tile::CaveFloor | Tile::ShallowWater
```

The full updated `walkable` call becomes:
```rust
pub fn walkable(self) -> bool {
  matches!(
    self,
    Tile::Air
      | Tile::Floor
      | Tile::Grass
      | Tile::Sand
      | Tile::StairsUp
      | Tile::StairsDown
      | Tile::TallGrass
      | Tile::Ash
      | Tile::Road
      | Tile::WoodFloor
      | Tile::CaveFloor
      | Tile::ShallowWater
  )
}
```

- [ ] **Step 5: Add `opaque` arms**

```rust
pub fn opaque(self) -> bool {
  matches!(
    self,
    Tile::Wall
      | Tile::CobblestoneWall
      | Tile::BrickWall
      | Tile::WoodWall
      | Tile::CaveWall
      | Tile::Door
  )
}
```

- [ ] **Step 6: Add `name` arms**

```rust
Tile::TallGrass        => "Tall Grass",
Tile::Bush             => "Bush",
Tile::Ash              => "Ash",
Tile::Lava             => "Lava",
Tile::ShallowWater     => "Shallow Water",
Tile::DeepWater        => "Deep Water",
Tile::Road             => "Road",
Tile::WoodWall         => "Wooden Wall",
Tile::WoodFloor        => "Wooden Floor",
Tile::Fence            => "Fence",
Tile::CaveWall         => "Cave Wall",
Tile::CaveFloor        => "Cave Floor",
Tile::CrystalFormation => "Crystal Formation",
```

- [ ] **Step 7: Verify it compiles**

```bash
cargo check 2>&1 | head -30
```

Expected: no errors (all match arms exhaustive).

- [ ] **Step 8: Commit**

```bash
git add src/level.rs
git commit -m "feat: add new Tile variants (vegetation, cave, settlement, terrain)"
```

---

## Task 2: Add ZoneWorld and zone constants

**Files:**
- Modify: `src/level.rs`

- [ ] **Step 1: Add constants and ZoneWorld struct**

Add after the existing `World` struct definition (after the `impl World` block, before `build_test_world`):

```rust
// ---------------------------------------------------------------------------
// Zone world — 10×10×4 grid of 48×48 Levels
// ---------------------------------------------------------------------------

pub const ZONE_WIDTH:  usize = 48;
pub const ZONE_HEIGHT: usize = 48;
pub const WORLD_COLS:  usize = 10;
pub const WORLD_ROWS:  usize = 10;
pub const WORLD_DEPTH: usize = 4;

/// A 10×10×4 grid of zones.  zones[zx][zy][z] is one 48×48 Level.
/// Surface is z=3; underground levels are z=2, z=1, z=0.
pub struct ZoneWorld {
  pub zones: Vec<Vec<Vec<Level>>>,
}

impl ZoneWorld {
  /// Construct an empty ZoneWorld; every level is filled with `fill`.
  pub fn new(fill: Tile) -> Self {
    let zones = (0..WORLD_COLS)
      .map(|_| {
        (0..WORLD_ROWS)
          .map(|_| {
            (0..WORLD_DEPTH)
              .map(|_| Level::new(ZONE_WIDTH, ZONE_HEIGHT, fill))
              .collect()
          })
          .collect()
      })
      .collect();
    ZoneWorld { zones }
  }

  pub fn zone(&self, zx: usize, zy: usize, z: usize) -> &Level {
    &self.zones[zx][zy][z]
  }

  pub fn zone_mut(&mut self, zx: usize, zy: usize, z: usize) -> &mut Level {
    &mut self.zones[zx][zy][z]
  }

  pub fn in_bounds(&self, zx: i32, zy: i32) -> bool {
    zx >= 0 && zy >= 0
      && (zx as usize) < WORLD_COLS
      && (zy as usize) < WORLD_ROWS
  }
}
```

- [ ] **Step 2: Verify it compiles**

```bash
cargo check 2>&1 | head -30
```

Expected: no errors.

- [ ] **Step 3: Commit**

```bash
git add src/level.rs
git commit -m "feat: add ZoneWorld struct and zone constants (ZONE_WIDTH/HEIGHT, WORLD_COLS/ROWS/DEPTH)"
```

---

## Task 3: Extend Location::Coords and Object::spawn_at

**Files:**
- Modify: `src/entities.rs`

`Location::Coords` needs zone coordinates so entities know which zone they inhabit. `Object::spawn_at` is the primary spawn point and must be updated to accept them.

- [ ] **Step 1: Add `zx, zy` to `Location::Coords`**

In `src/entities.rs`, change:

```rust
pub enum Location {
  Coords { x: i32, y: i32, z: usize },
  Inventory(Entity),
  Nowhere
}
```

to:

```rust
pub enum Location {
  Coords { x: i32, y: i32, z: usize, zx: usize, zy: usize },
  Inventory(Entity),
  Nowhere
}
```

- [ ] **Step 2: Update `Object::spawn_at`**

Change:

```rust
pub fn spawn_at(self, commands: &mut Commands, x: i32, y: i32, z: usize) -> Entity {
  let mut e = commands.spawn_empty();
  (self.0)(&mut e);
  e.insert(Location::Coords { x, y, z });
  e.id()
}
```

to:

```rust
pub fn spawn_at(self, commands: &mut Commands, x: i32, y: i32, z: usize, zx: usize, zy: usize) -> Entity {
  let mut e = commands.spawn_empty();
  (self.0)(&mut e);
  e.insert(Location::Coords { x, y, z, zx, zy });
  e.id()
}
```

- [ ] **Step 3: Check what breaks**

```bash
cargo check 2>&1 | grep "^error"
```

Expected: several errors — every destructure of `Location::Coords` and every call to `spawn_at`. These are all fixed in the next tasks.

---

## Task 4: Update PlayerPos, remove CurrentZ, migrate GameWorld

**Files:**
- Modify: `src/main.rs`

This is the largest single task. It replaces `CurrentZ`, updates `PlayerPos`, changes `GameWorld` to hold a `ZoneWorld`, and fixes every usage site. Work through compile errors one by one.

- [ ] **Step 1: Update imports in `src/main.rs`**

Change the `use` block at the top from:

```rust
use {
  bevy::prelude::*,
  combat::{TileEntityIndex, enemy_ai, maintain_tile_index},
  level::{FovGrid, Tile, World, build_test_world, compute_fov},
  trl::entities::{Dialogue, DialogueTree, Enemy, Glyph, Gravity, Location, Named, Object, Stats, Wearing},
};
```

to:

```rust
use {
  bevy::prelude::*,
  combat::{TileEntityIndex, enemy_ai, maintain_tile_index},
  level::{
    FovGrid, Tile, ZoneWorld, ZONE_WIDTH, ZONE_HEIGHT,
    build_test_world, compute_fov,
  },
  trl::entities::{Dialogue, DialogueTree, Enemy, Glyph, Gravity, Location, Named, Object, Stats, Wearing},
};
```

- [ ] **Step 2: Update `PlayerPos`**

Change:

```rust
#[derive(Component)]
struct PlayerPos {
  x: i32,
  y: i32
}
```

to:

```rust
#[derive(Component)]
struct PlayerPos {
  x: i32,
  y: i32,
  zx: usize,
  zy: usize,
  z: usize,
}
```

- [ ] **Step 3: Update `GameWorld` and remove `CurrentZ`**

Change:

```rust
#[derive(Resource)]
struct GameWorld(World);

#[derive(Resource)]
struct CurrentZ(usize);
```

to:

```rust
#[derive(Resource)]
struct GameWorld(ZoneWorld);
```

Delete `CurrentZ` entirely.

- [ ] **Step 4: Update `Fov` initialization**

`FovGrid::new` is currently called with `world.width, world.height`. Change every `FovGrid::new(gw.0.width, gw.0.height)` to `FovGrid::new(ZONE_WIDTH, ZONE_HEIGHT)`.

- [ ] **Step 5: Update `tile_screen_pos` call sites**

The function signature stays the same. Every call currently passes `gw.0.width, gw.0.height`. Change all to pass `ZONE_WIDTH, ZONE_HEIGHT`.

- [ ] **Step 6: Update `screen_to_tile` call sites**

Same pattern — change `w: gw.0.width, h: gw.0.height` call sites to `ZONE_WIDTH, ZONE_HEIGHT`.

- [ ] **Step 7: Update the `App` builder in `main()`**

Change:

```rust
let world = build_test_world();
let start_z = 2;
let fov = FovGrid::new(world.width, world.height);

App::new()
  // ...
  .insert_resource(GameWorld(world))
  .insert_resource(CurrentZ(start_z))
  .insert_resource(Fov(fov))
```

to:

```rust
let world = build_test_world();
let fov = FovGrid::new(ZONE_WIDTH, ZONE_HEIGHT);

App::new()
  // ...
  .insert_resource(GameWorld(world))
  .insert_resource(Fov(fov))
```

Remove `.insert_resource(CurrentZ(start_z))`.

- [ ] **Step 8: Update `setup()`**

In `setup()`, the player is spawned with `Stats` and positioned. Change:

```rust
let level = gw.0.level(cz.0);
let (px, py) = find_walkable(level, 35, 29);
compute_fov(&mut fov.0, level, px, py, FOV_RADIUS);

commands.spawn((
  // ...
  Player,
  PlayerPos { x: px, y: py },
  // ...
));
```

to:

```rust
const START_ZX: usize = 0;
const START_ZY: usize = 0;
const START_Z:  usize = 2;

let level = gw.0.zone(START_ZX, START_ZY, START_Z);
let (px, py) = find_walkable(level, 35, 29);
compute_fov(&mut fov.0, level, px, py, FOV_RADIUS);

commands.spawn((
  // ...
  Player,
  PlayerPos { x: px, y: py, zx: START_ZX, zy: START_ZY, z: START_Z },
  // ...
));
```

Also fix the three `spawn_at` calls in `setup`:

```rust
Object::rat_soldier().spawn_at(&mut commands, ex1, ey1, cz.0);
Object::armored_rat_soldier().spawn_at(&mut commands, ex2, ey2, cz.0);
Object::catgirl()
  .add(Dialogue(&dialogue::MIRA))
  .spawn_at(&mut commands, cx1, cy1, cz.0);
```

becomes:

```rust
Object::rat_soldier().spawn_at(&mut commands, ex1, ey1, START_Z, START_ZX, START_ZY);
Object::armored_rat_soldier().spawn_at(&mut commands, ex2, ey2, START_Z, START_ZX, START_ZY);
Object::catgirl()
  .add(Dialogue(&dialogue::MIRA))
  .spawn_at(&mut commands, cx1, cy1, START_Z, START_ZX, START_ZY);
```

Also remove `cz: Res<CurrentZ>` from the `setup` parameters.

- [ ] **Step 9: Update `spawn_level_tiles`**

`spawn_level_tiles` currently takes `world: &World, z: usize`. Change to:

```rust
fn spawn_level_tiles(
  commands: &mut Commands,
  asset_server: &AssetServer,
  world: &ZoneWorld,
  zx: usize,
  zy: usize,
  z: usize,
) {
  let level = world.zone(zx, zy, z);
  for y in 0..level.height {
    for x in 0..level.width {
      let tile = level.tiles[y][x];
      if tile == Tile::Air { continue; }
      let pos = tile_screen_pos(x, y, ZONE_WIDTH, ZONE_HEIGHT);
      // ... rest unchanged, replace world.width/height with ZONE_WIDTH/ZONE_HEIGHT
    }
  }
}
```

Update `rebuild_level` to match:

```rust
fn rebuild_level(
  commands: &mut Commands,
  asset_server: &AssetServer,
  tile_query: &Query<Entity, With<TileGlyph>>,
  world: &ZoneWorld,
  zx: usize,
  zy: usize,
  z: usize,
) {
  despawn_level_tiles(commands, tile_query);
  spawn_level_tiles(commands, asset_server, world, zx, zy, z);
}
```

Update all call sites of `spawn_level_tiles` and `rebuild_level` in `setup` and elsewhere.

- [ ] **Step 10: Fix `update_entity_visibility`**

Remove `cz: Res<CurrentZ>` parameter, add a player query:

```rust
fn update_entity_visibility(
  player_q: Query<&PlayerPos, With<Player>>,
  mut entity_q: Query<(&Location, &mut Visibility), With<GlyphVisual>>,
) {
  let Ok(pos) = player_q.single() else { return };
  for (location, mut vis) in entity_q.iter_mut() {
    *vis = if let Location::Coords { z, zx, zy, .. } = location
      && *z == pos.z && *zx == pos.zx && *zy == pos.zy
    {
      Visibility::Visible
    } else {
      Visibility::Hidden
    };
  }
}
```

- [ ] **Step 11: Fix `apply_gravity`**

Remove `cz: ResMut<CurrentZ>`. Use `player_q: Query<&mut PlayerPos, With<Player>>` instead.

For non-player entities, filter to current zone by comparing `Location::Coords { zx, zy, .. }` against the player's zone. Update location z only for entities in the current zone:

```rust
fn apply_gravity(
  gw: Res<GameWorld>,
  asset_server: Res<AssetServer>,
  mut fov: ResMut<Fov>,
  mut commands: Commands,
  tile_query: Query<Entity, With<TileGlyph>>,
  mut player_q: Query<&mut PlayerPos, With<Player>>,
  mut entity_q: Query<&mut Location, (With<Gravity>, Without<Player>)>,
) {
  let Ok(mut pos) = player_q.single_mut() else { return };

  // Non-player gravity: only simulate current zone
  for mut location in entity_q.iter_mut() {
    if let Location::Coords { x, y, z, zx, zy } = *location
      && zx == pos.zx && zy == pos.zy
      && z > 0
      && should_fall(&gw.0.zone(zx, zy, z), x, y)
    {
      *location = Location::Coords { x, y, z: z - 1, zx, zy };
    }
  }

  // Player gravity
  let z = pos.z;
  let (zx, zy) = (pos.zx, pos.zy);
  if z > 0 && should_fall(gw.0.zone(zx, zy, z), pos.x, pos.y) {
    pos.z -= 1;
    rebuild_level(&mut commands, &asset_server, &tile_query, &gw.0, zx, zy, pos.z);
    fov.0 = FovGrid::new(ZONE_WIDTH, ZONE_HEIGHT);
    compute_fov(&mut fov.0, gw.0.zone(zx, zy, pos.z), pos.x, pos.y, FOV_RADIUS);
  }
}
```

Also update `should_fall` to take `&ZoneWorld` and the full zone coords, preserving the original two-condition check:

```rust
fn should_fall(gw: &ZoneWorld, zx: usize, zy: usize, x: i32, y: i32, z: usize) -> bool {
  let here = gw.zone(zx, zy, z).tiles[y as usize][x as usize];
  let below = z.checked_sub(1)
    .map(|z1| gw.zone(zx, zy, z1).tiles[y as usize][x as usize]);
  here.causes_falling() || below.is_some_and(|t| t.causes_falling())
}
```

Update the two call sites in `apply_gravity`:
- Non-player: `should_fall(&gw.0, zx, zy, x, y, z)`
- Player: `should_fall(&gw.0, pos.zx, pos.zy, pos.x, pos.y, pos.z)`

- [ ] **Step 12: Fix `update_time_mode`**

Remove `cz: Res<CurrentZ>`. Get z and zone from the player query:

```rust
fn update_time_mode(
  mut clock: ResMut<GameClock>,
  player_q: Query<&PlayerPos, With<Player>>,
  enemy_q: Query<&Location, With<Enemy>>,
) {
  let enemy_near = player_q.single().is_ok_and(|pos| {
    enemy_q.iter().any(|loc| {
      let Location::Coords { x, y, z, zx, zy } = *loc else { return false };
      zx == pos.zx && zy == pos.zy && z == pos.z
        && (x - pos.x).abs() <= ENEMY_ALERT_RADIUS
        && (y - pos.y).abs() <= ENEMY_ALERT_RADIUS
    })
  });
  clock.mode = if enemy_near { TimeMode::TurnBased } else { TimeMode::RealTime };
}
```

- [ ] **Step 13: Fix `update_hud`**

`update_hud` uses `cz: Res<CurrentZ>` for the level display. Replace with a player query:

```rust
fn update_hud(
  clock: Res<GameClock>,
  player_q: Query<&PlayerPos, With<Player>>,
  mut time_q: Query<(&mut Text2d, &mut TextColor), (With<TimeDisplay>, Without<LevelDisplay>)>,
  mut level_q: Query<&mut Text2d, (With<LevelDisplay>, Without<TimeDisplay>)>,
) {
  // time display unchanged ...

  if let Ok(mut text) = level_q.single_mut()
    && let Ok(pos) = player_q.single()
  {
    let label = match pos.z {
      0 => "Deep Cave (z=0)",
      1 => "Shallow Cave (z=1)",
      2 => "Surface (z=2)",
      3 => "Building Upper (z=3)",
      z => return *text = Text2d::new(format!("z={z}"))
    };
    *text = Text2d::new(format!("{} [{},{}]", label, pos.zx, pos.zy));
  }
}
```

- [ ] **Step 14: Fix `mouse_hover_tile`**

Replace `cz: Res<CurrentZ>` with a player query. Replace `&cz` arg in `tile_hover_text` call with the player pos:

```rust
fn mouse_hover_tile(
  windows: Query<&Window>,
  camera_q: Query<(&Camera, &GlobalTransform), With<Camera2d>>,
  gw: Res<GameWorld>,
  player_q: Query<&PlayerPos, With<Player>>,
  fov: Res<Fov>,
  index: Res<TileEntityIndex>,
  named_q: Query<(&Named, Option<&Stats>)>,
  mut info_q: Query<&mut Text2d, With<TileInfoDisplay>>,
) {
  if let Ok(mut info_text) = info_q.single_mut()
    && let Ok(window) = windows.single()
    && let Ok((camera, cam_transform)) = camera_q.single()
    && let Ok(pos) = player_q.single()
  {
    *info_text = Text2d::new(
      tile_hover_text(window, camera, cam_transform, &gw, pos, &fov, &index, &named_q)
        .unwrap_or_default()
    );
  }
}
```

In `tile_hover_text`, replace `cz: &CurrentZ` parameter with `player_pos: &PlayerPos` and use `player_pos.zx, player_pos.zy, player_pos.z` where `cz.0` was used.

- [ ] **Step 15: Fix `handle_menus` and `execute_interaction`**

`handle_menus` has `cz: ResMut<CurrentZ>`. Replace with extracting z from the player query when needed. In `execute_interaction`, `cz` is used for level lookups and modified on ascend/descend. Since `PlayerPos` now owns z, update these functions to modify `pos.z` directly and pass zone coords into `rebuild_level`.

`handle_menus` signature change — remove `cz: ResMut<CurrentZ>`:
```rust
fn handle_menus(
  // remove: mut cz: ResMut<CurrentZ>,
  // keep everything else
)
```

`execute_interaction` change — remove `cz` parameter, use `pos.z` for current z, and `pos.zx, pos.zy` for zone:

```rust
fn execute_interaction(
  action: &InteractionAction,
  gw: &mut ResMut<GameWorld>,
  clock: &mut ResMut<GameClock>,
  fov: &mut ResMut<Fov>,
  dialogue_state: &mut ResMut<DialogueState>,
  commands: &mut Commands,
  asset_server: &AssetServer,
  tile_query: &Query<Entity, With<TileGlyph>>,
  player_query: &mut Query<(&mut PlayerPos, &mut Transform), With<Player>>,
) {
  if let Ok((mut pos, mut transform)) = player_query.single_mut() {
    match action {
      InteractionAction::Ascend => {
        if pos.z + 1 < WORLD_DEPTH {
          pos.z += 1;
          rebuild_level(commands, asset_server, tile_query, &gw.0, pos.zx, pos.zy, pos.z);
          fov.0 = FovGrid::new(ZONE_WIDTH, ZONE_HEIGHT);
          compute_fov(&mut fov.0, gw.0.zone(pos.zx, pos.zy, pos.z), pos.x, pos.y, FOV_RADIUS);
          transform.translation =
            tile_screen_pos(pos.x as usize, pos.y as usize, ZONE_WIDTH, ZONE_HEIGHT) + Vec3::Z;
          clock.advance(PlayerAction::Ascend.time_cost());
        }
      }
      InteractionAction::Descend => {
        if pos.z > 0 {
          pos.z -= 1;
          rebuild_level(commands, asset_server, tile_query, &gw.0, pos.zx, pos.zy, pos.z);
          fov.0 = FovGrid::new(ZONE_WIDTH, ZONE_HEIGHT);
          compute_fov(&mut fov.0, gw.0.zone(pos.zx, pos.zy, pos.z), pos.x, pos.y, FOV_RADIUS);
          transform.translation =
            tile_screen_pos(pos.x as usize, pos.y as usize, ZONE_WIDTH, ZONE_HEIGHT) + Vec3::Z;
          clock.advance(PlayerAction::Descend.time_cost());
        }
      }
      InteractionAction::OpenDoor(dx, dy) => {
        gw.0.zone_mut(pos.zx, pos.zy, pos.z).set(*dx, *dy, Tile::Floor);
        rebuild_level(commands, asset_server, tile_query, &gw.0, pos.zx, pos.zy, pos.z);
        compute_fov(&mut fov.0, gw.0.zone(pos.zx, pos.zy, pos.z), pos.x, pos.y, FOV_RADIUS);
        clock.advance(1.0);
      }
      InteractionAction::Talk { speaker, tree } => {
        **dialogue_state = DialogueState::Open { speaker, tree, node_name: tree.nodes[0].name };
      }
    }
  }
}
```

- [ ] **Step 16: Fix `gather_interactions`**

`gather_interactions` takes `level: &level::Level, px, py, z, depth`. The `depth` parameter was for checking z < depth on stairs. Replace with WORLD_DEPTH:

```rust
fn gather_interactions(level: &level::Level, px: i32, py: i32, z: usize) -> Vec<InteractionOption> {
  // replace `z + 1 < depth` with `z + 1 < WORLD_DEPTH`
  // replace `z > 0` check stays the same
}
```

Update the call site in `player_input`:

```rust
let level = gw.0.zone(pos.zx, pos.zy, pos.z);
let mut options = gather_interactions(level, pos.x, pos.y, pos.z);
```

- [ ] **Step 17: Fix `player_input`**

Remove `cz: Res<CurrentZ>`. The function gets zone info from `pos: &mut PlayerPos`. Update all `cz.0` to `pos.z`, `level` lookup to `gw.0.zone(pos.zx, pos.zy, pos.z)`, and `tile_screen_pos` calls to use `ZONE_WIDTH, ZONE_HEIGHT`.

- [ ] **Step 18: Fix `update_fov_visuals`**

Remove `cz: Res<CurrentZ>`. Get the zone from player query:

```rust
fn update_fov_visuals(
  fov: Res<Fov>,
  gw: Res<GameWorld>,
  player_q: Query<&PlayerPos, With<Player>>,
  mut glyph_tiles: Query<(&TileGlyph, &mut TextColor), Without<TilePng>>,
  mut sprite_tiles: Query<(&TileGlyph, &mut Sprite), With<TilePng>>,
) {
  let Ok(pos) = player_q.single() else { return };
  let level = gw.0.zone(pos.zx, pos.zy, pos.z);
  // rest unchanged
}
```

- [ ] **Step 19: Build to verify**

```bash
cargo build 2>&1 | grep "^error"
```

Expected: no errors. If errors remain, fix them — every error at this point is a missed `cz` reference or a missed `Location::Coords` destructure that needs `..` or the new fields.

- [ ] **Step 20: Update `setup` call to `spawn_level_tiles`**

In `setup`, find `spawn_level_tiles(&mut commands, &asset_server, &gw.0, cz.0)` and update to `spawn_level_tiles(&mut commands, &asset_server, &gw.0, START_ZX, START_ZY, START_Z)`.

---

## Task 5: Update `build_test_world` to return `ZoneWorld`

**Files:**
- Modify: `src/level.rs`

The test world content stays the same but is placed in zone (0, 0).

- [ ] **Step 1: Rewrite `build_test_world`**

Replace the function signature and body opener:

```rust
pub fn build_test_world() -> ZoneWorld {
  let mut world = ZoneWorld::new(Tile::Air);
  const ZX: usize = 0;
  const ZY: usize = 0;
```

Then replace every `world.level_mut(z)` with `world.zone_mut(ZX, ZY, z)`.

Replace the `place_stairs` calls — the function currently takes `&mut [Level]`. Update to pass a slice of the zone column:

```rust
// Old: place_stairs(&mut world.levels, z_from, z_to, x, y);
// New: pass a slice of the specific zone column
place_stairs(world.zones[ZX][ZY].as_mut_slice(), z_from, z_to, x, y);
```

Replace `clear_around` calls:

```rust
// Old: clear_around(world.level_mut(z), x, y, r);
// New:
clear_around(world.zone_mut(ZX, ZY, z), x, y, r);
```

End the function with `world` instead of `world`.

Full replacement (keeping existing room/corridor placement logic, just updating method calls):

```rust
pub fn build_test_world() -> ZoneWorld {
  const W: usize = ZONE_WIDTH;
  const H: usize = ZONE_HEIGHT;
  const ZX: usize = 0;
  const ZY: usize = 0;

  let mut world = ZoneWorld::new(Tile::Air);

  // === z=2: surface ===
  {
    let s = world.zone_mut(ZX, ZY, 2);
    fill_rect(s, 0, 0, W, H, Tile::Grass);
    fill_rect(s, 10, 20, 28, 3, Tile::Sand);
    fill_rect(s, 23, 8, 3, 15, Tile::Sand);
    place_room_with_door(s, 18, 6, 12, 10, Side::South, 6, Tile::BrickWall);
    fill_rect(s, 24, 7, 1, 5, Tile::BrickWall);
    fill_rect(s, 3, 28, 8, 6, Tile::Air);
    fill_rect(s, 32, 28, 7, 5, Tile::Water);
    for &(tx, ty) in &[(8, 8), (11, 6), (15, 10), (28, 14), (38, 8), (42, 16)] {
      s.set(tx, ty, Tile::Wall);
    }
    for &(tx, ty, item) in &[
      (10, 22, Item::GoldCoin),
      (16, 8, Item::Rock),
      (30, 16, Item::Torch),
      (44, 28, Item::Mushroom),
    ] {
      s.set_item(tx, ty, Some(item));
    }
  }

  // === z=3: building upper floor ===
  {
    let u = world.zone_mut(ZX, ZY, 3);
    place_room_with_door(u, 18, 6, 12, 10, Side::South, 6, Tile::BrickWall);
    fill_rect(u, 23, 7, 1, 4, Tile::BrickWall);
    u.set(23, 10, Tile::Door);
  }

  place_stairs(world.zones[ZX][ZY].as_mut_slice(), 2, 3, 28, 8);

  // === z=1: shallow cave ===
  {
    let c = world.zone_mut(ZX, ZY, 1);
    fill_rect(c, 0, 0, W, H, Tile::CobblestoneWall);
    carve_blob(c, 20, 24, 12, Tile::Floor);
    carve_blob(c, 6, 32, 7, Tile::Floor);
    carve_blob(c, 38, 16, 7, Tile::Floor);
    place_wide_corridor(c, 13, 32, 20, 24);
    place_wide_corridor(c, 32, 24, 38, 16);
    carve_blob(c, 16, 18, 4, Tile::Water);
    carve_blob(c, 28, 28, 3, Tile::Sand);
    for &(tx, ty, item) in &[
      (16, 22, Item::GoldCoin),
      (16, 23, Item::GoldCoin),
      (34, 14, Item::HealthPotion),
      (22, 26, Item::Torch),
      (8, 30, Item::Mushroom),
    ] {
      c.set_item(tx, ty, Some(item));
    }
  }

  place_stairs(world.zones[ZX][ZY].as_mut_slice(), 1, 2, 6, 32);
  clear_around(world.zone_mut(ZX, ZY, 1), 6, 32, 2);
  clear_around(world.zone_mut(ZX, ZY, 2), 6, 32, 2);

  // === z=0: deep cave ===
  {
    let d = world.zone_mut(ZX, ZY, 0);
    fill_rect(d, 0, 0, W, H, Tile::CobblestoneWall);
    carve_blob(d, 24, 24, 14, Tile::Floor);
    carve_blob(d, 12, 12, 9, Tile::Floor);
    carve_blob(d, 40, 20, 6, Tile::Floor);
    place_wide_corridor(d, 18, 16, 24, 24);
    place_wide_corridor(d, 38, 24, 40, 20);
    carve_blob(d, 28, 32, 5, Tile::Water);
    carve_blob(d, 10, 34, 4, Tile::Sand);
    for &(tx, ty, item) in &[
      (20, 22, Item::GoldCoin),
      (21, 22, Item::GoldCoin),
      (20, 23, Item::GoldCoin),
      (11, 10, Item::HealthPotion),
      (38, 18, Item::Torch),
      (26, 28, Item::Rock),
      (9, 33, Item::Mushroom),
      (42, 22, Item::GoldCoin),
    ] {
      d.set_item(tx, ty, Some(item));
    }
  }

  place_stairs(world.zones[ZX][ZY].as_mut_slice(), 0, 1, 20, 24);
  clear_around(world.zone_mut(ZX, ZY, 0), 20, 24, 2);
  clear_around(world.zone_mut(ZX, ZY, 1), 20, 24, 2);

  world
}
```

Note: room positions are adjusted to fit within 48×48 (the old test world was 80×60). The essential structure (surface with building, two cave levels, four z-levels) is preserved.

- [ ] **Step 2: Build to verify**

```bash
cargo build 2>&1 | grep "^error"
```

Expected: no errors.

- [ ] **Step 3: Commit**

```bash
git add src/level.rs src/entities.rs src/main.rs
git commit -m "refactor: replace World with ZoneWorld; PlayerPos gains zx/zy/z; remove CurrentZ"
```

---

## Task 6: Update `combat.rs` for zone-aware simulation

**Files:**
- Modify: `src/combat.rs`

- [ ] **Step 1: Update `maintain_tile_index` to filter by current zone**

Add a player query parameter and filter:

```rust
pub fn maintain_tile_index(
  mut index: ResMut<TileEntityIndex>,
  query: Query<(Entity, &Location)>,
  player_q: Query<&crate::PlayerPos, With<crate::Player>>,
) {
  index.0.clear();
  let Ok(pos) = player_q.single() else { return };
  for (entity, location) in query.iter() {
    if let Location::Coords { x, y, z, zx, zy } = location
      && *zx == pos.zx && *zy == pos.zy
    {
      index.0.entry((*x, *y, *z)).or_default().push(entity);
    }
  }
}
```

- [ ] **Step 2: Update `enemy_ai` to remove `CurrentZ`, use `PlayerPos`**

Remove `cz: Res<crate::CurrentZ>`. Get zone coords from player query:

```rust
pub fn enemy_ai(
  time: Res<Time>,
  index: Res<TileEntityIndex>,
  gw: Res<crate::GameWorld>,
  mut player_q: Query<(&crate::PlayerPos, &mut Stats), (With<crate::Player>, Without<Enemy>)>,
  mut enemy_q: Query<
    (&mut Location, &mut TimeSinceAction, &Stats, Option<&Wearing>),
    (With<Enemy>, Without<crate::Player>),
  >,
) {
  if let Ok((player_pos, mut player_stats)) = player_q.single_mut() {
    let (px, py) = (player_pos.x, player_pos.y);
    let level = gw.0.zone(player_pos.zx, player_pos.zy, player_pos.z);
    let dt = time.delta_secs();
    let mut claimed: std::collections::HashSet<(i32, i32)> = std::collections::HashSet::new();

    for (mut location, mut timer, enemy_stats, enemy_wearing) in enemy_q.iter_mut() {
      timer.0 += dt;

      if let Location::Coords { x: ex, y: ey, z: ez, zx: ezx, zy: ezy } = *location
        && ezx == player_pos.zx && ezy == player_pos.zy && ez == player_pos.z
      {
        let dist = (px - ex).abs().max((py - ey).abs());

        if dist == 1 && timer.0 >= 1.0 / enemy_stats.attack_speed {
          let dmg = resolve_damage(enemy_stats.attack, enemy_wearing);
          player_stats.hp = (player_stats.hp - dmg).max(0);
          if player_stats.hp == 0 {
            bevy::log::info!("You died.");
          }
          timer.0 = 0.0;
        } else if timer.0 >= 1.0 / enemy_stats.move_speed {
          let (dx, dy) = step_toward(ex, ey, px, py);
          let (nx, ny) = (ex + dx, ey + dy);
          if level.walkable(nx, ny)
            && !index.0.contains_key(&(nx, ny, ez))
            && !claimed.contains(&(nx, ny))
          {
            let nz = if level.tiles[ny as usize][nx as usize].causes_falling() && ez > 0 {
              ez - 1
            } else {
              ez
            };
            *location = Location::Coords { x: nx, y: ny, z: nz, zx: ezx, zy: ezy };
            claimed.insert((nx, ny));
            timer.0 = 0.0;
          }
        }
      }
    }
  }
}
```

- [ ] **Step 3: Build and run**

```bash
cargo build 2>&1 | grep "^error"
```

Expected: no errors.

```bash
cargo run
```

Expected: game launches, player starts on the surface at zone (0,0), all existing behavior (movement, combat, dialogue, FOV) works. HUD shows `Surface (z=2) [0,0]`.

- [ ] **Step 4: Commit**

```bash
git add src/combat.rs
git commit -m "refactor: enemy_ai and tile index filter by current zone (zx, zy)"
```

---

## Task 7: Zone transition on edge walk

**Files:**
- Modify: `src/main.rs`

When the player walks off a zone edge (tile x < 0, x ≥ ZONE_WIDTH, y < 0, y ≥ ZONE_HEIGHT), transition to the adjacent zone if it's in bounds.

- [ ] **Step 1: Add a `try_zone_transition` helper**

Add this function to `src/main.rs`:

```rust
/// If `pos` is out of the current zone's tile bounds, attempt a zone transition.
/// Returns true if a transition occurred (caller should skip normal move handling).
fn try_zone_transition(
  pos: &mut PlayerPos,
  transform: &mut Transform,
  gw: &ZoneWorld,
  fov: &mut FovGrid,
  commands: &mut Commands,
  asset_server: &AssetServer,
  tile_query: &Query<Entity, With<TileGlyph>>,
) -> bool {
  let (mut new_zx, mut new_zy) = (pos.zx as i32, pos.zy as i32);
  let (mut new_x, mut new_y) = (pos.x, pos.y);

  if pos.x < 0 {
    new_zx -= 1;
    new_x = ZONE_WIDTH as i32 - 1;
  } else if pos.x >= ZONE_WIDTH as i32 {
    new_zx += 1;
    new_x = 0;
  }

  if pos.y < 0 {
    new_zy -= 1;
    new_y = ZONE_HEIGHT as i32 - 1;
  } else if pos.y >= ZONE_HEIGHT as i32 {
    new_zy += 1;
    new_y = 0;
  }

  // No transition if still within current zone
  if new_zx == pos.zx as i32 && new_zy == pos.zy as i32 {
    return false;
  }

  // Refuse if destination zone is out of world bounds
  if !gw.in_bounds(new_zx, new_zy) {
    // Clamp position back into current zone
    pos.x = pos.x.clamp(0, ZONE_WIDTH as i32 - 1);
    pos.y = pos.y.clamp(0, ZONE_HEIGHT as i32 - 1);
    return true;
  }

  // Perform transition
  pos.zx = new_zx as usize;
  pos.zy = new_zy as usize;
  pos.x = new_x;
  pos.y = new_y;

  rebuild_level(commands, asset_server, tile_query, gw, pos.zx, pos.zy, pos.z);
  *fov = FovGrid::new(ZONE_WIDTH, ZONE_HEIGHT);
  compute_fov(fov, gw.zone(pos.zx, pos.zy, pos.z), pos.x, pos.y, FOV_RADIUS);
  transform.translation =
    tile_screen_pos(pos.x as usize, pos.y as usize, ZONE_WIDTH, ZONE_HEIGHT) + Vec3::Z;

  true
}
```

- [ ] **Step 2: Call it in `player_input` after updating position**

In `player_input`, find the block where `pos.x` and `pos.y` are updated after a successful move:

```rust
if bumped.is_none() {
  pos.x += dx;
  pos.y += dy;
  transform.translation = ...
  compute_fov(...)
}
```

Replace with:

```rust
if bumped.is_none() {
  pos.x += dx;
  pos.y += dy;
  let transitioned = try_zone_transition(
    &mut pos, &mut transform, &gw.0, &mut fov.0,
    &mut commands, &asset_server, &tile_query,
  );
  if !transitioned {
    transform.translation =
      tile_screen_pos(pos.x as usize, pos.y as usize, ZONE_WIDTH, ZONE_HEIGHT) + Vec3::Z;
    compute_fov(&mut fov.0, gw.0.zone(pos.zx, pos.zy, pos.z), pos.x, pos.y, FOV_RADIUS);
  }
}
```

Add `tile_query: Query<Entity, With<TileGlyph>>` and `asset_server: Res<AssetServer>` to `player_input`'s parameter list. The full updated signature becomes:

```rust
fn player_input(
  keys: Res<ButtonInput<KeyCode>>,
  time: Res<Time>,
  asset_server: Res<AssetServer>,          // new
  gw: Res<GameWorld>,
  pause: Res<PauseMenu>,
  dialogue_state: Res<DialogueState>,
  mut menu: ResMut<InteractMenu>,
  mut clock: ResMut<GameClock>,
  mut cooldown: ResMut<MoveCooldown>,
  mut fov: ResMut<Fov>,
  index: Res<TileEntityIndex>,
  mut commands: Commands,
  tile_query: Query<Entity, With<TileGlyph>>,  // new
  mut player_query: Query<(&mut PlayerPos, &mut Transform, &Stats), With<Player>>,
  mut enemy_query: Query<(&mut Stats, Option<&Wearing>), (With<Enemy>, Without<Player>)>,
  dialogue_q: Query<(&Named, &Dialogue)>,
)
```

- [ ] **Step 3: Run and test zone transitions manually**

```bash
cargo run
```

Walk the player to the right edge of the zone (tile x=47), then one step further.
Expected: tiles despawn and respawn (adjacent zone loads — currently Air, so mostly empty), HUD zone coords update to `[1,0]`. Walk back left to edge, cross back — HUD returns to `[0,0]` and test content reappears.

Walk to the world edge (zone 0,0 going left/up). Expected: player is stopped, does not cross.

- [ ] **Step 4: Commit**

```bash
git add src/main.rs
git commit -m "feat: zone transition — walk off zone edge loads adjacent zone"
```

---

## Task 8: Final build verification

- [ ] **Step 1: Full build and test**

```bash
cargo test
```

Expected: all existing tests in `combat.rs` pass.

```bash
cargo run
```

Manual checks:
- Player starts in zone (0,0), z=2 (surface), in the test room
- Movement, FOV, combat, dialogue all work
- Walking off the east edge transitions to zone (1,0) — currently empty Air/wall
- Walking back west returns to zone (0,0) with test content
- Ascending/descending stairs changes z within zone (0,0)
- HUD shows zone coords
- Enemy AI only acts when in the same zone as the player

- [ ] **Step 2: Final commit**

```bash
git add -p  # stage any remaining changes
git commit -m "feat: zone infrastructure complete — ZoneWorld, zone transitions, simulation filtering"
```

---

## Notes for Plan B

Plan B (world generation) will:
- Add the `noise` crate dependency
- Replace `build_test_world` with a `generate_world(seed: u64) -> ZoneWorld` function
- Implement island mask, continuous noise tile assignment, cave generation, town layout procgen
- Add vegetation entities (Tree, Boulder) and the NPC layer (`world_data.rs`)
- Implement the world map image overlay
