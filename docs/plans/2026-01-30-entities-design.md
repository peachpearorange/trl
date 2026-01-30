# Entities Design

Spawnable entity variants for the tile-based roguelike.

## Location Component

Every entity gets a `Location` passed to `spawn()`:

```rust
#[derive(Component, Clone, Debug)]
pub enum Location {
    Coords { x: i32, y: i32 },
    Inventory(Entity),  // held by another entity
    Nowhere,            // not placed yet
}
```

## Value Types

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Item {
    Sword,
    Coin,
    Potion,
    Key,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Material {
    Stone,
    Wood,
    Metal,
}
```

## Components

```rust
#[derive(Component)]
pub struct Collidable(pub bool);

#[derive(Component)]
pub struct LightSource { pub radius: u32 }

#[derive(Component)]
pub struct GroundItem(pub Item);

#[derive(Component)]
pub struct Door { pub open: bool }

#[derive(Component)]
pub struct WallComp { pub material: Material }

#[derive(Component)]
pub struct Faction(pub EntityFaction);

#[derive(Component)]
pub struct Character;

#[derive(Component)]
pub struct Player;

#[derive(Component)]
pub struct Enemy;

#[derive(Component)]
pub struct Tree;
```

## Spawnable Variants

```rust
spawnable! {
    // Root for all characters - takes faction as parameter
    (BaseCharacter { faction: EntityFaction }, (Character, Faction(faction),)),

    // Player inherits from BaseCharacter
    (Player {}, BaseCharacter { faction: EntityFaction::Player }, (Player,)),

    // Enemy inherits from BaseCharacter
    (Enemy {}, BaseCharacter { faction: EntityFaction::Hostile }, (Enemy,)),

    // Environment objects
    (Tree {}, (Tree, Collidable(true),)),

    (Wall { material: Material }, (WallComp { material }, Collidable(true),)),

    (Door { open: bool }, (Door { open }, Collidable(!open),)),

    // Items on ground
    (GroundItem { item: Item }, (GroundItem(item),)),

    // Torch - emits light
    (Torch { radius: u32 }, (LightSource { radius },)),
}
```

## Usage

```rust
Spawnable::Player {}.spawn(&mut commands, Location::Coords { x: 5, y: 5 });
Spawnable::Wall { material: Material::Stone }.spawn(&mut commands, Location::Coords { x: 0, y: 0 });
Spawnable::Door { open: false }.spawn(&mut commands, Location::Coords { x: 3, y: 2 });
```

## File Structure

- `src/spawnable.rs` - macro definition only
- `src/entities.rs` - Location, Item, Material, components, and `spawnable!` invocation
