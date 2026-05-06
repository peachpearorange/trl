# Space Game Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Pivot the game to a space setting with a player-owned spaceship that teleports between variable-size locations in a 3D galaxy.

**Architecture:** New `Galaxy` + `Ship` types alongside existing `ZoneWorld`. An `ActiveZone` resource holds the currently rendered merged tile grid (ship + docked destination, or just ship in deep space). Existing rendering, FOV, combat, dialogue, and crafting systems are reused. New systems for docking/undocking, navigation/transit, atmosphere/vacuum damage, and crew are added.

**Tech Stack:** Rust, Bevy 0.18.1, noise crate (existing), haalka (existing)

---

## File Structure

| File | Purpose |
|------|---------|
| `src/galaxy.rs` | `Galaxy`, `Location`, `LocationType`, `LocationId`, coordinate helpers |
| `src/ship.rs` | `Ship` resource, ship layout builder |
| `src/active_zone.rs` | `ActiveZone` resource, merge/dock/undock logic |
| `src/navigation.rs` | Navigation UI overlay, flight console interaction, `TransitState` |
| `src/atmosphere.rs` | Vacuum damage system, `EVASuit` component |
| `src/crew.rs` | `CrewMember` component, recruitment integration, shipboard AI |
| `src/galaxy_gen.rs` | Space procgen: asteroids, stations, derelicts, planets, ruins |
| `src/level.rs` (modify) | New space `Tile` variants, atmosphere helper |
| `src/main.rs` (modify) | Space entry point, startup systems |

---

## Phase 1: Foundation Types

### Task 1: Add space Tile variants to level.rs

**Files:**
- Modify: `src/level.rs`

- [ ] **Step 1: Add space tile variants to the Tile enum**

Append these variants to the existing `Tile` enum in `src/level.rs` (do not remove any existing variants):

```rust
// --- Space tiles ---
DeckPlate,
Bulkhead,
Window,
AirlockDoor,
StationFloor,
StationWall,
DerelictFloor,
DerelictWall,
Conduit,
AsteroidRock,
AsteroidFloor,
Regolith,
Vacuum,
IceFloor,
IceWall,
AlienSoil,
AlienGrass,
CrystalGrowth,
AlienFluid,
```

- [ ] **Step 2: Add match arms for new tiles on all Tile methods**

Extend `Tile::glyph()`:
```rust
DeckPlate | StationFloor | DerelictFloor | AsteroidFloor | WoodFloor | CaveFloor => ".",
Bulkhead | StationWall | DerelictWall | AsteroidRock | IceWall => "#",
Window => "o",
AirlockDoor => "+",
Conduit => "=",
Regolith | IceFloor | AlienSoil => ",",
Vacuum => " ",
AlienGrass => "\"",
CrystalGrowth => "*",
AlienFluid => "~",
```

Extend `Tile::color()`:
```rust
DeckPlate | StationFloor => [0.55, 0.58, 0.62],
Bulkhead => [0.45, 0.47, 0.50],
Window => [0.2, 0.25, 0.7],
AirlockDoor => [0.7, 0.65, 0.3],
StationWall => [0.5, 0.52, 0.55],
DerelictFloor => [0.35, 0.33, 0.3],
DerelictWall => [0.3, 0.28, 0.25],
Conduit => [0.6, 0.55, 0.2],
AsteroidRock => [0.4, 0.35, 0.3],
AsteroidFloor => [0.5, 0.45, 0.4],
Regolith => [0.55, 0.5, 0.45],
Vacuum => [0.0, 0.0, 0.0],
IceFloor => [0.7, 0.75, 0.85],
IceWall => [0.5, 0.55, 0.7],
AlienSoil => [0.45, 0.35, 0.55],
AlienGrass => [0.3, 0.55, 0.3],
CrystalGrowth => [0.5, 0.8, 0.95],
AlienFluid => [0.5, 0.3, 0.7],
```

Extend `Tile::walkable()`:
```rust
DeckPlate | StationFloor | DerelictFloor | AsteroidFloor | Regolith
| IceFloor | AlienSoil | AlienGrass | Conduit => true,
Bulkhead | StationWall | DerelictWall | Window | AsteroidRock | IceWall
| CrystalGrowth => false,
AirlockDoor => false,
Vacuum | AlienFluid => true,
```

Extend `Tile::opaque()`:
```rust
Bulkhead | StationWall | DerelictWall | AsteroidRock | IceWall => true,
Window => false,
AirlockDoor | DeckPlate | StationFloor | DerelictFloor | AsteroidFloor
| Regolith | Vacuum | IceFloor | AlienSoil | AlienGrass | CrystalGrowth
| AlienFluid | Conduit => false,
```

Extend `Tile::minimap_color()`:
```rust
DeckPlate | StationFloor => [0.45, 0.47, 0.5],
Bulkhead | StationWall => [0.35, 0.37, 0.4],
AsteroidRock | AsteroidFloor => [0.42, 0.38, 0.33],
Regolith | IceFloor => [0.6, 0.62, 0.68],
AlienSoil | AlienGrass => [0.35, 0.45, 0.3],
AlienFluid => [0.4, 0.25, 0.6],
Vacuum => [0.02, 0.03, 0.06],
DerelictFloor | DerelictWall => [0.28, 0.26, 0.22],
IceWall => [0.45, 0.5, 0.62],
Conduit => [0.5, 0.45, 0.15],
AirlockDoor => [0.6, 0.55, 0.2],
Window => [0.15, 0.2, 0.55],
CrystalGrowth => [0.4, 0.65, 0.8],
```

Extend `Tile::name()`:
```rust
DeckPlate => "Deck Plate",
Bulkhead => "Bulkhead",
Window => "Window",
AirlockDoor => "Airlock Door",
StationFloor => "Station Floor",
StationWall => "Station Wall",
DerelictFloor => "Derelict Floor",
DerelictWall => "Derelict Wall",
Conduit => "Conduit",
AsteroidRock => "Asteroid Rock",
AsteroidFloor => "Asteroid Floor",
Regolith => "Regolith",
Vacuum => "Vacuum",
IceFloor => "Ice Floor",
IceWall => "Ice Wall",
AlienSoil => "Alien Soil",
AlienGrass => "Alien Grass",
CrystalGrowth => "Crystal Growth",
AlienFluid => "Alien Fluid",
```

- [ ] **Step 3: Add `has_atmosphere` method to Tile**

```rust
impl Tile {
    /// Whether this tile provides breathable atmosphere.
    /// Always returns false for Vacuum; true for interior tiles.
    pub fn has_atmosphere(self) -> bool {
        !matches!(self, Tile::Vacuum | Tile::Air)
    }
}
```

- [ ] **Step 4: Verify compilation**

Run: `cargo check 2>&1`
Expected: Compilation succeeds with no errors.

- [ ] **Step 5: Commit**

