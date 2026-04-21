# Zone World Design

**Date**: 2026-04-21  
**Status**: Approved

## Overview

Replace the current single fixed-size `World` with a `ZoneWorld`: a 10×10×4 grid of `Zone`s, each
48×48 tiles. The world is a continuous noise-generated space; zones are purely a rendering and
simulation window into it. The player walks seamlessly between zones. A world map shows the full
terrain as a generated image.

---

## 1. Data Model

```
ZoneWorld {
    zones: Vec<Vec<Vec<Zone>>>,   // indexed [zx][zy][z]
    width:  10,                   // zones horizontally
    height: 10,                   // zones vertically
    depth:   4,                   // z-levels: 3=surface, 2/1/0=underground
}

Zone = current Level — 48×48 tiles + 48×48 item grid
```

`PlayerPos` gains zone coordinates:

```rust
struct PlayerPos { x: i32, y: i32, zx: usize, zy: usize, z: usize }
```

`CurrentZ` is removed; z is read from `PlayerPos`. Systems that need to know the active zone query
the player entity directly.

The zone grid is a technical loading unit only — it has no effect on how content is generated.

---

## 2. World Generation Pipeline

Run once at startup from a fixed `WORLD_SEED: u64`. All generation is deterministic.

### 2.1 Continuous Noise

Tile types are determined by sampling Perlin/simplex noise at **world-space coordinates**:

```
wx = zx * 48 + tx
wy = zy * 48 + ty
```

The same noise function is evaluated for every tile regardless of which zone it sits in, producing
smooth transitions across zone boundaries.

### 2.2 Island Mask

A radial distance falloff from world center `(240, 240)` is multiplied into the noise value. Tiles
far from center become Ocean/DeepWater. The world is a single island surrounded by sea.

### 2.3 Surface Tile Assignment

Noise value → tile type mapping (approximate thresholds, tunable):

| Noise range | Tile(s) |
|-------------|---------|
| Very low    | DeepWater, ShallowWater |
| Low         | Sand, Beach |
| Low-mid     | Grass, TallGrass |
| Mid         | Forest (Tree entities + Grass floor) |
| Mid-high    | Ash, Bush |
| High        | Lava, Ash (hot region near center) |

A second noise octave adds local variation: scattered boulders (entities), cave entrances, water
pools.

### 2.4 Underground Generation

Underground levels (z=0,1,2) use cellular automata or worm-carver algorithms run across the full
underground world space. Cave systems are continuous and cross zone boundaries freely — there is no
per-zone cave generation. The surface biome above influences cave character loosely (e.g. wetter
surface → more flooded sections underground).

### 2.5 Town Placement

The generator identifies 3–5 areas of flat, walkable surface tiles suitable for a settlement.
Town layouts are procedurally generated: a road grid, building footprints (WoodWall/WoodFloor),
fences, and open plazas. Towns are carved into the world tile data after the noise pass.

### 2.6 Manual NPC Layer

After all worldgen is complete, `world_data.rs` injects named NPCs at specific world coordinates:

```rust
fn world_npcs() -> Vec<NpcPlacement> {
    vec![
        NpcPlacement { zx: 3, zy: 4, x: 12, y: 8, object: Object::mira() },
        // ...
    ]
}
```

Towns receive named NPCs with authored `DialogueTree`s (defined in `dialogue.rs`) via this layer.
Generic inhabitants (guards, villagers, merchants) come from NPC templates defined in `entities.rs`
and are scattered through towns procedurally — they have hover flavor text but no dialogue trees.

---

## 3. Zone Transitions

When the player moves and their tile position goes out of bounds (x < 0, x ≥ 48, y < 0, y ≥ 48):

1. `zx` or `zy` increments/decrements by 1
2. Tile position wraps to the opposite edge (walk off east at x=48 → arrive at x=0)
3. Current zone's tile entities despawn; new zone's tile entities spawn
4. FOV recomputes from the new tile position
5. All non-tile entities persist as Bevy entities — no despawning

Ocean zones at the world boundary are impassable (DeepWater tiles). The player cannot walk off
the world edge.

Because tile types come from continuous noise, the tile at zone edge (x=47) and the tile at the
adjacent zone's edge (x=0) are generated from adjacent world coordinates and match naturally — no
seam stitching required.

### Entity Simulation

All entities always exist as Bevy entities. Systems that simulate behavior (enemy AI, gravity) filter
by current zone — entities in other zones are skipped. `update_entity_visibility` already does this;
AI and physics follow the same pattern.

---

## 4. World Map

Press `M` to open, `M` or `Escape` to close.

- A `bevy::render::texture::Image` is generated at startup by sampling the world tile data across
  all surface zones. Each pixel corresponds to one tile (480×480) or a fixed downsample.
- Each pixel is colored by tile type (water=blue, grass=green, lava=red, sand=tan, ash=grey, etc.)
- Rendered as a fullscreen `Sprite` overlay (z=50, above all game content)
- Zone grid lines optionally overlaid as faint dark lines
- Mouse hover draws a rectangle around the hovered zone cell
- Player's current zone is highlighted with a distinct border color

The map is generated once at startup and cached. It does not update during play (tile changes like
opening doors do not affect the map image).

---

## 5. New Tile Types

Added to the `Tile` enum:

**Terrain**
- `TallGrass` — passable, not opaque
- `Bush` — impassable, not opaque
- `Ash` — passable, not opaque
- `Lava` — passable but damages entities each turn
- `ShallowWater` — passable
- `DeepWater` — impassable
- `Road` — passable (settlement floors)

**Construction**
- `WoodWall` — impassable, opaque
- `WoodFloor` — passable
- `Fence` — impassable, not opaque

**Underground**
- `CaveWall` — impassable, opaque
- `CaveFloor` — passable
- `CrystalFormation` — impassable, not opaque (glows faintly)

---

## 6. Interactable Vegetation & Objects

**Entities** (interactable, spawned with `Location` component):
- `Tree` — impassable; can be chopped (future interaction)
- `Boulder` — impassable; can be pushed or mined (future interaction)

These appear in the `TileEntityIndex` and show up in hover info and the interaction menu.

**Tiles** (non-interactable, dense ground cover):
- `TallGrass`, `Bush`, `Ash` — placed by noise, no interaction

---

## 7. Files Affected / New Files

| File | Change |
|------|--------|
| `src/level.rs` | `Level` → `Zone`; new `ZoneWorld`; worldgen pipeline; new `Tile` variants |
| `src/main.rs` | `PlayerPos` gains `zx/zy/z`; zone transition logic; world map overlay |
| `src/entities.rs` | `Tree`, `Boulder` object definitions; NPC template types |
| `src/dialogue.rs` | New NPC dialogue trees |
| `src/world_data.rs` | New file — `NpcPlacement` list, post-procgen injection |
| `src/map.rs` | Replace existing unused generation code with world map image generation and rendering |

The existing `combat.rs`, `tile_loader.rs`, and `dialogue.rs` systems need minimal changes — they
operate on tile/entity data that remains structurally the same.
