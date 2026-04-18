# Content System Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add Named/Stats/equipment components, creature constructor fns (rat soldier, catgirl NPC), a tile-entity spatial index, SS13-style hover tooltip, and bump combat.

**Architecture:** New components live in `src/entities.rs`. A new `src/combat.rs` module (declared in `main.rs`) holds the `TileEntityIndex` resource, enemy AI, and damage logic. Entity visuals use Bevy's `Text2d` added by a system watching `Added<Glyph>`. `main.rs` connects everything via system registration and enemy spawning in `setup`.

**Tech Stack:** Bevy 0.18, Rust 2024 edition. Binary crate (`main.rs`) uses library crate (`trl::entities`, `trl::tile_loader`) via `use trl::...`.

---

## File Map

| File | Role |
|------|------|
| `src/entities.rs` | Add components, creature fns, `Spawnable::npc()`, `Spawnable::spawn_at()` |
| `src/combat.rs` | New: `TileEntityIndex`, `TimeSinceAction`, damage fn, enemy AI system |
| `src/main.rs` | `mod combat`, import entities, register resources/systems, spawn enemies, update hover/input |

---

## Task 1: New Components in entities.rs

**Files:**
- Modify: `src/entities.rs`

- [ ] **Add `Glyph`, `Named`, `Stats`, `Armor`, `Wielding`, `Wearing` and `Item::Spear` to entities.rs.**

  Replace the existing `Item` enum and add all new types. The full additions after the existing `Material` enum:

  ```rust
  // In the VALUE TYPES section, extend Item:
  #[derive(Clone, Copy, Debug, PartialEq, Eq)]
  pub enum Item {
    Sword,
    Coin,
    Potion,
    Key,
    Spear,
  }

  #[derive(Clone, Copy, Debug, PartialEq, Eq)]
  pub enum Armor {
    Leather,
    Chain,
    Plate,
  }

  impl Armor {
    pub fn dr(self) -> i32 {
      match self {
        Armor::Leather => 1,
        Armor::Chain => 2,
        Armor::Plate => 3,
      }
    }
  }
  ```

  Add these to the COMPONENTS section:

  ```rust
  /// ASCII glyph visual: char + RGB color for Text2d rendering.
  #[derive(Component, Clone)]
  pub struct Glyph {
    pub ch: char,
    pub color: [f32; 3],
  }

  /// Identity and SS13-style flavor text shown on hover.
  #[derive(Component)]
  pub struct Named {
    pub name:   &'static str,
    pub flavor: &'static str,
  }

  /// Flat combat stats.
  #[derive(Component)]
  pub struct Stats {
    pub hp:           i32,
    pub max_hp:       i32,
    pub attack:       i32,
    pub move_speed:   f32,
    pub attack_speed: f32,
  }

  /// What an entity is holding. None = unarmed (has hands, holds nothing).
  #[derive(Component)]
  pub struct Wielding(pub Option<Item>);

  /// Armor being worn. None = unarmored.
  #[derive(Component)]
  pub struct Wearing(pub Option<Armor>);

  /// Tracks time since the entity last acted (seconds). Used by enemy AI.
  #[derive(Component)]
  pub struct TimeSinceAction(pub f32);
  ```

- [ ] **Run `cargo check` and confirm it compiles.**

  ```
  cargo check
  ```

  Expected: no errors (some dead_code warnings are fine).

- [ ] **Commit.**

  ```bash
  git add src/entities.rs
  git commit -m "feat: add Named, Stats, Glyph, Wielding, Wearing, Armor, TimeSinceAction components"
  ```

---

## Task 2: Spawnable::npc(), spawn_at(), and Creature Definitions

**Files:**
- Modify: `src/entities.rs`

- [ ] **Add `Spawnable::npc()` and `Spawnable::spawn_at()` to the impl block.**

  In `src/entities.rs`, extend the `impl Spawnable` block:

  ```rust
  /// NPC base: Neutral faction, non-blocking.
  pub fn npc() -> Self {
    Self::new((Character, FactionComp(Faction::Neutral)))
  }

  /// Spawn this entity at tile coordinates, inserting Location::Coords.
  pub fn spawn_at(self, commands: &mut Commands, x: i32, y: i32) -> Entity {
    let mut e = commands.spawn_empty();
    (self.0)(&mut e);
    e.insert(Location::Coords { x, y });
    e.id()
  }
  ```