```bash
git add src/level.rs
git commit -m "feat: add space tile variants to Tile enum"
```

### Task 2: Add LocationType enum to level.rs

**Files:**
- Modify: `src/level.rs`

- [ ] **Step 1: Add LocationType enum above the Tile enum**

```rust
/// What kind of place a Location is. Determines atmosphere, procgen strategy, and flavor.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum LocationType {
    ShipInterior,
    SpaceStation,
    DerelictShip,
    AsteroidField,
    PlanetSurface { breathable: bool },
    DeepSpace,
    Ruins,
}
```

- [ ] **Step 2: Verify compilation**

Run: `cargo check 2>&1`
Expected: Compiles.

- [ ] **Step 3: Commit**

```bash
git add src/level.rs
git commit -m "feat: add LocationType enum"
```

### Task 3: Create galaxy.rs with Galaxy and Location structs

**Files:**
- Create: `src/galaxy.rs`
- Modify: `src/lib.rs`

- [ ] **Step 1: Create src/galaxy.rs**

```rust
use std::collections::HashMap;
use crate::level::{Level, LocationType};

/// Unique identifier for a Location in the Galaxy.
pub type LocationId = (i32, i32, i32);

/// Where the ship connects when docking at this location.
#[derive(Clone, Debug)]
pub struct LandingSpot {
    pub x: i32,
    pub y: i32,
    pub z: usize,
}

/// One variable-size zone in the galaxy.
#[derive(Clone, Debug)]
pub struct Location {
    pub width: usize,
    pub height: usize,
    pub depth: usize,
    pub levels: Vec<Level>,
    pub location_type: LocationType,
    pub landing_spots: Vec<LandingSpot>,
}

impl Location {
    pub fn new(
        width: usize,
        height: usize,
        depth: usize,
        location_type: LocationType,
        fill: crate::level::Tile,
    ) -> Self {
        let levels = (0..depth)
            .map(|_| Level::new(width, height, fill))
            .collect();
        Location {
            width,
            height,
            depth,
            levels,
            location_type,
            landing_spots: Vec::new(),
        }
    }

    pub fn level(&self, z: usize) -> &Level {
        &self.levels[z]
    }

    pub fn level_mut(&mut self, z: usize) -> &mut Level {
        &mut self.levels[z]
    }
}

/// The sparse galaxy map. Locations are lazily generated when first visited.
#[derive(Clone, Debug)]
pub struct Galaxy {
    pub locations: HashMap<LocationId, Location>,
}

impl Galaxy {
    pub fn new() -> Self {
        Galaxy {
            locations: HashMap::new(),
        }
    }

    pub fn get(&self, id: LocationId) -> Option<&Location> {
        self.locations.get(&id)
    }

    pub fn get_mut(&mut self, id: LocationId) -> Option<&mut Location> {
        self.locations.get_mut(&id)
    }

    pub fn insert(&mut self, id: LocationId, location: Location) {
        self.locations.insert(id, location);
    }

    /// Euclidean distance between two location coordinates.
    pub fn distance(a: LocationId, b: LocationId) -> f64 {
        let dx = (a.0 - b.0) as f64;
        let dy = (a.1 - b.1) as f64;
        let dz = (a.2 - b.2) as f64;
        (dx * dx + dy * dy + dz * dz).sqrt()
    }
}
```

- [ ] **Step 2: Register module in lib.rs**

Add to `src/lib.rs`:
```rust
pub mod galaxy;
```

- [ ] **Step 3: Verify compilation**

Run: `cargo check 2>&1`
Expected: Compiles.

- [ ] **Step 4: Commit**

```bash
git add src/galaxy.rs src/lib.rs
git commit -m "feat: add Galaxy and Location types"
```

### Task 4: Create ship.rs with Ship resource and layout builder

**Files:**
- Create: `src/ship.rs`
- Modify: `src/lib.rs`

- [ ] **Step 1: Define Ship constants and layout builder**

Create `src/ship.rs`:

```rust
use crate::galaxy::LocationId;
use crate::level::{Level, LocationType, Tile};

pub const SHIP_WIDTH: usize = 20;
pub const SHIP_HEIGHT: usize = 15;
pub const SHIP_DEPTH: usize = 1;

/// The position of the airlock within the ship's local tile grid.
pub const AIRLOCK_X: i32 = 10;
pub const AIRLOCK_Y: i32 = 14; // south wall, bottom row

/// The position of the flight console within the ship.
pub const CONSOLE_X: i32 = 10;
pub const CONSOLE_Y: i32 = 2;

/// Ship state resource.
#[derive(Clone, Debug)]
pub struct Ship {
    pub location_id: LocationId, // the ship's own location in the galaxy
    pub docked_at: Option<LocationId>,
    pub fuel: u32,
    pub max_fuel: u32,
}

impl Ship {
    pub fn new(location_id: LocationId) -> Self {
        Ship {
            location_id,
            docked_at: None,
            fuel: 500,
            max_fuel: 500,
        }
    }
}

/// Build the ship interior as a Location.
pub fn build_ship_interior() -> crate::galaxy::Location {
    use crate::galaxy::Location;

    let mut loc = Location::new(
        SHIP_WIDTH,
        SHIP_HEIGHT,
        SHIP_DEPTH,
        LocationType::ShipInterior,
        Tile::Vacuum,
    );

    let deck = loc.level_mut(0);

    // Fill interior with deck plates, surrounded by bulkhead walls
    for y in 0..SHIP_HEIGHT as i32 {
        for x in 0..SHIP_WIDTH as i32 {
            let is_edge = x == 0 || x == SHIP_WIDTH as i32 - 1
                || y == 0 || y == SHIP_HEIGHT as i32 - 1;
            deck.set(x, y, if is_edge { Tile::Bulkhead } else { Tile::DeckPlate });
        }
    }

    // Airlock door (south wall)
    deck.set(AIRLOCK_X, AIRLOCK_Y, Tile::AirlockDoor);

    // Windows along north and side walls
    for x in 3..17 {
        deck.set(x, 0, Tile::Window);
    }
    for y in 3..12 {
        deck.set(0, y, Tile::Window);
        deck.set(SHIP_WIDTH as i32 - 1, y, Tile::Window);
    }

    // Interior walls for rooms
    // Crew quarters (left side)
    for y in 4..7 {
        deck.set(6, y, Tile::Bulkhead);
    }
    deck.set(6, 4, Tile::AirlockDoor); // crew quarters door

    // Engineering (right side)
    for y in 8..11 {
        deck.set(13, y, Tile::Bulkhead);
    }
    deck.set(13, 9, Tile::AirlockDoor); // engineering door

    // Conduits in engineering
    deck.set(16, 10, Tile::Conduit);
    deck.set(17, 10, Tile::Conduit);
    deck.set(16, 9, Tile::Conduit);

    loc
}
```

- [ ] **Step 2: Register module in lib.rs**

