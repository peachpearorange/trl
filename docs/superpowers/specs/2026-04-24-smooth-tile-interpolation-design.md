# Smooth Tile Interpolation Design

**Date:** 2026-04-24
**Status:** Approved

## Goal

Add linear visual interpolation between tile positions for all moving entities (player, enemies, falling objects). Entities glide smoothly across tiles instead of teleporting. The camera locks to the player's interpolated position so the world scrolls smoothly while the player stays screen-fixed.

## Data Model

### `Visuals` Component (replaces `GlyphVisual` marker)

```rust
#[derive(Component)]
pub struct Visuals {
    pub prev: Vec2,           // Tile position before the most recent move
    pub last_move_time: f32,  // Clock.time when Location last changed
    pub display: Vec2,        // Current interpolated visual position (recomputed each frame)
}
```

- Spawned on entities that should interpolate: **Player**, **Enemy**, any entity with **`Gravity`**
- On spawn: `prev = current Location as Vec2`, `last_move_time = clock.time` (or set far in past for instant appearance)
- Static entities (tiles, non-moving items) get no `Visuals` component — zero overhead

### No New Resources or Constants

Lerp speed is implicit: one tile distance per `MOVE_COOLDOWN` duration (0.12s). At `TILE_SIZE = 32.0`, effective speed is ~267 px/sec but this is never stored as a constant.

## Systems

### `interpolate_visual_positions` (NEW)

Runs every frame before `sync_entity_positions`. For each entity with both `Location` and `Visuals`:

```
current = Vec2(location.x as f32, location.y as f32)
elapsed = max(0, clock.time - visuals.last_move_time)
progress = min(1.0, elapsed / MOVE_COOLDOWN)
visual_position = visuals.prev.lerp(current, progress)
```

Pure weighted average. No chasing, no speed constant. Uses wall-clock time (`Clock.time`) for interpolation in both real-time and turn-based modes — turn-based actions still animate smoothly over real time then freeze until next action.

Result written to `Visuals.display` each frame for consumption by `sync_entity_positions`.

### Movement System Integration

When a movement system (`player_input`, `enemy_ai`, `apply_gravity`) writes a new `Location` to an entity:

```
if entity has Visuals:
    visuals.prev = old_location_as_vec2  // or current interpolated display pos for direction changes
    visuals.last_move_time = clock.time
```

Two strategies for this:
1. **Explicit:** Each movement system updates `Visuals` after changing `Location`
2. **Implicit (preferred):** A diffing system detects `Location` changes against cached `Visuals.prev` each frame and updates automatically. Movement systems remain unaware of visuals.

### `sync_entity_positions` (MODIFIED)

Currently reads `Location` -> writes `Transform`. Now checks for `Visuals` first:

```
if entity has Visuals:
    transform.translation = world_from_visual_pos(visuals.display)
else:
    transform.translation = world_from_location(location)  // existing snap behavior
```

### `camera_follow` (MODIFIED)

Currently: lerp-factor 0.15 toward player's snapped Transform.
New: exact pixel lock to player's interpolated `Visuals` display position.

```
visual_pos = player_visuals.display  // from interpolate_visual_positions
camera.transform.translation = camera_target_from(visual_pos)  // exact lock, no lerp
```

Camera-side lerp is unnecessary because `Visuals` IS the smoothed value.

### Tile Rendering (UNCHANGED)

Tiles are static grid elements without `Visuals`. They continue snapping to integer coordinates. Smooth world-scrolling comes entirely from camera tracking the player's continuous visual position.

## System Order (Updated)

```
maintain_tile_index
setup_glyph_visuals
interpolate_visual_positions   <- NEW: compute Visuals.display from prev/current + progress
sync_entity_positions          <- now reads Visuals.display when available
update_entity_visibility
advance_realtime
update_time_mode
handle_world_map
handle_menus
handle_dialogue
handle_interact
player_input                  <- may update Visuals.prev/last_move_time
ApplyDeferred
apply_gravity                 <- may update Visuals.prev/last_move_time
enemy_ai                      <- may update Visuals.prev/last_move_time
camera_follow                 <- locks to player's Visuals display pos
update_fov_visuals
```

## Edge Cases

### Direction Change Mid-Interpolation

Player moving east, visual at 60% toward next tile (~5.6, 3.0). Player taps north:

1. `prev` set to current interpolated display position (~5.6, 3.0) — NOT the tile center
2. `last_move_time` reset to `clock.time`
3. Entity pivots from its current screen position, starts sliding north

No snap-to-tile-center. The pivot happens from wherever the entity visually is.

### Continuous Straight-Line Movement

Player holds east, moves every 0.12s cooldown:

- t=0.00: move fires, visual at old tile, starts sliding
- t=0.12: visual arrives exactly at new tile, next move fires immediately
- Result: constant velocity, no stop-start jitter at tile boundaries

The interpolation naturally completes as the next move arrives because the visual travel time equals the move cooldown.

### Zone Transitions

Player crosses zone boundary: tiles despawn/respawn at snapped positions. Player's `Location` jumps to new zone coords, `Visuals` resets. Brief moment of static tiles + interpolating player during transition — acceptable since zone transitions are already visual "cuts."

### Entity Lifecycle

- **Spawn:** `prev = current Location as Vec2`, `last_move_time` set far in past (progress clamps to 1.0). Entity appears at its tile instantly.
- **Death/Despawn:** Irrelevant — entity removed.

### Turn-Based Mode

Uses wall-clock time for interpolation. Each turn-based action animates smoothly over ~0.12s of real time, then visuals freeze until next action. Consistent feel across both modes.

## What Stays Unchanged

- All collision detection (reads `Location` integers only)
- Enemy AI pathfinding (reads `Location`)
- FOV computation (reads `PlayerPos` / `Location`)
- Gravity system (writes `Location`)
- Spatial index `TileEntityIndex` (indexed by `Location`)
- Turn/time costing logic
- HUD and UI rendering (z=5, camera-child entities)