- [ ] **Add creature constructor functions to `impl Spawnable`.**

  ```rust
  pub fn rat_soldier() -> Self {
    Self::enemy()
      .add((
        Named {
          name: "Rat Soldier",
          flavor: "A wiry rat-person clutching a crude spear. Smells like wet fur and old iron.",
        },
        Stats { hp: 10, max_hp: 10, attack: 3, move_speed: 3.0, attack_speed: 1.0 },
        Wielding(Some(Item::Spear)),
        Wearing(None),
        Glyph { ch: 'r', color: [0.9, 0.6, 0.4] },
        TimeSinceAction(0.0),
      ))
  }

  pub fn armored_rat_soldier() -> Self {
    Self::enemy()
      .add((
        Named {
          name: "Rat Soldier",
          flavor: "A wiry rat-person clutching a crude spear. Smells like wet fur and old iron.",
        },
        Stats { hp: 10, max_hp: 10, attack: 3, move_speed: 3.0, attack_speed: 1.0 },
        Wielding(Some(Item::Spear)),
        Wearing(Some(Armor::Leather)),
        Glyph { ch: 'r', color: [0.7, 0.5, 0.3] },
        TimeSinceAction(0.0),
      ))
  }

  pub fn catgirl() -> Self {
    Self::npc()
      .add((
        Named {
          name: "Catgirl",
          flavor: "She eyes you warily, ears flat against her head.",
        },
        Stats { hp: 8, max_hp: 8, attack: 2, move_speed: 4.0, attack_speed: 1.2 },
        Wielding(None),
        Wearing(None),
        Glyph { ch: 'c', color: [0.9, 0.7, 0.9] },
        TimeSinceAction(0.0),
      ))
  }
  ```

- [ ] **Run `cargo check`.**

  ```
  cargo check
  ```

  Expected: no errors.

- [ ] **Commit.**

  ```bash
  git add src/entities.rs
  git commit -m "feat: add Spawnable::npc, spawn_at, rat_soldier, armored_rat_soldier, catgirl"
  ```

---

## Task 3: combat.rs — TileEntityIndex and Damage

**Files:**
- Create: `src/combat.rs`
- Modify: `src/main.rs` (add `mod combat`, import, register resource + system)

- [ ] **Create `src/combat.rs` with `TileEntityIndex` and damage logic.**

  ```rust
  // src/combat.rs
  use {
    bevy::prelude::*,
    std::collections::HashMap,
    trl::entities::{Location, Wearing, Armor},
  };

  // ---------------------------------------------------------------------------
  // Tile-entity spatial index
  // ---------------------------------------------------------------------------

  /// Maps tile coords to entities currently at that position.
  /// Rebuilt from scratch each frame — simple, always correct.
  #[derive(Resource, Default)]
  pub struct TileEntityIndex(pub HashMap<(i32, i32), Vec<Entity>>);

  pub fn maintain_tile_index(
    mut index: ResMut<TileEntityIndex>,
    query: Query<(Entity, &Location)>,
  ) {
    index.0.clear();
    for (entity, location) in query.iter() {
      if let Location::Coords { x, y } = location {
        index.0.entry((*x, *y)).or_default().push(entity);
      }
    }
  }

  // ---------------------------------------------------------------------------
  // Damage calculation
  // ---------------------------------------------------------------------------

  /// Compute damage dealt to a target, accounting for armor DR.
  pub fn resolve_damage(attack: i32, wearing: Option<&Wearing>) -> i32 {
    let dr = wearing
      .and_then(|w| w.0)
      .map(|armor| armor.dr())
      .unwrap_or(0);
    (attack - dr).max(0)
  }

  #[cfg(test)]
  mod tests {
    use super::*;
    use trl::entities::{Wearing, Armor};

    #[test]
    fn no_armor_deals_full_damage() {
      assert_eq!(resolve_damage(5, None), 5);
    }

    #[test]
    fn armor_reduces_damage() {
      let wearing = Wearing(Some(Armor::Leather)); // DR 1
      assert_eq!(resolve_damage(5, Some(&wearing)), 4);
    }

    #[test]
    fn armor_cannot_go_below_zero() {
      let wearing = Wearing(Some(Armor::Plate)); // DR 3
      assert_eq!(resolve_damage(2, Some(&wearing)), 0);
    }

    #[test]
    fn chain_armor_dr() {
      let wearing = Wearing(Some(Armor::Chain)); // DR 2
      assert_eq!(resolve_damage(4, Some(&wearing)), 2);
    }
  }
  ```

