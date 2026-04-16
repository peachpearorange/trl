//! Entity types and spawnable definitions for the game.

use {crate::tile_loader::Faction,
     bevy::prelude::*};

// ============ LOCATION ============

/// Where an entity exists in the world.
#[derive(Component, Clone, Debug)]
pub enum Location {
  /// At specific tile coordinates.
  Coords { x: i32, y: i32 },
  /// In another entity's inventory.
  Inventory(Entity),
  /// Not placed anywhere (template, UI preview, etc.).
  Nowhere
}

// ============ VALUE TYPES ============

/// Items that can be picked up and used.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Item {
  Sword,
  Coin,
  Potion,
  Key
}

/// Materials for construction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Material {
  Stone,
  Wood,
  Metal
}

// ============ COMPONENTS ============

/// Whether this entity blocks movement.
#[derive(Component)]
pub struct Collidable(pub bool);

/// Emits light in a radius.
#[derive(Component)]
pub struct LightSource {
  pub radius: u32
}

/// An item sitting on the ground.
#[derive(Component)]
pub struct GroundItem(pub Item);

/// A door that can be opened/closed.
#[derive(Component)]
pub struct Door {
  pub open: bool
}

/// Wall construction material.
#[derive(Component)]
pub struct WallComp {
  pub material: Material
}

/// Combat/AI faction.
#[derive(Component)]
pub struct FactionComp(pub Faction);

/// Marker for character entities (player, enemies, NPCs).
#[derive(Component)]
pub struct Character;

/// The player character.
#[derive(Component)]
pub struct Player;

/// An enemy entity.
#[derive(Component)]
pub struct Enemy;

/// A tree entity.
#[derive(Component)]
pub struct Tree;

// ============ SPAWNABLE ============

/// Composable entity blueprint. Chain constructor fns that delegate to
/// each other, then call `.spawn()`.
///
/// ```ignore
/// player().add(Location::Coords { x: 5, y: 3 }).spawn(&mut commands);
/// ```
pub struct Spawnable(Box<dyn FnOnce(&mut EntityCommands) + Send + Sync>);

impl Spawnable {
  fn new(bundle: impl Bundle) -> Self {
    Self(Box::new(|e: &mut EntityCommands| { e.insert(bundle); }))
  }

  pub fn add(self, bundle: impl Bundle) -> Self {
    Self(Box::new(|e: &mut EntityCommands| { (self.0)(e); e.insert(bundle); }))
  }

  pub fn spawn(self, commands: &mut Commands) -> Entity {
    let mut e = commands.spawn_empty();
    (self.0)(&mut e);
    e.id()
  }

  // ---- constructor hierarchy ----
  //
  //  physical ── character ─┬─ player
  //     │                   └─ enemy
  //     └─ structure ─┬─ wall
  //                   ├─ tree
  //                   └─ door

  pub fn physical(blocks: bool) -> Self      { Self::new(Collidable(blocks)) }
  pub fn character(faction: Faction) -> Self  { Self::physical(true).add((Character, FactionComp(faction))) }
  pub fn player() -> Self                     { Self::character(Faction::Player).add(Player) }
  pub fn enemy() -> Self                      { Self::character(Faction::Hostile).add(Enemy) }
  pub fn structure(blocks: bool) -> Self      { Self::physical(blocks) }
  pub fn wall(material: Material) -> Self     { Self::structure(true).add(WallComp { material }) }
  pub fn tree() -> Self                       { Self::structure(true).add(Tree) }
  pub fn door(open: bool) -> Self             { Self::structure(!open).add(Door { open }) }
  pub fn ground_item(item: Item) -> Self      { Self::new(GroundItem(item)) }
  pub fn torch(radius: u32) -> Self           { Self::new(LightSource { radius }) }
}