Add to `src/lib.rs`:
```rust
pub mod ship;
```

- [ ] **Step 3: Verify compilation**

Run: `cargo check 2>&1`
Expected: Compiles.

- [ ] **Step 4: Commit**

```bash
git add src/ship.rs src/lib.rs
git commit -m "feat: add Ship resource and ship layout builder"
```

---

## Phase 2: ActiveZone — The Rendered Tile Grid

### Task 5: Create ActiveZone resource

**Files:**
- Create: `src/active_zone.rs`
- Modify: `src/lib.rs`

- [ ] **Step 1: Create src/active_zone.rs**

```rust
use crate::galaxy::{Galaxy, Location, LocationId};
use crate::level::Level;
use crate::ship::{Ship, AIRLOCK_X, AIRLOCK_Y};

/// The currently rendered tile grid. Holds the merged tiles of the ship
/// and (when docked) the destination location.
#[derive(Clone, Debug)]
pub struct ActiveZone {
    pub levels: Vec<Level>,
    pub width: usize,
    pub height: usize,
    pub depth: usize,
    /// Offset within the active zone where the ship's (0,0) tile maps.
    pub ship_origin: (i32, i32),
    /// If docked, offset where the destination's (0,0) tile maps.
    pub dest_origin: Option<(i32, i32)>,
    /// The destination's dimensions when docked.
    pub dest_dims: Option<(usize, usize, usize)>,
}

impl ActiveZone {
    /// Create an ActiveZone containing only the ship (deep space mode).
    pub fn ship_only(ship_loc: &Location) -> Self {
        let w = ship_loc.width;
        let h = ship_loc.height;
        ActiveZone {
            levels: ship_loc.levels.clone(),
            width: w,
            height: h,
            depth: ship_loc.depth,
            ship_origin: (0, 0),
            dest_origin: None,
            dest_dims: None,
        }
    }

    /// Create an ActiveZone with ship docked to a destination at a landing spot.
    /// Position the destination so its landing spot is adjacent to the ship's airlock.
    pub fn docked(
        ship_loc: &Location,
        dest: &Location,
        landing_spot_idx: usize,
    ) -> Option<Self> {
        let spot = dest.landing_spots.get(landing_spot_idx)?;

        // Ship airlock is at (AIRLOCK_X, AIRLOCK_Y) in ship-local coords.
        // The tile just south of the airlock (AIRLOCK_X, AIRLOCK_Y + 1) is the
        // boundary — that's where the destination's landing spot should be adjacent.
        // We place the destination so its landing spot tile is at (AIRLOCK_X, AIRLOCK_Y + 1).

        let dest_offset_x = 0; // Destination tiles placed flush — landing spot sits just below airlock
        let dest_offset_y = (AIRLOCK_Y + 2) - spot.y; // Position landing_spot at row AIRLOCK_Y + 2

        // Airlock is at ship_origin + (AIRLOCK_X, AIRLOCK_Y).
        // Landing spot should be adjacent (south):
        //   dest local (spot.x, spot.y) maps to active-zone (AIRLOCK_X, AIRLOCK_Y + 1)

        let dest_origin = (
            AIRLOCK_X - spot.x,
            AIRLOCK_Y + 1 - spot.y,
        );

        // Compute bounding box
        let ship_x0 = 0i32;
        let ship_y0 = 0i32;
        let ship_x1 = ship_loc.width as i32;
        let ship_y1 = ship_loc.height as i32;

        let dest_x0 = dest_origin.0;
        let dest_y0 = dest_origin.1;
        let dest_x1 = dest_origin.0 + dest.width as i32;
        let dest_y1 = dest_origin.1 + dest.height as i32;

        let min_x = ship_x0.min(dest_x0);
        let min_y = ship_y0.min(dest_y0);
        let max_x = ship_x1.max(dest_x1);
        let max_y = ship_y1.max(dest_y1);

        let total_w = (max_x - min_x) as usize;
        let total_h = (max_y - min_y) as usize;
        let total_d = ship_loc.depth.max(dest.depth);

        // Shift ship_origin so it's positive in the merged grid
        let ship_origin = (
            ship_x0 - min_x,
            ship_y0 - min_y,
        );
        let dest_origin_shifted = (
            dest_x0 - min_x,
            dest_y0 - min_y,
        );

        let mut levels: Vec<Level> = (0..total_d)
            .map(|_| Level::new(total_w, total_h, crate::level::Tile::Vacuum))
            .collect();

        // Copy ship tiles
        for y in 0..ship_loc.height {
            for x in 0..ship_loc.width {
                for z in 0..ship_loc.depth {
                    let tile = ship_loc.levels[z].tiles[y][x];
                    let lx = ship_origin.0 + x as i32;
                    let ly = ship_origin.1 + y as i32;
                    if lx >= 0 && ly >= 0
                        && (lx as usize) < total_w
                        && (ly as usize) < total_h
                    {
                        levels[z].set(lx, ly, tile);
                    }
                    // Copy items
                    if let Some(item) = ship_loc.levels[z].items[y][x] {
                        let lx = ship_origin.0 + x as i32;
                        let ly = ship_origin.1 + y as i32;
                        if lx >= 0 && ly >= 0
                            && (lx as usize) < total_w
                            && (ly as usize) < total_h
                        {
                            levels[z].set_item(lx, ly, Some(item));
                        }
                    }
                }
            }
        }

        // Copy destination tiles
        for y in 0..dest.height {
            for x in 0..dest.width {
                for z in 0..dest.depth {
                    let tile = dest.levels[z].tiles[y][x];
                    let lx = dest_origin_shifted.0 + x as i32;
                    let ly = dest_origin_shifted.1 + y as i32;
                    if lx >= 0 && ly >= 0
                        && (lx as usize) < total_w
                        && (ly as usize) < total_h
                    {
                        levels[z].set(lx, ly, tile);
                    }
                    if let Some(item) = dest.levels[z].items[y][x] {
                        let lx = dest_origin_shifted.0 + x as i32;
                        let ly = dest_origin_shifted.1 + y as i32;
                        if lx >= 0 && ly >= 0
                            && (lx as usize) < total_w
                            && (ly as usize) < total_h
                        {
                            levels[z].set_item(lx, ly, Some(item));
                        }
                    }
                }
            }
        }

        Some(ActiveZone {
            levels,
            width: total_w,
            height: total_h,
            depth: total_d,
            ship_origin,
            dest_origin: Some(dest_origin_shifted),
            dest_dims: Some((dest.width, dest.height, dest.depth)),
        })
    }

    pub fn level(&self, z: usize) -> &Level {
        &self.levels[z]
    }

    pub fn level_mut(&mut self, z: usize) -> &mut Level {
        &mut self.levels[z]
    }
}
```

- [ ] **Step 2: Register module in lib.rs**

Add to `src/lib.rs`:
```rust
pub mod active_zone;
```

- [ ] **Step 3: Verify compilation**