- [ ] **Run the tests to confirm they pass.**

  ```
  cargo test combat
  ```

  Expected output:
  ```
  test combat::tests::no_armor_deals_full_damage ... ok
  test combat::tests::armor_reduces_damage ... ok
  test combat::tests::armor_cannot_go_below_zero ... ok
  test combat::tests::chain_armor_dr ... ok
  ```

- [ ] **Add `mod combat` to `main.rs` and register the resource and system.**

  At the top of `src/main.rs`, add the module declaration after the existing `mod level;`:

  ```rust
  mod combat;
  ```

  Add to the `use` block at the top of `main.rs`:

  ```rust
  use {
    bevy::prelude::*,
    combat::{TileEntityIndex, maintain_tile_index},
    level::{FovGrid, Tile, World, build_test_world, compute_fov},
    trl::entities::{
      Enemy, FactionComp, Glyph, Location, Named, Spawnable, Stats, Wearing,
    },
    trl::tile_loader::Faction,
  };
  ```

  In `main()`, add the resource and system. After `.insert_resource(Fov(fov))`:

  ```rust
  .insert_resource(TileEntityIndex::default())
  ```

  In the `add_systems(Update, (...).chain())` call, add `maintain_tile_index` at the start of the chain (before other systems so the index is fresh):

  ```rust
  .add_systems(
    Update,
    (
      maintain_tile_index,
      advance_realtime,
      handle_menus,
      player_input,
      camera_follow,
      update_fov_visuals,
      mouse_hover_tile,
      update_hud,
    )
      .chain()
  )
  ```

- [ ] **Run `cargo check`.**

  ```
  cargo check
  ```

  Expected: no errors.

- [ ] **Commit.**

  ```bash
  git add src/combat.rs src/main.rs
  git commit -m "feat: add TileEntityIndex, damage resolution, wire combat module"
  ```

---

## Task 4: Entity Visuals — Glyph Rendering and Position Sync

**Files:**
- Modify: `src/main.rs`

- [ ] **Add the `EnemyGlyph` marker component and two rendering systems to `main.rs`.**

  Add this component near the other component definitions in `main.rs`:

  ```rust
  /// Marker for entities that have had their Text2d visual set up.
  #[derive(Component)]
  struct GlyphVisual;
  ```

  Add these two systems anywhere in `main.rs` before the `main()` fn:

  ```rust
  /// Watch for newly-added Glyph+Location entities and insert Text2d visuals.
  fn setup_glyph_visuals(
    mut commands: Commands,
    gw: Res<GameWorld>,
    query: Query<(Entity, &Glyph, &Location), (Added<Glyph>, Without<GlyphVisual>)>,
  ) {
    for (entity, glyph, location) in query.iter() {
      if let Location::Coords { x, y } = location {
        let pos = tile_screen_pos(*x as usize, *y as usize, gw.0.width, gw.0.height)
          + Vec3::new(0.0, 0.0, 2.0);
        commands.entity(entity).insert((
          Text2d::new(glyph.ch.to_string()),
          TextFont { font_size: TILE_SIZE, ..default() },
          TextColor(Color::srgb(glyph.color[0], glyph.color[1], glyph.color[2])),
          Transform::from_translation(pos),
          GlyphVisual,
        ));
      }
    }
  }

  /// Keep entity Transform in sync when Location::Coords changes.
  fn sync_entity_positions(
    gw: Res<GameWorld>,
    mut query: Query<(&Location, &mut Transform), (With<GlyphVisual>, Changed<Location>)>,
  ) {
    for (location, mut transform) in query.iter_mut() {
      if let Location::Coords { x, y } = location {
        transform.translation = tile_screen_pos(*x as usize, *y as usize, gw.0.width, gw.0.height)
          + Vec3::new(0.0, 0.0, 2.0);
      }
    }
  }
  ```

