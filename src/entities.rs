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
  Key,
  Spear
}

/// Armor types that can be worn.
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

/// ASCII glyph visual: char + RGB color for Text2d rendering.
#[derive(Component, Clone, Debug)]
pub struct Glyph {
  pub ch: char,
  pub color: Color,
}

/// Identity and SS13-style flavor text shown on hover.
#[derive(Component, Debug)]
pub struct Named {
  pub name:   &'static str,
  pub flavor: &'static str,
}

/// Flat combat stats.
#[derive(Component, Debug)]
pub struct Stats {
  pub hp:           i32,
  pub max_hp:       i32,
  pub attack:       i32,
  pub move_speed:   f32,
  pub attack_speed: f32,
}

/// What an entity is holding. None = unarmed (has hands, holds nothing).
#[derive(Component, Debug)]
pub struct Wielding(pub Option<Item>);

/// Armor being worn. None = unarmored.
#[derive(Component, Debug)]
pub struct Wearing(pub Option<Armor>);

/// Tracks time since the entity last acted (seconds). Used by enemy AI.
#[derive(Component, Debug, Default)]
pub struct TimeSinceAction(pub f32);

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
        Glyph { ch: 'r', color: Color::srgb(0.9, 0.6, 0.4) },
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
        Glyph { ch: 'r', color: Color::srgb(0.7, 0.5, 0.3) },
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
        Glyph { ch: 'c', color: Color::srgb(0.9, 0.7, 0.9) },
        TimeSinceAction(0.0),
      ))
  }
}