Run: `cargo check 2>&1`
Expected: Compiles.

- [ ] **Step 4: Commit**

```bash
git add src/active_zone.rs src/lib.rs
git commit -m "feat: add ActiveZone resource for merged tile rendering"
```

---

## Phase 3: Space Game Bootstrap — Render the Ship

### Task 6: Create space game entry point that renders the ship

**Files:**
- Modify: `src/main.rs`

- [ ] **Step 1: Add space game resource types**

Add above `fn main()` in `src/main.rs`:

```rust
/// Holds the currently active tile grid for rendering.
#[derive(Resource)]
struct CurrentZone(pub active_zone::ActiveZone);
```

Add imports at the top of `main.rs`:
```rust
use trl::{galaxy, ship, active_zone};
```

- [ ] **Step 2: Add a space_main() function**

Add below the existing `main()` function:

```rust
fn space_main() {
    use level::ZONE_WIDTH;
    use level::ZONE_HEIGHT;

    let mut galaxy = galaxy::Galaxy::new();
    let ship_id: galaxy::LocationId = (-1, -1, -1); // special sentinel coords for ship

    // Build ship location and insert into galaxy
    let ship_location = ship::build_ship_interior();
    galaxy.insert(ship_id, ship_location.clone());

    let ship_res = ship::Ship::new(ship_id);
    let active = active_zone::ActiveZone::ship_only(&ship_location);

    // Player starts on the ship, near the center
    let start_x: i32 = ship::SHIP_WIDTH as i32 / 2;
    let start_y: i32 = ship::SHIP_HEIGHT as i32 / 2;
    let start_z: usize = 0;

    let fov = level::FovGrid::new(active.width, active.height);

    App::new()
        .add_plugins(haalka::HaalkaPlugin::default())
        .add_plugins(DefaultPlugins
            .set(ImagePlugin::default_linear())
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: "trl — space".into(),
                    resolution: (1200u32, 800u32).into(),
                    ..default()
                }),
                ..default()
            }))
        .insert_resource(ClearColor(Color::srgb(0.02, 0.02, 0.05)))
        .insert_resource(galaxy)
        .insert_resource(ship_res)
        .insert_resource(CurrentZone(active))
        .init_resource::<RenderFrame>()
        .init_resource::<TurnBasedWorldState>()
        .insert_resource(Clock::new())
        .insert_resource(TimeModeAuto(true))
        .init_resource::<ChestOpenPending>()
        .insert_resource(UiState::default())
        .insert_resource(Fov(fov))
        .insert_resource(TileEntityIndex::default())
        .add_plugins(ui::UiPlugin)
        .add_systems(Startup, (space_setup, ui::spawn_haalka_root).chain())
        .configure_sets(Update, SimStep.run_if(should_run_sim_step))
        .add_systems(Update, (
            bump_render_frame,
            maintain_tile_index,
            setup_glyph_visuals,
            update_time_mode,
            handle_world_map,
            handle_dialogue,
            handle_menus,
            handle_interact,
            handle_utility_menus,
            space_player_input,
            ApplyDeferred,
            apply_gravity.in_set(SimStep),
            enemy_ai.in_set(SimStep),
            track_movement,
            interpolate_visual_positions,
            sync_entity_positions,
            update_entity_visibility,
            space_camera_follow,
            update_fov_visuals,
            update_tile_hover_highlight,
        ).chain())
        .run();
}
```

- [ ] **Step 3: Add space_setup startup system**

```rust
fn space_setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    current: Res<CurrentZone>,
    mut fov: ResMut<Fov>,
    mut images: ResMut<Assets<Image>>,
    mut world_map: ResMut<WorldMapView>,
) {
    commands.spawn((Camera2d, Fxaa::default(), Msaa::Off));

    // Render the active zone tiles
    spawn_level_tiles_space(&mut commands, &asset_server, &current.0);

    let hover_img = white_pixel_image(&mut images);
    commands.spawn((
        TileHoverHighlight,
        Sprite {
            image: hover_img,
            custom_size: Some(Vec2::splat(TILE_SIZE)),
            color: Color::srgba(0.95, 0.92, 0.45, 0.28),
            ..default()
        },
        Transform::from_translation(Vec3::new(0.0, 0.0, 0.25)),
        Visibility::Hidden,
    ));

    // Place player at center of ship
    let start_x: i32 = ship::SHIP_WIDTH as i32 / 2;
    let start_y: i32 = ship::SHIP_HEIGHT as i32 / 2;
    let (sox, soy) = current.0.ship_origin;
    let local_x = sox + start_x;
    let local_y = soy + start_y;

    let start_local = Vec2::new(local_x as f32, local_y as f32);

    commands.spawn((
        Sprite {
            image: asset_server.load("textures/retro-future_post-apocalyptic_settlement_guard.png"),
            custom_size: Some(Vec2::splat(TILE_SIZE)),
            color: Color::WHITE,
            ..default()
        },
        Transform::from_translation(
            tile_screen_pos(local_x as f32, local_y as f32, current.0.width, current.0.height) + Vec3::Z
        ),
        Player,
        SpacePlayerPos { x: local_x, y: local_y, z: 0 },
        Stats { hp: 20, max_hp: 20, attack: 5, move_speed: 3.0, attack_speed: 1.0 },
        Inventory::default(),
        GlyphVisual,
        Visuals {
            prev: start_local,
            last_move_start_frame: None,
            display: start_local,
            last_pos: start_local,
        },
    ));

    // Compute initial FOV
    let lvl = current.0.level(0);
    compute_fov_space(&mut fov.0, lvl, local_x, local_y, FOV_RADIUS);

    // World map image from the active zone surface
    world_map.image = generate_map_from_active_zone(&current.0, &mut images);
}
```

- [ ] **Step 4: Add SpacePlayerPos component**

```rust
/// Player position in ActiveZone-local tile coordinates.
#[derive(Component)]
struct SpacePlayerPos {
    x: i32,
    y: i32,
    z: usize,
}
```

- [ ] **Step 5: Add space tile rendering function**

