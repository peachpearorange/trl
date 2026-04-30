# Defined NPCs System

## Summary

Each defined NPC gets its own file under `src/npcs/`, containing an `Object` constructor fn, inline dialogue tree, stats, and starting gear. A shared `Object::defined_npc()` base constructor reduces boilerplate. Mira is migrated from `entities.rs`/`dialogue.rs` into this new structure, and 4 new NPCs are added.

## Structure

```
src/npcs/
├── mod.rs          — re-exports all NPC fns
├── mira.rs         — catgirl cartographer (migrated)
├── chronos.rs      — time-travelling sock-stealing wizard
├── unit7.rs        — malfunctioning robot
├── kong.rs         — psychic genetically modified monkey
└── guard.rs        — generic guard
```

## Object::defined_npc()

New constructor on `Object` in `entities.rs`. Builds on `Object::npc()` (which provides `Collidable(false)`, `Character`, `FactionComp(Neutral)`, `Gravity`), then adds the components every defined NPC needs:

```rust
pub fn defined_npc(
    named: Named,
    stats: Stats,
    wielding: Option<Item>,
    wearing: Option<Armor>,
    glyph: Glyph,
    dialogue: &'static DialogueTree,
) -> Self {
    Self::npc()
        .add(named)
        .add(stats)
        .add(Wielding(wielding))
        .add(Wearing(wearing))
        .add(glyph)
        .add(Dialogue(dialogue))
}
```

## Per-NPC File Format

Each file exports a single `pub fn name() -> Object` and a `static DialogueTree`.

```rust
// src/npcs/chronos.rs
use crate::entities::*;

static DIALOGUE: DialogueTree = DialogueTree { nodes: &[...] };

pub fn chronos() -> Object {
    Object::defined_npc(
        Named { name: "Chronos", flavor: "A disheveled wizard in mismatched socks" },
        Stats { hp: 12, max_hp: 12, attack: 3, move_speed: 1, attack_speed: 1 },
        None,
        None,
        Glyph { ch: 'W', color: Color::srgb(0.6, 0.2, 0.9) },
        &DIALOGUE,
    )
}
```

## NPCs to Create

### Mira (migrated)
- **File:** `mira.rs`
- **Glyph:** `c` magenta (unchanged)
- **Stats:** 8 HP, 2 ATK (unchanged)
- **Dialogue:** 13-node tree about rat occupation, caves, ancient machines (moved from `dialogue.rs`)

### Chronos — Time-Travelling Wizard
- **File:** `chronos.rs`
- **Glyph:** `W` purple
- **Flavor:** "A disheveled wizard in mismatched socks"
- **Stats:** 12 HP, 3 ATK
- **Gear:** None
- **Dialogue:** Obsessed with socks, cryptic time-travel references, accidentally useful hints about the future/past

### Unit-7 — Malfunctioning Robot
- **File:** `unit7.rs`
- **Glyph:** `R` cyan
- **Flavor:** "A dented robot sparking intermittently"
- **Stats:** 20 HP, 4 ATK
- **Gear:** None
- **Dialogue:** Garbled corporate-speak, error messages, fragments of useful data about the ancient machines, occasionally reboots mid-sentence

### Kong — Psychic Monkey
- **File:** `kong.rs`
- **Glyph:** `M` green
- **Flavor:** "A small monkey with unsettlingly intelligent eyes"
- **Stats:** 6 HP, 1 ATK
- **Gear:** None
- **Dialogue:** Telepathic communication (italicized/bracketed text), philosophical, knows things it shouldn't, occasionally reads the player's mind

### Guard — Generic Guard
- **File:** `guard.rs`
- **Glyph:** `G` white
- **Flavor:** "A tired-looking guard"
- **Stats:** 10 HP, 3 ATK
- **Gear:** `Wielding(Some(Item::IronSword))`, `Wearing(Some(Armor::Leather))`
- **Dialogue:** Short, generic ("I used to be an adventurer like you..." etc.)

## Migration

1. Move Mira's dialogue tree from `src/dialogue.rs` into `src/npcs/mira.rs`
2. Move `Object::catgirl()` from `entities.rs` into `src/npcs/mira.rs` as `pub fn mira() -> Object`
3. Update spawn call in `main.rs` to use `npcs::mira()`
4. Delete `src/dialogue.rs` (only contained Mira's dialogue)
5. Remove `mod dialogue;` from `main.rs`

## Spawning

NPCs are spawned in `main.rs` `setup()`. For now, new NPCs are placed at hardcoded positions near the player start (same pattern as Mira currently). The `world_data::NpcPlacement` system exists for future zone-based placement but is out of scope here.
