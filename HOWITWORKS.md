# Project Context for Future Agents

This file captures how this repository is organized and how the runtime works, so future tasks can skip repeated discovery.

Future agents: when you discover non-obvious behavior, ordering constraints, hidden coupling, or any tricky "gotcha" about how this codebase works, add it to this file in the relevant section (or create a short new section). Keep this document updated as a living map.

## What This Repo Is

- Rust + Bevy game project with two binaries:
  - `trl` (main game) in `src/main.rs`
  - `editor` (map/content tooling) in `src/editor.rs`
- Core gameplay uses ECS systems in `src/main.rs` plus subsystem modules.

## Entrypoints

- `src/main.rs`
  - Builds initial galaxy/location state
  - Inserts core resources (`Galaxy`, `Ship`, `CurrentZone`, UI/resources)
  - Registers plugins (`UiPlugin`, `ParticlesPlugin`, `PostProcessPlugin`, `OutlinePlugin`)
  - Spawns world/player/cameras
  - Runs per-frame simulation + rendering systems
- `src/editor.rs`
  - Standalone level/object editor and generation tooling path

## Runtime Model (High Level)

- Authoritative world content is stored in `Galaxy` (`src/galaxy.rs`).
- Play happens in a merged active zone (`CurrentZone(ActiveZone)`) from `src/active_zone.rs`.
- Entity tile occupancy is indexed each frame in `TileEntityIndex` (`src/combat.rs`) and reused by AI + interactions.
- Visibility is maintained through `Fov` (`src/level.rs` + update systems in `src/main.rs`).
- UI is resource-driven:
  - gameplay systems update state/resources/log
  - `src/ui.rs` sync systems mirror that into display resources for Haalka UI

## Main Gameplay Loop

Most orchestration lives in `src/main.rs`:

- Input and movement (`accumulate_dir`, `player_input`)
- Interaction resolution (`resolve_bump_interact`, `apply_bump_auto_interact`)
- Menus/dialogue/crafting handlers
- Deferred actions application:
  - navigation
  - chest/loot flushing
  - bed save application
- Simulation scheduling via frame/tick resources (`RenderFrame`, `Clock`, `TurnBasedWorldState`) and sim-step sets

## Major Subsystems and Where They Live

- Entity/component model + object constructors: `src/entities.rs`
- Combat, AI, pathing, status ticking: `src/combat.rs`
- Abilities/loadout targeting/cooldowns: `src/abilities.rs`
- Path/trajectory overlay: `src/path_overlay.rs`
- UI plugin and display sync: `src/ui.rs`
- Tile/sprite/palette/render metadata:
  - `src/tiles.rs`
  - `src/sprites.rs`
- Prefabs and stamping helpers: `src/prefabs.rs`
- Loot + crafting:
  - `src/loot.rs`
  - `src/crafting.rs`
- Docking and ship:
  - `src/docking.rs`
  - `src/ship.rs`
- Visual effects/render layers:
  - particles: `src/particles.rs`
  - outlines: `src/outline.rs`
  - post-process pipeline: `src/post_process.rs`

## Content Generation and Locations

- Procedural:
  - planets: `src/locations/planet_gen.rs`
  - stations: `src/locations/station_gen.rs`
- Hand-authored locations:
  - `src/locations/starter_planet.rs`
  - `src/locations/meridian_station.rs`
  - `src/locations/gamma_station.rs`
  - `src/locations/mushroom_planet.rs`
  - `src/locations/lava_planet.rs`
  - `src/locations/asteroid_field.rs`
- Reusable NPC definitions/dialogue:
  - `src/npcs/*.rs`

## Data Flow Notes That Save Time

- `Location` components are the authoritative tile positions for entities.
- `TileEntityIndex` is rebuilt from those positions and should be consulted when adding AI/interaction logic.
- Deferred generation is used for some locations; `Galaxy` can materialize those on demand when docking/navigation occurs.
- Ability UI and targeting are resource-driven (`TargetingState`, ability slot resources) rather than direct UI mutation.
- Zone navigation (`apply_pending_navigation`) **despawns every tilemap chunk + entity and respawns fresh ones**, and resets `Fov`. Any cached per-tile *visual* state that persists across frames (e.g. the FOV fade `brightness` grid in `update_fov_visuals`) must key off the live chunk `Entity` and reset when it changes — otherwise stale state plus a "skip if unchanged" optimization leaves the new empty chunk unwritten (tiles render black while entities show, since entities aren't chunk-based and the player isn't FOV-gated).
- `update_fov` rebuilds `Fov` from scratch *every* frame (in the ungated `EveryFrame` set) from the level's opaque tiles plus all `BlocksSight` entities (closed doors). It mutates the resource unconditionally, so `Fov.is_changed()` is effectively always true — don't gate visuals on it.

## Practical "Where To Edit" Guide

- Add a new player action or interact behavior: `src/main.rs`, `src/entities.rs`, maybe `src/ui.rs`
- Add a new enemy behavior: `src/combat.rs` (+ object definition in `src/entities.rs`)
- Add a new item/loot/craft chain: `src/level.rs`, `src/loot.rs`, `src/crafting.rs`, `src/entities.rs`
- Add a new location type:
  - handcrafted: `src/locations/*.rs` + registration in startup path
  - procedural: `src/locations/planet_gen.rs` or `src/locations/station_gen.rs`
- Add visual effects/presentation:
  - particles/outlines/post: corresponding module above
  - tile art/palette behavior: `src/sprites.rs` and `src/tiles.rs`

## Files Worth Treating Carefully

- `src/main.rs`: central orchestration and system ordering; small ordering changes can have large side effects.
- `src/entities.rs`: many constructors and shared gameplay definitions.
- `src/combat.rs`: dense AI/combat logic with multiple systems depending on shared resources.
- `src/locations/planet_gen.rs`: large procedural pipeline, easy to introduce regressions if touched broadly.