```rust
fn spawn_level_tiles_space(
    commands: &mut Commands,
    asset_server: &AssetServer,
    zone: &active_zone::ActiveZone,
) {
    for z in 0..zone.depth {
        let level = zone.level(z);
        for y in 0..level.height {
            for x in 0..level.width {
                let tile = level.tiles[y][x];
                if tile == Tile::Air || tile == Tile::Vacuum {
                    continue;
                }
                let pos = tile_screen_pos(x as f32, y as f32, zone.width, zone.height);
                if let Some(path) = tile.texture_path() {
                    commands.spawn((
                        Sprite {
                            image: asset_server.load(path),
                            custom_size: Some(Vec2::splat(TILE_SIZE)),
                            color: Color::srgba(0.0, 0.0, 0.0, 0.0),
                            ..default()
                        },
                        Transform::from_translation(pos),
                        TileGlyph { x, y },
                        TilePng,
                    ));
                } else {
                    let [r, g, b] = tile.color();
                    commands.spawn((
                        Text2d::new(tile.glyph()),
                        TextFont { font_size: TILE_SIZE, ..default() },
                        TextColor(Color::srgba(r, g, b, 0.0)),
                        Transform::from_translation(pos),
                        TileGlyph { x, y },
                    ));
                }

                if let Some(item) = level.items[y][x] {
                    let [r, g, b] = item.color();
                    commands.spawn((
                        Text2d::new(item.glyph()),
                        TextFont { font_size: TILE_SIZE, ..default() },
                        TextColor(Color::srgba(r, g, b, 0.0)),
                        Transform::from_translation(
                            tile_screen_pos(x as f32, y as f32, zone.width, zone.height)
                                + Vec3::new(0.0, 0.0, 1.0)
                        ),
                        ItemGlyph { x, y },
                    ));
                }
            }
        }
    }
}
```

- [ ] **Step 6: Add space camera follow**

```rust
fn space_camera_follow(
    player_q: Query<&Visuals, With<Player>>,
    current: Res<CurrentZone>,
    mut cam_q: Query<&mut Transform, With<Camera2d>>,
    windows: Query<&Window>,
) {
    if let Ok(vis) = player_q.single()
        && let Ok(mut cam_tf) = cam_q.single_mut()
        && let Ok(win) = windows.single()
    {
        let w = win.resolution.width();
        let h = win.resolution.height();
        let screen_center = Vec2::new(w / 2.0, h / 2.0);
        let viewport_center = Vec2::new(w * 0.35, (h - STATUS_BAR_HEIGHT) / 2.0);
        let offset = viewport_center - screen_center;

        let local = vis.display;
        let world_pos = Vec2::new(
            (local.x - current.0.width as f32 / 2.0) * TILE_SIZE,
            (current.0.height as f32 / 2.0 - local.y) * TILE_SIZE,
        );
        cam_tf.translation = (world_pos - offset).extend(0.0);
    }
}
```

- [ ] **Step 7: Add FOV helper for space game**

```rust
fn compute_fov_space(fov: &mut level::FovGrid, level: &level::Level, lx: i32, ly: i32, radius: i32) {
    compute_fov(fov, level, lx, ly, radius, |_, _| false);
}
```

- [ ] **Step 8: Add map generation from ActiveZone**

```rust
fn generate_map_from_active_zone(
    zone: &active_zone::ActiveZone,
    images: &mut Assets<Image>,
) -> Handle<Image> {
    use bevy::asset::RenderAssetUsages;
    use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

    let w = zone.width;
    let h = zone.height;
    let mut data = vec![0u8; w * h * 4];

    let level = zone.level(0);
    for ty in 0..h {
        for tx in 0..w {
            let [r, g, b] = level.tiles[ty][tx].minimap_color();
            let idx = (ty * w + tx) * 4;
            data[idx]     = (r * 255.0) as u8;
            data[idx + 1] = (g * 255.0) as u8;
            data[idx + 2] = (b * 255.0) as u8;
            data[idx + 3] = 255;
        }
    }

    images.add(Image::new(
        Extent3d {
            width: w as u32,
            height: h as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    ))
}
```

- [ ] **Step 9: Add space player input system**

```rust
fn space_player_input(
    keys: Res<ButtonInput<KeyCode>>,
    current: Res<CurrentZone>,
    ui: Res<UiState>,
    world_map: Res<WorldMapView>,
    mut clock: ResMut<Clock>,
    mut tb: ResMut<TurnBasedWorldState>,
    mut time_mode_auto: ResMut<TimeModeAuto>,
    mut fov: ResMut<Fov>,
    index: Res<TileEntityIndex>,
    mut player_query: Query<(&mut SpacePlayerPos, &Stats, &mut Inventory), With<Player>>,
    mut enemy_query: Query<&mut Stats, (With<Enemy>, Without<Player>)>,
    collidable_q: Query<&Collidable>,
) {
    // Time mode toggle
    if !ui.any_open() && !world_map.open && keys.just_pressed(KeyCode::KeyT) {
        if keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight) {
            time_mode_auto.0 = true;
        } else {
            time_mode_auto.0 = false;
            clock.mode = match clock.mode {
                TimeMode::RealTime => TimeMode::TurnBased,
                TimeMode::TurnBased => {
                    tb.pending_enemy_phase = false;
                    TimeMode::RealTime
                }
            };
        }
    }

    if !ui.any_open() && !world_map.open
        && let Ok((mut pos, stats, mut inventory)) = player_query.single_mut()
    {
        let player_attack = stats.attack;
        if clock.move_cooldown_frames > 0 {
            clock.move_cooldown_frames -= 1;
        }

        let turn_based_block = clock.mode == TimeMode::TurnBased
            && (clock.move_cooldown_frames > 0 || tb.pending_enemy_phase);

        if !turn_based_block && keys.just_pressed(KeyCode::Period)
            && !keys.pressed(KeyCode::ShiftLeft)
            && !keys.pressed(KeyCode::ShiftRight)
        {
            clock.advance(PlayerAction::Wait.time_cost());
            note_player_turn_moved_world(&*clock, &mut *tb);
        } else if !turn_based_block
            && (if clock.mode == TimeMode::TurnBased {
                any_direction_just_pressed(&keys)
            } else {
                any_direction_pressed(&keys)
            }) && clock.move_cooldown_frames == 0
        {
            let level = current.0.level(pos.z);
            let dir = read_direction(&keys);
            let (raw_dx, raw_dy) = (dir.0, dir.1);

            let (dx, dy) = resolve_move(level, pos.x, pos.y, raw_dx, raw_dy);

            if (dx, dy) != (0, 0) {
                let target_x = pos.x + dx;
                let target_y = pos.y + dy;

                // Bump-attack enemies
                let enemy_hit = index.0.get(&(target_x, target_y, pos.z))
                    .and_then(|entities| entities.iter().find(|&&e| enemy_query.get(e).is_ok()).copied());

                if let Some(hostile) = enemy_hit {
                    if let Ok(mut es) = enemy_query.get_mut(hostile) {
                        es.hp -= player_attack;
                        // Enemy death is handled by enemy_death_check (Task 12)
                    }
                } else {
                    let blocked = index.0.get(&(target_x, target_y, pos.z))
                        .is_some_and(|entities| entities.iter().any(|&e| {
                            collidable_q.get(e).is_ok_and(|c| c.0)
                        }));

                    if !blocked {
                        pos.x = target_x;
                        pos.y = target_y;
                        compute_fov_space(
                            &mut fov.0,
                            current.0.level(pos.z),
                            pos.x,
                            pos.y,
                            FOV_RADIUS,
                        );

                        // Auto-pickup
                        let lvl = current.0.level(pos.z);
                        if (pos.y as usize) < lvl.height && (pos.x as usize) < lvl.width {
                            if let Some(item) = lvl.items[pos.y as usize][pos.x as usize] {
                                *inventory.0.entry(item).or_insert(0) += 1;
                            }
                        }
                    }
                }

                clock.advance(PlayerAction::Move { dx, dy }.time_cost());
                clock.move_cooldown_frames = RENDER_FRAMES_PER_SIM_STEP;
                note_player_turn_moved_world(&*clock, &mut *tb);
            }
        }
    }
}
```

