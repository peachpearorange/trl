# Haalka UI Integration Design

**Date:** 2026-04-23
**Status:** Approved

## Overview

Migrate all Text2d-based HUD and overlay UI to Haalka (reactive UI framework for Bevy). Haalka owns the full window layout with a game viewport on the left (~70%) and panel UI on the right/bottom. A single `sync_ui` system bridges Bevy game state to Haalka signals each frame. Game systems remain completely untouched.

## Layout

```
HaalkaApp (root)
+-- Row (main horizontal split)
|   +-- GameViewport (~70% width)     <- Camera2d renders here
|   +-- Column (right sidebar, ~30%)
|       +-- StatsPanel               <- HP bar, stats, z-level, time mode
|       +-- InventoryPanel           <- Item list with counts
|       +-- HoverInfoPanel           <- Tile/entity under cursor
|       +-- MessageLog (flex-grow)   <- Scrollable event log
+-- Column (bottom area)
    +-- StatusBar                    <- Tile name, coords, mode, time
    +-- DialogueOverlay (conditional) <- Skyrim-style dimmed box

Overlays (centered on top of everything):
+-- PausePopup                     <- Esc menu with backdrop
+-- InteractPopup                  <- Interaction options menu
```

## Visual Style

- Background: `#1a1a2e` (dark navy)
- Text: `#e0e0e0` (light gray)
- Borders: `1px solid #333355` (subtle)
- Font: readable sans-serif (system default or embedded)
- No gradients, no textures
- Dialogue/popup backdrop: `rgba(0, 0, 0, 0.5)` semi-transparent dim

## Signal Architecture

Six signal resources, all written by a single `sync_ui` system:

| Signal Resource | Data Shape | Source |
|----------------|------------|--------|
| `ClockSignal` | `{ mode: "RT"|"TB", tick: f32 }` | `Res<Clock>` |
| `PlayerSignal` | `{ hp, max_hp, attack, speed, x, y, z }` | `Query<Player>` |
| `InvSignal` | `Vec<(Item, u32)>` | Player `Inventory` component |
| `HoverSignal` | `Option<{ tile_name, entity_name?, hp_bar?, flavor? }>` | Mouse raycast + `TileEntityIndex` |
| `DialogueSignal` | `Option<{ speaker, text, choices: Vec<...> }>` | `UiState.dialogue` |
| `OverlaySignal` | `enum { None, Pause(PauseMenu), Interact(Vec<InteractionOption>) }` | `UiState.pause/.interact` |
| `LogSignal` | `Vec<MessageEntry>` (append-only, capped at ~100) | Accumulated in sync_ui |

### sync_ui System

Single system that reads Bevy world state each frame and pushes to signals. Runs in Update schedule after game logic systems. Game systems have zero knowledge of UI.

```rust
fn sync_ui(
    clock: Res<Clock>,
    player: Query<(&PlayerPos, &Stats, &Inventory), With<Player>>,
    ui: Res<UiState>,
    fov: Res<Fov>,
    gw: Res<GameWorld>,
    index: Res<TileEntityIndex>,
    named: Query<(&Named, Option<&Stats>)>,
    windows: Query<&Window>,
    camera: Query<(&Camera, &GlobalTransform), With<Camera2d>>,
    // Signal resources:
    clock_sig: Res<ClockSignal>,
    player_sig: Res<PlayerSignal>,
    inv_sig: Res<InvSignal>,
    hover_sig: Res<HoverSignal>,
    dialogue_sig: Res<DialogueSignal>,
    overlay_sig: Res<OverlaySignal>,
    log_sig: Res<LogSignal>,
)
```

## File Organization

### New: src/ui.rs

Contains:
- Small data structs for signal shapes (`ClockData`, `PlayerData`, `HoverInfo`, `MessageEntry`)
- Six signal resource types (thin wrappers around `Mutable<T>`)
- Plugin/setup function that creates signal resources and builds Haalka tree
- `build_ui(root)` — constructs full layout tree with reactive bindings
- Panel sub-functions: `stats_panel()`, `inventory_panel()`, `hover_panel()`, `message_log()`, `status_bar()`
- Overlay functions: `dialogue_overlay()`, `pause_popup()`, `interact_popup()`

Estimated: 350-500 lines.

### Changes to src/main.rs

**Remove:**
- Component defs: `TimeDisplay`, `LevelDisplay`, `InventoryDisplay`, `TileInfoDisplay`, `HudElement`
- HUD spawning in `setup()` — 4 commands.spawn blocks for time/level/tile-info/inventory
- `update_hud()` system function
- `mouse_hover_tile()` system function
- `tile_hover_text()` helper
- Overlay spawn/despawn helpers: `spawn_pause_overlay()`, `despawn_overlays()`, `despawn_interact_overlays()`
- Dialogue overlay spawning inside `handle_dialogue()`
- Interact overlay spawning inside `show_interact_menu()`

**Add:**
- `mod ui;` at top
- `ui::UiPlugin` to `.add_plugins()` in main
- Remove HUD systems from Update schedule chain

**Keep unchanged:**
- All game logic systems: player_input, apply_gravity, enemy_ai, camera_follow, handle_menus, handle_dialogue, handle_interact, update_fov_visuals, etc.
- Entity spawning (player, enemies, NPCs, trees, tiles)

## Panel Details

### Stats Panel
- HP bar: width = `(hp / max_hp * 100)%`, color green (>66%) -> yellow (33-66%) -> red (<33%)
- Attack, speed as label pairs
- Z-level name + zone coords
- Time mode indicator ("Real Time" / "Turn Based")

### Inventory Panel
- Each item: "Nx ItemName" line
- Shows "(empty)" when no items

### Hover Info Panel
- Tile coordinates and name
- If entity present: name, HP bar, flavor text
- Empty/hyphenated when nothing under cursor

### Message Log
- Scrollable area
- Newest messages at bottom
- Capped to ~100 entries (drop oldest)
- Format: event description string

### Status Bar
- Single-line: tile name | coordinates | mode | tick count
- Thin height (~24px)

### Dialogue Overlay (Skyrim-style)
- Anchored bottom-center, above status bar
- Semi-transparent dark background with subtle border
- Speaker name, separator, dialogue text
- Numbered choices list
- Only visible when `DialogueSignal` is `Some(...)`

### Pause Popup
- Centered on screen
- Dimmed backdrop behind it
- Title + numbered options
- Closes on Esc or selection

### Interact Popup
- Centered slightly above screen center
- "Use what?" header + numbered interaction options
- Esc to cancel

## Migration Order

1. Create `src/ui.rs` with signal resource types and empty plugin
2. Add `ui::UiPlugin` to app, verify it compiles/launches
3. Build layout tree skeleton (viewport area + empty panels)
4. Wire `sync_ui` system with ClockSignal and PlayerSignal
5. Build stats_panel with live data
6. Build inventory_panel
7. Build hover info panel (requires mouse raycast in sync_ui)
8. Build message log (needs message accumulation logic)
9. Build status bar
10. Replace dialogue Text2d overlay with Haalka dialogue_overlay
11. Replace pause menu Text2d overlay with Haalka pause_popup
12. Replace interact menu Text2d overlay with Haalka interact_popup
13. Clean up removed code from main.rs
14. Polish: borders, spacing, colors, font sizing