- [ ] **Register both systems in the `Update` chain in `main()`.**

  Update the chain to include the new systems after `maintain_tile_index`:

  ```rust
  .add_systems(
    Update,
    (
      maintain_tile_index,
      setup_glyph_visuals,
      sync_entity_positions,
      advance_realtime,
      handle_menus,
      player_input,
      camera_follow,
      update_fov_visuals,
      mouse_hover_tile,
      update_hud,
    )
      .chain()
  )
  ```

- [ ] **Spawn enemies and NPCs in `setup()`.**

  After `compute_fov(...)` in the `setup` fn, add enemy/NPC spawning. Pick a few walkable tiles near the player start position:

  ```rust
  // Spawn some enemies and NPCs for testing
  let (ex1, ey1) = find_walkable(level, px + 5, py);
  let (ex2, ey2) = find_walkable(level, px + 3, py + 4);
  let (cx1, cy1) = find_walkable(level, px - 4, py + 2);

  Spawnable::rat_soldier().spawn_at(&mut commands, ex1, ey1);
  Spawnable::armored_rat_soldier().spawn_at(&mut commands, ex2, ey2);
  Spawnable::catgirl().spawn_at(&mut commands, cx1, cy1);
  ```

  Also add `Stats` to the player entity in setup so it has HP:

  ```rust
  commands.spawn((
    Text2d::new("@"),
    TextFont { font_size: TILE_SIZE, ..default() },
    TextColor(Color::srgb(1.0, 1.0, 0.0)),
    Transform::from_translation(
      tile_screen_pos(px as usize, py as usize, gw.0.width, gw.0.height) + Vec3::Z
    ),
    Player,
    PlayerPos { x: px, y: py },
    Stats { hp: 20, max_hp: 20, attack: 5, move_speed: 3.0, attack_speed: 1.0 },
  ));
  ```

- [ ] **Run the game and verify enemies appear as glyphs on the map.**

  ```
  cargo run
  ```

  Expected: `r` glyphs (orange-ish) and a `c` glyph (pink) visible on the map near the player start.

- [ ] **Commit.**

  ```bash
  git add src/main.rs
  git commit -m "feat: entity glyph rendering, sync positions, spawn rat soldiers and catgirl"
  ```

---

## Task 5: Hover Tooltip — Named + HP Bar

**Files:**
- Modify: `src/main.rs`

- [ ] **Update `mouse_hover_tile` to show entity name, flavor, and HP bar.**

  The current `mouse_hover_tile` signature needs two new query params. Replace the entire function:

  ```rust
  fn mouse_hover_tile(
    windows: Query<&Window>,
    camera_q: Query<(&Camera, &GlobalTransform), With<Camera2d>>,
    gw: Res<GameWorld>,
    cz: Res<CurrentZ>,
    fov: Res<Fov>,
    index: Res<TileEntityIndex>,
    named_q: Query<(&Named, Option<&Stats>)>,
    mut info_q: Query<&mut Text2d, With<TileInfoDisplay>>,
  ) {
    let Ok(mut info_text) = info_q.single_mut() else { return };
    let Ok(window) = windows.single() else { return };
    let Ok((camera, cam_transform)) = camera_q.single() else { return };

    let Some(cursor_pos) = window.cursor_position() else {
      *info_text = Text2d::new("");
      return;
    };
    let Ok(world_pos) = camera.viewport_to_world_2d(cam_transform, cursor_pos) else {
      *info_text = Text2d::new("");
      return;
    };

    let (tx, ty) = screen_to_tile(world_pos, gw.0.width, gw.0.height);
    let level = gw.0.level(cz.0);

    let in_bounds = tx >= 0
      && ty >= 0
      && (tx as usize) < level.width
      && (ty as usize) < level.height;

    if !in_bounds {
      *info_text = Text2d::new("");
      return;
    }

    let visible = fov.0.is_visible(tx as usize, ty as usize);
    let revealed = fov.0.is_revealed(tx as usize, ty as usize);

    if !visible && !revealed {
      *info_text = Text2d::new("");
      return;
    }

    let tile = level.tiles[ty as usize][tx as usize];
    let tile_line = if revealed && !visible {
      format!("({tx}, {ty})\n{} (remembered)", tile.name())
    } else {
      format!("({tx}, {ty})\n{}", tile.name())
    };

    // Entity info — only show for currently visible tiles
    let entity_lines = if visible {
      index
        .0
        .get(&(tx, ty))
        .and_then(|entities| entities.first())
        .and_then(|&e| named_q.get(e).ok())
        .map(|(named, stats)| {
          let hp_bar = stats.map(|s| {
            let filled = ((s.hp as f32 / s.max_hp as f32) * 10.0).round() as usize;
            let empty = 10usize.saturating_sub(filled);
            format!("\n[{}{}] {}/{}", "█".repeat(filled), "░".repeat(empty), s.hp, s.max_hp)
          });
          format!("\n\n{}{}\n{}", named.name, hp_bar.unwrap_or_default(), named.flavor)
        })
        .unwrap_or_default()
    } else {
      String::new()
    };

    *info_text = Text2d::new(format!("{tile_line}{entity_lines}"));
  }
  ```