Note: The `commands` access for despawning enemies can't be done through a regular query parameter — we'll need a follow-up task to add a death queue or use `EventWriter`. For now, enemy death on bump-attack is deferred.

- [ ] **Step 10: Wire space_main() and make both entry points accessible**

Modify `fn main()` to call `space_main()` instead (temporarily, for testing):

```rust
fn main() {
    space_main();
}
```

Keep the original `main()` function body accessible by renaming it to `fantasy_main()` (not called, preserved).

- [ ] **Step 11: Verify compilation and run**

Run: `cargo check 2>&1`
Expected: Compiles.

Run: `cargo run 2>&1`
Expected: Game window opens, ship interior tiles are visible, player can walk around the ship.

- [ ] **Step 12: Commit**

```bash
git add src/main.rs
git commit -m "feat: add space game bootstrap with ship rendering"
```

---

## Phase 4: Docking & Undocking

### Task 7: Implement docking and undocking systems

**Files:**
- Create: `src/docking.rs`
- Modify: `src/lib.rs`
- Modify: `src/main.rs`

- [ ] **Step 1: Create src/docking.rs**

```rust
use crate::active_zone::ActiveZone;
use crate::galaxy::{Galaxy, LocationId};
use crate::ship::Ship;
use bevy::prelude::*;

/// Event: request to dock the ship at a specific landing spot of the destination.
#[derive(Event)]
pub struct DockEvent {
    pub dest_id: LocationId,
    pub landing_spot_idx: usize,
}

/// Event: request to undock (prepares for transit or deep space).
#[derive(Event)]
pub struct UndockEvent;

/// System that handles DockEvent: merges ship + destination into ActiveZone.
pub fn handle_dock(
    mut events: EventReader<DockEvent>,
    mut galaxy: ResMut<Galaxy>,
    mut ship: ResMut<Ship>,
    mut current: ResMut<ActiveZone>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    tile_query: Query<Entity, With<crate::TileGlyph>>,
) {
    for ev in events.read() {
        let Some(ship_loc) = galaxy.get(ship.location_id).cloned() else {
            continue;
        };
        let Some(dest) = galaxy.get(ev.dest_id) else {
            continue;
        };

        let Some(merged) = ActiveZone::docked(&ship_loc, dest, ev.landing_spot_idx) else {
            warn!("Failed to merge ship with destination at {:?}", ev.dest_id);
            continue;
        };

        ship.docked_at = Some(ev.dest_id);

        // Rebuild tile entities for the merged zone
        despawn_level_tiles(&mut commands, &tile_query);
        spawn_level_tiles_space(&mut commands, &asset_server, &merged);

        *current = merged;
        info!("Docked at {:?}", ev.dest_id);
    }
}

/// System that handles UndockEvent: extracts ship tiles back into ship-only ActiveZone.
pub fn handle_undock(
    mut events: EventReader<UndockEvent>,
    mut galaxy: ResMut<Galaxy>,
    mut ship: ResMut<Ship>,
    mut current: ResMut<ActiveZone>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    tile_query: Query<Entity, With<crate::TileGlyph>>,
) {
    for _ in events.read() {
        let Some(ship_loc) = galaxy.get(ship.location_id) else {
            continue;
        };

        ship.docked_at = None;
        let ship_only = ActiveZone::ship_only(ship_loc);

        despawn_level_tiles(&mut commands, &tile_query);
        spawn_level_tiles_space(&mut commands, &asset_server, &ship_only);

        *current = ship_only;
        info!("Undocked — now in deep space");
    }
}
```

- [ ] **Step 2: Register module and events**

Add to `src/lib.rs`:
```rust
pub mod docking;
```

- [ ] **Step 3: Add docking systems to space_main()**

In `space_main()`, add events and docking systems:

```rust
.add_event::<docking::DockEvent>()
.add_event::<docking::UndockEvent>()
.add_systems(Update, (
    docking::handle_dock,
    docking::handle_undock,
    // ... rest of systems
))
```

- [ ] **Step 4: Verify compilation**

Run: `cargo check 2>&1`
Expected: Compiles.

- [ ] **Step 5: Commit**

```bash
git add src/docking.rs src/lib.rs src/main.rs
git commit -m "feat: add docking and undocking systems"
```

---

## Phase 5: Starter Planet — First Dockable Location

### Task 8: Generate starter planet with existing NPCs

**Files:**
- Create: `src/galaxy_gen.rs`
- Modify: `src/lib.rs`
- Modify: `src/main.rs`

- [ ] **Step 1: Create src/galaxy_gen.rs with starter planet generation**

```rust
use crate::galaxy::Location;
use crate::level::{LocationType, Tile, fill_rect, place_room, place_room_with_door, Side, Level};

/// Generate a small starter planet at the origin of the galaxy.
pub fn generate_starter_planet() -> Location {
    const W: usize = 48;
    const H: usize = 48;
    const D: usize = 1;

    let mut loc = Location::new(W, H, D, LocationType::PlanetSurface { breathable: true }, Tile::AlienGrass);

    let level = loc.level_mut(0);

    // Small outpost building
    place_room_with_door(level, 18, 20, 12, 8, Side::South, 6, Tile::StationWall);
    // Change interior to StationFloor
    for y in 21..27 {
        for x in 19..29 {
            level.set(x, y, Tile::StationFloor);
        }
    }

    // Landing spot just south of the building
    fill_rect(level, 18, 28, 12, 4, Tile::Road);
    loc.landing_spots.push(crate::galaxy::LandingSpot {
        x: 24,  // center of the road pad
        y: 29,  // top of the road pad
        z: 0,
    });

    // Scatter some trees and rocks for flavor
    let feature_spots: &[(i32, i32, Tile)] = &[
        (5, 5, Tile::CrystalGrowth),
        (8, 12, Tile::CrystalGrowth),
        (40, 8, Tile::CrystalGrowth),
        (38, 30, Tile::CrystalGrowth),
        (10, 35, Tile::AlienFluid),
        (11, 35, Tile::AlienFluid),
        (10, 36, Tile::AlienFluid),
    ];

    for &(x, y, tile) in feature_spots {
        level.set(x, y, tile);
    }

    loc
}

/// Generate a named NPC spawn placement for the starter planet.
pub struct NpcPlacement {
    pub x: i32,
    pub y: i32,
    pub object: crate::entities::Object,
}

pub fn starter_planet_npcs() -> Vec<NpcPlacement> {
    vec![
        NpcPlacement { x: 22, y: 25, object: crate::npcs::mira::mira() },
        NpcPlacement { x: 20, y: 23, object: crate::npcs::chronos::chronos() },
        NpcPlacement { x: 26, y: 22, object: crate::npcs::unit7::unit7() },
        NpcPlacement { x: 22, y: 21, object: crate::npcs::kong::kong() },
        NpcPlacement { x: 24, y: 23, object: crate::npcs::guard::guard() },
    ]
}
```

