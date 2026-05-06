# Space Game Design

**Date**: 2026-05-06
**Status**: Approved

## Overview

Pivot the game to a space setting. The player owns a physical spaceship that teleports between locations in a 3D galaxy. Each location is a variable-size zone with atmosphere or vacuum. The ship docks to destinations, merging into a single walkable space. Hundreds of procgen locations: asteroid fields, space stations, derelict ships, planetary surfaces, ruins.

ZoneWorld and the existing fantasy content are preserved; the space game is a new world implementation alongside it.

---

## 1. Galaxy & Location Model

Locations live at 3D integer coordinates `(gx, gy, gz)` in a sparse galaxy map. Each location has variable width, height, and depth (number of z-levels/decks).

```rust
struct Galaxy {
    locations: HashMap<(i32, i32, i32), Location>,
}

struct Location {
    width: usize,
    height: usize,
    depth: usize,
    levels: Vec<Level>,        // one Level per z-slice (reuses existing Level)
    location_type: LocationType,
    landing_spots: Vec<LandingSpot>,
}

struct LandingSpot {
    x: i32, y: i32, z: usize,  // where the ship airlock connects
}
```

A `Level` is the existing `Level` — a 2D grid of `Tile`s + items. The difference from `ZoneWorld` is that locations vary in size and are sparsely placed in 3D rather than filling a dense 2D grid.

The galaxy is deterministic from `WORLD_SEED` — locations are lazily generated when the ship first visits their coordinates.

---

## 2. Ship as a Persistent Location

The ship is a `Location` that exists independently of the galaxy. It has a fixed layout (e.g. 20x15 tiles across 1 z-level), built once at character creation.

Key tiles/features:
- **Flight console** — interactable entity, opens the navigation UI
- **Airlock** — tile that connects to a destination's landing spot when docked; in deep space, used for EVA
- **Crew quarters** — beds, storage
- **Engineering** — reactor/fuel systems, interactable for refueling/repairs
- **Windows** — special tiles that render the outside (starfield in deep space, destination exterior when docked)

```rust
struct Ship {
    location_id: LocationId,
    docked_at: Option<(i32, i32, i32)>,  // None = deep space
    fuel: u32,
    crew: Vec<Entity>,
}
```

Ship interior entities (console, NPCs, items) are spawned like any other location's entities. When the ship detaches, those entities persist.

---

## 3. Docking, Undocking, and Zone Merging

When the ship docks, both the ship and destination locations are active simultaneously — they form one continuous walkable space.

**Docking:**
1. Ship is in deep space near the destination
2. Player uses flight console, selects a landing spot
3. Ship's airlock tile is aligned adjacent to the landing spot tile
4. Both location's tiles render together; FOV covers the combined space
5. Walking from airlock to landing spot is a normal tile move

**Undocking:**
1. Player uses flight console, initiates jump to new destination
2. Any crew/player outside the ship are warned or pulled aboard
3. Ship detaches — destination tiles despawn, ship tiles remain
4. Transit begins

**Deep space:**
- Only ship interior tiles are rendered
- Airlock opens to vacuum — outside is a small "space" zone of vacuum tiles
- Windows render starfield

Camera follows the player whether on ship or in destination — it's all one continuous tile space when docked. AI and physics filter to entities within proximity regardless of which side of the dock they're on.

---

## 4. Navigation & Flight

The flight console is an interactable entity on the ship (walk up, press Space). It opens the navigation overlay.

**Navigation view:**
- List of known locations sorted by distance from current position
- Each entry: name/type, distance, fuel cost, brief descriptor
- Selecting a destination shows a jump preview — minimap rendering of the target location's tiles

**Jump (20 seconds):**
- Ship detaches, enters transit mode
- During transit: ship interior is active space, windows show warp/starfield effect
- Transit is real-time — player can walk around ship, talk to crew, craft
- When transit completes, ship arrives in deep space at destination coordinates
- Fuel consumed on jump initiation

**Arrival & docking:**
- Player sees destination through windows
- Flight console shows docking options with landing spot previews
- Player selects landing spot, ship docks, zones merge
- Can also choose not to dock (stay in deep space, EVA, or jump elsewhere)

```rust
struct TransitState {
    active: bool,
    elapsed: f32,           // seconds
    duration: f32,          // 20.0
    dest_coords: (i32, i32, i32),
}
```

Locations are discovered via nav data items, station computers, or NPC intel. The starter planet is always known.

---

## 5. Atmosphere & EVA

Atmosphere is a property derived from the location and tile. No atmosphere = entities without EVA suits take damage per sim tick.

```rust
impl Tile {
    fn has_atmosphere(&self, location_type: LocationType) -> bool {
        match location_type {
            LocationType::SpaceStation
            | LocationType::ShipInterior
            | LocationType::PlanetSurface(breathable) if breathable => true,
            LocationType::AsteroidField
            | LocationType::DerelictShip
            | LocationType::DeepSpace
            | LocationType::Ruins
            | LocationType::PlanetSurface(false) => false,
        }
    }
}
```

