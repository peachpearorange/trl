# Content System Design
_2026-04-18_

## Overview

A code-as-content system for defining creatures, items, and armor using the existing `Spawnable` builder. New components cover identity/flavor text, combat stats, and equipment. Bump combat uses those stats. A tile-entity index enables efficient hover lookup; a right-side tooltip panel shows SS13-style name + flavor text.

This spec covers four things in one cohesive sprint:
1. New components (`Named`, `Stats`, `Wielding`, `Wearing`, `Armor`)
2. Creature definitions as `Spawnable` associated functions
3. Tile-entity spatial index + right-side hover tooltip
4. Bump combat

---

## 1. New Components

```rust
/// Identity and SS13-style flavor text.
#[derive(Component)]
pub struct Named {
    pub name:   &'static str,
    pub flavor: &'static str,
}

/// Flat combat stats. move_speed and attack_speed are in action-cost units
/// so a future turn-based upgrade can reinterpret them without new fields.
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

/// What armor an entity is wearing. None = unarmored.
#[derive(Component)]
pub struct Wearing(pub Option<Armor>);

pub enum Armor {
    Leather, // DR 1
    Chain,   // DR 2
    Plate,   // DR 3
}
```

`Armor` provides flat damage reduction (DR). Incoming damage = `max(0, attack - DR)`.

`Item` gains spear (and any other weapon items as needed). Weapons do not modify `Stats.attack` yet — that's a future pass.

---

## 2. Creature Definitions

Creature constructor functions are `impl Spawnable` associated functions, all in `src/entities.rs` alongside existing Spawnable code. Each is one `.add((...))` call with a tuple bundle.

```rust
impl Spawnable {
    pub fn rat_soldier() -> Self {
        Self::enemy(Sprite::RAT_PERSON)
            .add((
                Named { name: "Rat Soldier", flavor: "A wiry rat-person clutching a crude spear. Smells like wet fur and old iron." },
                Stats { hp: 10, max_hp: 10, attack: 3, move_speed: 3.0, attack_speed: 1.0 },
                Wielding(Some(Item::SPEAR)),
                Wearing(None),
            ))
    }

    pub fn armored_rat_soldier() -> Self {
        Self::enemy(Sprite::RAT_PERSON)
            .add((
                Named { name: "Rat Soldier", flavor: "A wiry rat-person clutching a crude spear. Smells like wet fur and old iron." },
                Stats { hp: 10, max_hp: 10, attack: 3, move_speed: 3.0, attack_speed: 1.0 },
                Wielding(Some(Item::SPEAR)),
                Wearing(Some(Armor::Leather)),
            ))
    }

    pub fn catgirl() -> Self {
        Self::npc(Sprite::CATGIRL)
            .add((
                Named { name: "Catgirl", flavor: "She eyes you warily, ears flat against her head." },
                Stats { hp: 8, max_hp: 8, attack: 2, move_speed: 4.0, attack_speed: 1.2 },
                Wielding(None),
                Wearing(None),
            ))
    }
}
```

`Spawnable::npc(sprite)` is a new base constructor (alongside `enemy`) using `Faction::Neutral` and `Collidable(false)` — NPCs don't block movement by default.

Spawner code (map gen) picks variants at spawn time with plain Rust:

```rust
if rng.gen_bool(0.3) { Spawnable::armored_rat_soldier() } else { Spawnable::rat_soldier() }
```

---

## 3. Tile-Entity Spatial Index + Hover Tooltip

### Spatial Index

```rust
#[derive(Resource, Default)]
pub struct TileEntityIndex(pub HashMap<(i32, i32), Vec<Entity>>);
```

A system runs after any `Location` component change (using Bevy change detection on `Location`) and rebuilds affected entries. On entity despawn, the entry is cleared.

Systems that need "what's on tile X,Y" query `TileEntityIndex` rather than iterating all entities.

### Hover Tooltip

A right-side UI panel (Bevy UI node, fixed to screen right). When the mouse hovers a tile:

- **Top line:** tile name (existing behavior, moved into this panel)
- **If a `Named` entity is on the tile:**
  - Entity name in white/bright
  - Flavor text below in a dimmer color (grey)
  - HP bar if the entity has `Stats` (e.g. `[██████░░░░] 6/10`)

Panel is invisible when not hovering anything of interest. Multiple entities on a tile: show the topmost (last in `Vec<Entity>` — or highest priority by component presence: character > item > structure).

---

## 4. Bump Combat

### Player attacks enemy

When player move input targets a tile occupied by a `Hostile` entity:
- Do not move
- Resolve attack: `damage = max(0, player.attack - enemy_dr)`
- Apply `enemy.hp -= damage`
- If `enemy.hp <= 0`: despawn enemy, remove from `TileEntityIndex`

`enemy_dr` comes from `Wearing`: `None` → 0, `Some(armor)` → armor's DR value.

### Enemy attacks player

Each enemy tracks time-since-last-action. On each update tick:
- If adjacent to player and `time_since_action >= 1.0 / attack_speed`: attack player, reset timer
- Otherwise: step toward player (pathfinding: direct step toward player, blocked by `Collidable` tiles), gated by `move_speed`

### Player death

For now: log "You died." to the in-game message area. No game-over screen yet.

### Not in scope for this sprint

- Weapon stats modifying attack
- Status effects (bleed, trip)
- Special moves / cooldowns
- Ranged combat
- Enemy pathfinding beyond direct step

---

## File Changes

| File | Change |
|------|--------|
| `src/entities.rs` | Add `Named`, `Stats`, `Wielding`, `Wearing`, `Armor`; add creature constructor fns to `impl Spawnable`; add `Spawnable::npc()` |
| `src/level.rs` or `src/map.rs` | Add `Item::Spear` if not present |
| `src/main.rs` | Add `TileEntityIndex` resource; add spatial index maintenance system; extend hover to use index + show tooltip panel; add bump combat system; add enemy AI tick system |

`src/main.rs` is already 865 lines. Combat, enemy AI, and UI systems should be extracted into focused modules (`src/combat.rs`, `src/ui.rs`) during this sprint to keep files manageable.