- [ ] **Step 2: Register module in lib.rs**

Add to `src/lib.rs`:
```rust
pub mod galaxy_gen;
```

- [ ] **Step 3: Update space_main to include starter planet and dock at startup**

In `space_main()`, after creating the galaxy:

```rust
// Add starter planet at origin
let origin: galaxy::LocationId = (0, 0, 0);
let starter_planet = galaxy_gen::generate_starter_planet();
galaxy.insert(origin, starter_planet.clone());

// Build ship location
let ship_location = ship::build_ship_interior();
galaxy.insert(ship_id, ship_location.clone());

// Ship starts docked at the starter planet
let active = active_zone::ActiveZone::docked(
    &ship_location,
    &starter_planet,
    0, // first landing spot
).expect("ship should dock at starter planet");

let ship_res = ship::Ship {
    location_id: ship_id,
    docked_at: Some(origin),
    fuel: 500,
    max_fuel: 500,
};
```

- [ ] **Step 4: Spawn starter planet NPCs in space_setup**

In `space_setup()`, after spawning the player, add:

```rust
// Spawn starter planet NPCs at their destination-local coords mapped into the active zone
if let Some((dox, doy)) = current.0.dest_origin {
    for placement in galaxy_gen::starter_planet_npcs() {
        let wx = dox + placement.x;
        let wy = doy + placement.y;
        placement.object.spawn_at(&mut commands, wx, wy, 0);
    }
}

// Spawn trees as entities at destination coordinates
if let Some((dox, doy)) = current.0.dest_origin {
    for &(lx, ly) in &[
        (5, 5), (8, 12), (40, 8), (38, 30),
    ] {
        let wx = dox + lx;
        let wy = doy + ly;
        Object::tree().spawn_at(&mut commands, wx, wy, 0);
    }
}
```

- [ ] **Step 5: Verify compilation and run**

Run: `cargo check 2>&1`
Expected: Compiles.

Run: `cargo run 2>&1`
Expected: Player starts on the ship docked at the starter planet. Can walk from ship airlock onto the planet surface. NPCs visible.

- [ ] **Step 6: Commit**

```bash
git add src/galaxy_gen.rs src/lib.rs src/main.rs
git commit -m "feat: add starter planet with docked ship and existing NPCs"
```

---

## Phase 6: Navigation Console & Transit

### Task 9: Create navigation UI and transit system

**Files:**
- Create: `src/navigation.rs`
- Modify: `src/lib.rs`
- Modify: `src/main.rs`

- [ ] **Step 1: Create src/navigation.rs**

```rust
use bevy::prelude::*;
use crate::galaxy::{Galaxy, LocationId};
use crate::ship::Ship;
use crate::active_zone::ActiveZone;
use crate::docking::{DockEvent, UndockEvent};

/// Transit state: ship is traveling between locations.
#[derive(Resource, Default)]
pub struct TransitState {
    pub active: bool,
    pub elapsed: f32,
    pub duration: f32,
    pub dest_coords: Option<LocationId>,
}

/// Navigation UI open/closed state for the flight console.
#[derive(Resource, Default)]
pub struct NavView {
    pub open: bool,
    pub selected_idx: usize,
}

/// Known locations are all locations that have been generated or discovered.
/// In the future, some locations won't be known until nav data is found.
pub fn known_locations(galaxy: &Galaxy) -> Vec<(LocationId, &crate::galaxy::Location)> {
    galaxy.locations.iter()
        .filter(|(id, _)| **id != (-1, -1, -1)) // exclude ship
        .map(|(id, loc)| (*id, loc))
        .collect()
}

/// Initiate a jump to a destination. Triggers undock + transit start.
pub fn initiate_jump(
    dest: LocationId,
    ship: &mut Ship,
    transit: &mut TransitState,
    undock_events: &mut EventWriter<UndockEvent>,
) {
    undock_events.send(UndockEvent);
    ship.fuel = ship.fuel.saturating_sub(fuel_cost(ship.docked_at, dest));
    *transit = TransitState {
        active: true,
        elapsed: 0.0,
        duration: 20.0,
        dest_coords: Some(dest),
    };
}

fn fuel_cost(from: Option<LocationId>, to: LocationId) -> u32 {
    let origin = from.unwrap_or((0, 0, 0));
    Galaxy::distance(origin, to).ceil() as u32 * 10
}

/// Advance transit timer each frame. When complete, place ship at destination.
pub fn tick_transit(
    time: Res<Time>,
    mut transit: ResMut<TransitState>,
    mut ship: ResMut<Ship>,
    mut dock_events: EventWriter<DockEvent>,
) {
    if !transit.active {
        return;
    }
    transit.elapsed += time.delta_secs();
    if transit.elapsed >= transit.duration {
        transit.active = false;
        if let Some(dest) = transit.dest_coords {
            ship.docked_at = None; // arrived in deep space near dest, not docked yet
            info!("Arrived at {:?} — in deep space", dest);
        }
    }
}

/// Render the transit progress as a warp overlay effect.
/// For now, just log progress. Future: moving starfield, screen shake.
pub fn transit_visuals(
    transit: Res<TransitState>,
    mut log: ResMut<crate::ui::LogEntries>,
) {
    if transit.active {
        // This runs every frame — only log periodically in practice
        // Minimal: do nothing visual yet, just track state
    }
}
```

- [ ] **Step 2: Register module in lib.rs**

Add to `src/lib.rs`:
```rust
pub mod navigation;
```

- [ ] **Step 3: Add navigation systems to space_main**

Add resources and systems:

```rust
.init_resource::<navigation::NavView>()
.init_resource::<navigation::TransitState>()
.add_systems(Update, (
    navigation::tick_transit,
    navigation::transit_visuals,
    // ... rest of systems in chain
))
```

- [ ] **Step 4: Add flight console interaction entity**

In `src/entities.rs`, add a flight console constructor:

```rust
impl Object {
    pub fn flight_console() -> Self {
        Self::structure(true).add((
            Glyph::ascii('C', Color::srgb(0.3, 0.9, 0.4)),
            Named {
                name: "Flight Console",
                flavor: "Navigation computer. Plot a course to a destination.",
            },
            FlightConsole,
        ))
    }
}

/// Marker component for the flight console entity.
#[derive(Component)]
pub struct FlightConsole;
```