**EVA suit:** An inventory item or component that prevents vacuum damage. Airlocks are transition points — the airlock chamber has atmosphere, the tile outside does not.

Future refinement: per-room atmosphere in derelicts/ruins (sealed compartments). For now, location-level atmosphere is sufficient.

---

## 6. Space Tiles & Procgen

### New Tile Variants

**Ship interior:** `DeckPlate`, `Bulkhead`, `Window`, `AirlockDoor`

**Space structures:** `StationFloor`, `StationWall`, `DerelictFloor`, `DerelictWall`, `Conduit`

**Natural space:** `AsteroidRock`, `AsteroidFloor`, `Regolith`, `Vacuum`, `IceFloor`, `IceWall`

**Planetary surfaces:** `AlienSoil`, `AlienGrass`, `CrystalGrowth`, `AlienFluid`

`Vacuum` is semantically equivalent to `Air` — open space. The damage comes from the atmosphere check, not the tile itself.

### Location Generation by Type

| Type | Approach |
|------|----------|
| Asteroid field | Cellular automata cave gen (reuses existing underground procgen) |
| Derelict ship | Room placement + corridor carving, prefab chunks |
| Space station | Structured room placement (medbay, cargo, bridge) |
| Planet surface | Noise-based (reuses existing surface procgen) with biome params |
| Ruins | Mix of structured rooms + broken wall generation |
| Deep space | Empty vacuum except for rare drifting objects/derelicts |

Procgen reuses existing primitives (`place_room`, `carve_blob`, `place_corridor`) from `level.rs`.

Biome parameters derived from galaxy seed + coordinates — every location is deterministic.

---

## 7. New Items

Space-themed additions to the `Item` enum (existing items preserved):

- `FuelCanister` — refuels the ship
- `EVASuit` — prevents vacuum damage when equipped or in inventory
- `NavData` — unlocks a new location on the galaxy map
- `LaserPistol`, `PlasmaRifle` — space weapons
- `EnergyCell` — ammo/currency for energy weapons
- `SpacesuitHelmet`, `OxygenTank` — EVA components (future: multi-slot EVA)

Existing crafting/salvage system extends to these items.

---

## 8. Crew Recruitment & NPCs

NPCs can be recruited from locations to become crew members aboard the ship.

**Recruitment:**
- Talk to NPC at station/planet/derelict
- Dialogue may include "join crew" option (gated by reputation, payment, or quest)
- On recruitment: NPC despawns from current location, respawns aboard the ship

**Crew behavior:**
- Wander crew quarters, man stations, idle in common areas
- Some crew provide passive benefits: engineer (fuel efficiency), medic (passive healing on ship)
- Crew can be talked to while aboard
- Crew stay on the ship when docked (unless brought as companion)

```rust
#[derive(Component)]
struct CrewMember {
    role: CrewRole,
    home_ship: Entity,
}

enum CrewRole { Engineer, Medic, Gunner, Passenger }
```

Existing named NPCs (Mira, Chronos, Unit-7, Kong, Guard) are recontextualized as space characters on the starter planet. Their `DialogueTree` system is unchanged.

---

## 9. Starting Experience

Game begins on a small starter planet (the "test zone"). The ship is already docked at a landing spot on the surface. Existing NPCs are spawned near the landing site.

- Player starts aboard the ship, exits via airlock onto planet surface
- NPCs have their existing dialogue trees (later: space-themed rewrites)
- Flight console is functional — player can jump to nearby procgen locations
- Planet has a small outpost with basic supplies (fuel, EVA suit, starter gear)
- This planet is the player's "home" — always known, always reachable

This preserves immediate playability: the existing gameplay loop works on the starter planet, and the new space systems are layered on top.

---

## 10. Coexistence with ZoneWorld

- `ZoneWorld` and its fantasy content remain unchanged
- `main.rs` gets a space entry point that initializes `Galaxy` + `Ship` instead of `ZoneWorld`
- Existing `Level`, `Tile`, `Item`, `FovGrid`, dialogue/NPC systems, combat, gravity, crafting — all reused
- `Tile` and `Item` enums get space variants added (not replacing old ones)
- Procgen in `worldgen.rs` stays; new `galaxy_gen.rs` handles space procgen
- Both worlds are runnable independently

## 11. New / Affected Files

| File | Change |
|------|--------|
| `src/galaxy.rs` | New — `Galaxy`, `Location`, `LocationType`, lazy generation dispatch |
| `src/ship.rs` | New — `Ship`, ship layout builder, docking/undocking logic |
| `src/navigation.rs` | New — navigation UI overlay, jump preview rendering, transit state |
| `src/galaxy_gen.rs` | New — space procgen: asteroids, stations, derelicts, planets, ruins |
| `src/level.rs` | Add space `Tile` variants, atmosphere helpers |
| `src/entities.rs` | Flight console entity, space-themed `Object` constructors |
| `src/main.rs` | Space entry point (alongside existing ZoneWorld setup) |
| `src/crew.rs` | New — `CrewMember` component, recruitment integration, crew AI routines |