- [ ] **Run the game and verify the tooltip.**

  ```
  cargo run
  ```

  Expected: hovering over a tile with an `r` shows something like:
  ```
  (42, 31)
  Floor

  Rat Soldier [██████████] 10/10
  A wiry rat-person clutching a crude spear. Smells like wet fur and old iron.
  ```

- [ ] **Commit.**

  ```bash
  git add src/main.rs
  git commit -m "feat: hover tooltip shows entity name, flavor, HP bar"
  ```

---

## Task 6: Bump Combat

**Files:**
- Modify: `src/main.rs`, `src/combat.rs`

- [ ] **Add a `bump_attack` helper to `combat.rs`.**

  In `src/combat.rs`, add:

  ```rust
  use trl::entities::{Enemy, Stats, Wearing};

  /// Apply player attack to an enemy entity. Returns true if the enemy died.
  /// Caller is responsible for despawning dead entities.
  pub fn bump_attack(
    attacker_attack: i32,
    target_stats: &mut Stats,
    target_wearing: Option<&Wearing>,
  ) -> bool {
    let dmg = resolve_damage(attacker_attack, target_wearing);
    target_stats.hp -= dmg;
    target_stats.hp <= 0
  }
  ```

- [ ] **Update `player_input` to intercept bump-into-enemy as an attack.**

  The `player_input` function in `main.rs` needs access to the tile index and enemy queries. Add these to its parameters and add the attack check before the move:

  The new `player_input` signature (replace existing):

  ```rust
  fn player_input(
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    gw: Res<GameWorld>,
    pause: Res<PauseMenu>,
    mut menu: ResMut<InteractMenu>,
    mut clock: ResMut<GameClock>,
    mut cooldown: ResMut<MoveCooldown>,
    mut fov: ResMut<Fov>,
    cz: Res<CurrentZ>,
    index: Res<TileEntityIndex>,
    mut commands: Commands,
    mut player_query: Query<(&mut PlayerPos, &mut Transform, &Stats), With<Player>>,
    mut enemy_query: Query<(&mut Stats, Option<&Wearing>), (With<Enemy>, Without<Player>)>,
  ) {
  ```

  Then after `let (dx, dy) = resolve_move(level, pos.x, pos.y, dir.0, dir.1);`, before the move block, insert:

  ```rust
  // Bump attack: if target tile has an enemy, attack instead of moving
  let target_x = pos.x + dx;
  let target_y = pos.y + dy;
  if let Some(entities) = index.0.get(&(target_x, target_y)) {
    let hostile = entities.iter().find(|&&e| {
      enemy_query.get(e).is_ok()
    });
    if let Some(&enemy_entity) = hostile
      && let Ok((mut enemy_stats, enemy_wearing)) = enemy_query.get_mut(enemy_entity)
      && let Ok((_, _, player_stats)) = player_query.single()
    {
      let player_attack = player_stats.attack;
      let died = combat::bump_attack(player_attack, &mut enemy_stats, enemy_wearing);
      if died {
        commands.entity(enemy_entity).despawn();
      }
      clock.advance(PlayerAction::Move { dx, dy }.time_cost());
      cooldown.0 = MOVE_COOLDOWN;
      return;
    }
  }
  ```

  Note: `player_query` signature changed — `Stats` added. Fix the existing borrow in the function:

  ```rust
  let Ok((mut pos, mut transform, _player_stats)) = player_query.single_mut() else { return };
  ```

  (The `_player_stats` is used in the bump block via `player_query.single()` — since we've already borrowed mutably, use a temporary re-read. Actually, to avoid the borrow conflict, read attack before the mutable borrow. Restructure the function so the attack is read before `pos` is mutably borrowed, like this at the top of the function body):

  ```rust
  if *pause != PauseMenu::Closed || matches!(*menu, InteractMenu::Open { .. }) {
    return;
  }

  // Read player attack stat before any mutable borrows
  let player_attack = player_query.single().ok().map(|(_, _, s)| s.attack).unwrap_or(5);

  let Ok((mut pos, mut transform, _)) = player_query.single_mut() else { return };
  ```

  And in the bump block use `player_attack` directly instead of querying again:

  ```rust
  if let Some(&enemy_entity) = hostile
    && let Ok((mut enemy_stats, enemy_wearing)) = enemy_query.get_mut(enemy_entity)
  {
    let died = combat::bump_attack(player_attack, &mut enemy_stats, enemy_wearing);
    if died {
      commands.entity(enemy_entity).despawn();
    }
    clock.advance(PlayerAction::Move { dx, dy }.time_cost());
    cooldown.0 = MOVE_COOLDOWN;
    return;
  }
  ```

- [ ] **Add `bump_attack` to the `combat` imports in `main.rs`.**

  Update the `combat::` import line:

  ```rust
  use combat::{TileEntityIndex, bump_attack, maintain_tile_index};
  ```

  Or reference it as `combat::bump_attack(...)` directly (either is fine).

- [ ] **Run `cargo check`.**

  ```
  cargo check
  ```

  Expected: no errors.

- [ ] **Run the game and verify bump combat.**

  ```
  cargo run
  ```

  Expected: walking into a rat soldier deals damage (watch HP bar in hover tooltip decrease). Walking into the soldier again eventually despawns it.

- [ ] **Commit.**

  ```bash
  git add src/main.rs src/combat.rs
  git commit -m "feat: bump combat — player attacks hostile entities on move"
  ```

---

## Task 7: Enemy AI — Step Toward Player and Attack

**Files:**
- Modify: `src/combat.rs`, `src/main.rs`

- [ ] **Add the enemy AI system to `combat.rs`.**

  Add to `src/combat.rs`:

  ```rust
  use trl::entities::{Enemy, Stats, TimeSinceAction, Wearing};

  // ---------------------------------------------------------------------------
  // Enemy AI
  // ---------------------------------------------------------------------------

  /// Compute one tile step from (ex, ey) toward (px, py).
  fn step_toward(ex: i32, ey: i32, px: i32, py: i32) -> (i32, i32) {
    ((px - ex).signum(), (py - ey).signum())
  }

  pub fn enemy_ai(
    time: Res<Time>,
    index: Res<TileEntityIndex>,
    gw: Res<crate::GameWorld>,
    player_q: Query<(&crate::PlayerPos, &mut Stats), With<crate::Player>>,
    mut enemy_q: Query<
      (&mut Location, &mut TimeSinceAction, &Stats, Option<&Wearing>),
      With<Enemy>,
    >,
  ) {
    let Ok((player_pos, mut player_stats)) = player_q.single() else { return };
    let (px, py) = (player_pos.x, player_pos.y);
    let level = gw.0.level(/* need cz */ 0); // placeholder — see note below
    let dt = time.delta_secs();

    for (mut location, mut timer, enemy_stats, enemy_wearing) in enemy_q.iter_mut() {
      timer.0 += dt;

      let Location::Coords { x: ex, y: ey } = *location else { continue };
      let dist = (px - ex).abs().max((py - ey).abs());

      // Attack if adjacent
      if dist == 1 && timer.0 >= 1.0 / enemy_stats.attack_speed {
        let dmg = resolve_damage(enemy_stats.attack, enemy_wearing);
        player_stats.hp = (player_stats.hp - dmg).max(0);
        if player_stats.hp == 0 {
          // Player death: just log for now
          bevy::log::info!("You died.");
        }
        timer.0 = 0.0;
        continue;
      }

      // Move toward player
      if timer.0 >= 1.0 / enemy_stats.move_speed {
        let (dx, dy) = step_toward(ex, ey, px, py);
        let (nx, ny) = (ex + dx, ey + dy);
        let tile_walkable = level.walkable(nx, ny);
        let occupied = index.0.get(&(nx, ny)).map_or(false, |v| !v.is_empty());
        if tile_walkable && !occupied {
          *location = Location::Coords { x: nx, y: ny };
        }
        timer.0 = 0.0;
      }
    }
  }
  ```

  **Note on `CurrentZ`:** The enemy AI needs to know the current level. Pass `cz: Res<crate::CurrentZ>` and use `gw.0.level(cz.0)`. Add it to the system parameters:

  ```rust
  pub fn enemy_ai(
    time: Res<Time>,
    index: Res<TileEntityIndex>,
    cz: Res<crate::CurrentZ>,
    gw: Res<crate::GameWorld>,
    mut player_q: Query<(&crate::PlayerPos, &mut Stats), With<crate::Player>>,
    mut enemy_q: Query<
      (&mut Location, &mut TimeSinceAction, &Stats, Option<&Wearing>),
      With<Enemy>,
    >,
  ) {
    let Ok((player_pos, mut player_stats)) = player_q.single_mut() else { return };
    let (px, py) = (player_pos.x, player_pos.y);
    let level = gw.0.level(cz.0);
    let dt = time.delta_secs();

    for (mut location, mut timer, enemy_stats, enemy_wearing) in enemy_q.iter_mut() {
      timer.0 += dt;

      let Location::Coords { x: ex, y: ey } = *location else { continue };
      let dist = (px - ex).abs().max((py - ey).abs());

      if dist == 1 && timer.0 >= 1.0 / enemy_stats.attack_speed {
        let dmg = resolve_damage(enemy_stats.attack, enemy_wearing);
        player_stats.hp = (player_stats.hp - dmg).max(0);
        if player_stats.hp == 0 {
          bevy::log::info!("You died.");
        }
        timer.0 = 0.0;
        continue;
      }

      if timer.0 >= 1.0 / enemy_stats.move_speed {
        let (dx, dy) = step_toward(ex, ey, px, py);
        let (nx, ny) = (ex + dx, ey + dy);
        if level.walkable(nx, ny) && !index.0.contains_key(&(nx, ny)) {
          *location = Location::Coords { x: nx, y: ny };
        }
        timer.0 = 0.0;
      }
    }
  }
  ```

- [ ] **Register `enemy_ai` in `main.rs`.**

  Add `enemy_ai` to the imports from `combat`:

  ```rust
  use combat::{TileEntityIndex, bump_attack, enemy_ai, maintain_tile_index};
  ```

  Add it to the system chain in `main()`, after `player_input` and before `camera_follow`:

  ```rust
  .add_systems(
    Update,
    (
      maintain_tile_index,
      setup_glyph_visuals,
      sync_entity_positions,
      advance_realtime,
      handle_menus,
      player_input,
      enemy_ai,
      camera_follow,
      update_fov_visuals,
      mouse_hover_tile,
      update_hud,
    )
      .chain()
  )
  ```

- [ ] **Run `cargo check`.**

  ```
  cargo check
  ```

  Expected: no errors.

- [ ] **Run the game and verify enemy AI.**

  ```
  cargo run
  ```

  Expected: rat soldiers visibly move toward the player over time and deal damage (player HP visible when hovering your own tile if you add yourself to the index — or just watch the console for damage via log). Enemies stop adjacent to player and attack each second.

- [ ] **Commit.**

  ```bash
  git add src/combat.rs src/main.rs
  git commit -m "feat: enemy AI — step toward player, bump attack, player death log"
  ```

---

## Self-Review Checklist

- [x] **Spec coverage:** Named ✓, Stats ✓, Wielding ✓, Wearing ✓, Armor ✓, Glyph ✓, rat_soldier ✓, armored_rat_soldier ✓, catgirl ✓, TileEntityIndex ✓, hover tooltip ✓, bump combat ✓, enemy AI ✓, player death ✓
- [x] **No placeholders:** All code blocks are complete
- [x] **Type consistency:** `resolve_damage(i32, Option<&Wearing>)` used consistently in Task 3, 6, 7. `bump_attack` calls `resolve_damage`. `Stats`, `Named`, `Glyph` defined in Task 1, used in Tasks 2–7.
- [x] **Borrow conflict in Task 6:** Documented — read `player_attack` before mutable borrow of `pos`.
- [x] **`GameWorld`, `PlayerPos`, `CurrentZ`, `Player` are in `main.rs`:** Referenced as `crate::GameWorld` etc. from `combat.rs`.