- [ ] **Step 5: Spawn flight console on the ship in space_setup**

In `space_setup()`, spawn the console entity at ship-local coordinates mapped to active zone:

```rust
// Spawn flight console
let (sox, soy) = current.0.ship_origin;
let console_x = sox + ship::CONSOLE_X;
let console_y = soy + ship::CONSOLE_Y;
Object::flight_console().spawn_at(&mut commands, console_x, console_y, 0);
```

- [ ] **Step 6: Verify compilation**

Run: `cargo check 2>&1`
Expected: Compiles.

- [ ] **Step 7: Commit**

```bash
git add src/navigation.rs src/lib.rs src/entities.rs src/main.rs
git commit -m "feat: add navigation types, transit system, and flight console entity"
```

---

## Phase 7: Atmosphere & EVA

### Task 10: Create atmosphere/vacuum damage system

**Files:**
- Create: `src/atmosphere.rs`
- Modify: `src/lib.rs`
- Modify: `src/main.rs`
- Modify: `src/entities.rs`
- Modify: `src/level.rs`

- [ ] **Step 1: Create src/atmosphere.rs**

```rust
use bevy::prelude::*;
use crate::level::{Level, Tile};
use crate::SpacePlayerPos;

/// Marker: entity is protected from vacuum damage.
#[derive(Component)]
pub struct EVASuit;

/// System: entities on vacuum tiles without EVASuit take 1 damage per sim tick.
pub fn vacuum_damage(
    current: Res<crate::CurrentZone>,
    mut query: Query<(&SpacePlayerPos, &mut crate::entities::Stats), Without<EVASuit>>,
) {
    for (pos, mut stats) in query.iter_mut() {
        let level = current.0.level(pos.z);
        if pos.y < level.height as i32 && pos.x < level.width as i32
            && (pos.y as usize) < level.height
            && (pos.x as usize) < level.width
        {
            let tile = level.tiles[pos.y as usize][pos.x as usize];
            if !tile.has_atmosphere() {
                stats.hp -= 1;
            }
        }
    }
}
```

- [ ] **Step 2: Register module**

Add to `src/lib.rs`:
```rust
pub mod atmosphere;
```

- [ ] **Step 3: Add EVA suit item to level.rs Item enum**

Add to the `Item` enum in `src/level.rs` (not entities.rs):
```rust
EVASuit,
```

With name/glyph/color:
```rust
Item::EVASuit => "EVA Suit",
// glyph:
Item::EVASuit => "[",
// color:
Item::EVASuit => [0.9, 0.85, 0.7],
```

- [ ] **Step 4: Add vacuum_damage to SimStep in space_main**

```rust
vacuum_damage.in_set(SimStep),
```

- [ ] **Step 5: Mark player with EVASuit when they have one in inventory**

For now, add the component at startup to let the player walk in vacuum on the starter planet (which has atmosphere, so this is a no-op test). The real EVA suit gating comes later.

- [ ] **Step 6: Verify compilation**

Run: `cargo check 2>&1`
Expected: Compiles.

- [ ] **Step 7: Commit**

```bash
git add src/atmosphere.rs src/lib.rs src/main.rs src/level.rs
git commit -m "feat: add atmosphere/vacuum damage system and EVA suit item"
```

---

## Phase 8: Crew System

### Task 11: Create crew recruitment and shipboard AI

**Files:**
- Create: `src/crew.rs`
- Modify: `src/lib.rs`
- Modify: `src/main.rs`

- [ ] **Step 1: Create src/crew.rs**

```rust
use bevy::prelude::*;

/// Role a crew member serves on the ship.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CrewRole {
    Engineer,
    Medic,
    Gunner,
    Passenger,
}

/// Marks an NPC as a crew member aboard a ship.
#[derive(Component, Debug)]
pub struct CrewMember {
    pub role: CrewRole,
    pub ship_id: Entity,
}

/// Passive effects from crew members.
pub fn crew_passive_effects(
    crew_q: Query<&CrewMember>,
    mut ship: ResMut<crate::ship::Ship>,
) {
    let has_engineer = crew_q.iter().any(|c| c.role == CrewRole::Engineer);
    if has_engineer {
        // Engineer reduces fuel consumption — handled at jump time
        // For now, just a marker; the ship resource tracks this
    }
}

/// Simple shipboard AI: crew members wander randomly on the ship interior.
/// Only runs when the ship is the active location (not in transit necessarily,
/// but crew are always aboard).
pub fn crew_wander(
    mut crew_q: Query<&mut crate::entities::Location, With<CrewMember>>,
) {
    // For now, crew stay in place. Future: random walk within ship bounds.
}
```

- [ ] **Step 2: Register module**

Add to `src/lib.rs`:
```rust
pub mod crew;
```

- [ ] **Step 3: Add crew systems to space_main**

```rust
crew::crew_passive_effects,
crew::crew_wander,
```

- [ ] **Step 4: Verify compilation**

Run: `cargo check 2>&1`
Expected: Compiles.

- [ ] **Step 5: Commit**

```bash
git add src/crew.rs src/lib.rs src/main.rs
git commit -m "feat: add crew member component and basic shipboard systems"
```

---

## Phase 9: Integration & Playtesting

### Task 12: Polish pass — fix compilation issues, wire systems together

**Files:**
- Modify: `src/main.rs`

- [ ] **Step 1: Resolve enemy death Commands access in space_player_input**

The `commands` parameter was missing from `space_player_input`. Remove enemy death from that system and add a separate `enemy_death_check` system:

```rust
fn enemy_death_check(
    mut commands: Commands,
    enemy_q: Query<(Entity, &Stats), With<Enemy>>,
) {
    for (entity, stats) in enemy_q.iter() {
        if stats.hp <= 0 {
            commands.entity(entity).despawn();
        }
    }
}
```

Add it to the chain in SimStep:
```rust
enemy_death_check.in_set(SimStep),
```

- [ ] **Step 2: Ensure all space_main systems are wired correctly**

Verify the full system chain compiles and runs. The `space_player_input` function needs `Commands` removed and the enemy death check extracted.

- [ ] **Step 3: Verify compilation and run**

Run: `cargo check 2>&1`
Expected: Compiles.

Run: `cargo run 2>&1`
Expected: Ship docked at starter planet, flight console interactable, player and NPCs visible, can walk between ship and planet.

- [ ] **Step 4: Commit**

```bash
git add src/main.rs
git commit -m "feat: integration polish for space game systems"
```

---

## Task 13: Self-review checklist

After all tasks are complete, verify:
- [ ] `cargo check` passes with no errors
- [ ] `cargo run` launches the space game with ship + starter planet
- [ ] Player can walk around the ship interior
- [ ] Player walks through airlock onto planet surface (merged zone)
- [ ] NPCs visible and interactable on the planet
- [ ] Flight console entity exists and is interactable
- [ ] ZoneWorld code still compiles (verify with cfg flag or separate check)
