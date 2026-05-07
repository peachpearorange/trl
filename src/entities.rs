//! Entity types and spawnable definitions for the game.

use {crate::faction::Faction,
     bevy::prelude::*,
     std::sync::Arc};

// ============ DIALOGUE ============

/// A flat list of named nodes that forms one NPC's conversation.
#[derive(Debug)]
pub struct DialogueTree {
  pub nodes: &'static [DialogueNode],
}

impl DialogueTree {
  /// Find a node by name. Returns the first node if `name` is not found.
  pub fn find(&self, name: &str) -> &DialogueNode {
    self.nodes.iter().find(|n| n.name == name).unwrap_or(&self.nodes[0])
  }
}

/// One node in a dialogue tree: a name, what the NPC says, and the player's choices.
#[derive(Debug)]
pub struct DialogueNode {
  pub name:    &'static str,
  pub text:    &'static str,
  pub choices: &'static [DialogueChoice],
}

/// One response option the player can pick.
#[derive(Debug)]
pub struct DialogueChoice {
  /// Button label shown to the player.
  pub text: &'static str,
  /// Name of the next node, or `None` to end the conversation.
  pub next: Option<&'static str>,
}

/// Marks an entity as conversable; holds a pointer to its dialogue tree.
#[derive(Component, Clone, Debug)]
pub struct Dialogue(pub &'static DialogueTree);

/// Construct a [`DialogueTree`] (for use in `static` initializers).
pub const fn tree(nodes: &'static [DialogueNode]) -> DialogueTree {
  DialogueTree { nodes }
}

/// Construct a named [`DialogueNode`].
pub const fn node(
  name: &'static str,
  text: &'static str,
  choices: &'static [DialogueChoice],
) -> DialogueNode {
  DialogueNode { name, text, choices }
}

/// A choice that advances to another node by name.
pub const fn go(text: &'static str, next: &'static str) -> DialogueChoice {
  DialogueChoice { text, next: Some(next) }
}

/// A choice that ends the conversation.
pub const fn end(text: &'static str) -> DialogueChoice {
  DialogueChoice { text, next: None }
}

// ============ LOCATION ============

/// Where an entity exists in the world.
#[derive(Component, Clone, Debug)]
pub enum Location {
  /// At specific tile coordinates on z-level `z`.
  Coords { x: i32, y: i32, z: usize, zx: usize, zy: usize },
  /// In another entity's inventory.
  Inventory(Entity),
  /// Not placed anywhere (template, UI preview, etc.).
  Nowhere
}

impl Location {
  pub fn xyz(x: i32, y: i32, z: usize) -> Self {
    Location::Coords { x, y, z, zx: 0, zy: 0 }
  }

  /// World-space tile coordinates as Vec2 (for interpolation). Returns None for non-Coords.
  pub fn as_vec2(&self) -> Option<Vec2> {
    match self {
      Location::Coords { x, y, .. } => Some(Vec2::new(*x as f32, *y as f32)),
      _ => None,
    }
  }
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
#[derive(Component, Clone, Copy)]
pub struct Collidable(pub bool);

/// Emits light in a radius.
#[derive(Component, Clone, Copy)]
pub struct LightSource {
  pub radius: u32
}

/// An item sitting on the ground.
#[derive(Component, Clone, Copy)]
pub struct GroundItem(pub Item);

/// A door that can be opened/closed.
#[derive(Component, Clone)]
pub struct Door {
  pub open: bool,
  /// Original colour when closed; restored on close.
  pub closed_color: Color,
}

/// Wall construction material.
#[derive(Component, Clone, Copy)]
pub struct WallComp {
  pub material: Material
}

/// Combat/AI faction.
#[derive(Component, Clone, Copy)]
pub struct FactionComp(pub Faction);

/// Marker for character entities (player, enemies, NPCs).
#[derive(Component, Clone, Copy)]
pub struct Character;

/// The player character.
#[derive(Component, Clone, Copy)]
pub struct Player;

/// An enemy entity.
#[derive(Component, Clone, Copy)]
pub struct Enemy;

/// A tree entity.
#[derive(Component, Clone, Copy)]
pub struct Tree;

/// Placed loot container; blocks the tile until emptied.
#[derive(Component, Clone, Debug)]
pub struct LootChest {
  pub opened: bool,
}

/// Entity occupies its tile for line-of-sight (like an opaque tile) but need not block movement.
#[derive(Component, Clone, Copy)]
pub struct BlocksSight;

/// Visual for a grid entity: optional PNG (tile-sized sprite) or [`Text2d`] from `ch` + `color`.
#[derive(Component, Clone, Debug)]
pub struct Glyph {
  pub ch: char,
  pub color: Color,
  /// Asset path relative to `assets/` (e.g. `textures/catgirl.png`).
  pub texture: Option<&'static str>,
  /// Space-Qud–style mask: black → first color, white → second; transparent stays clear.
  pub sprite_palette: Option<(Color, Color)>,
}

impl Glyph {
  pub fn ascii(ch: char, color: Color) -> Self {
    Self {
      ch,
      color,
      texture: None,
      sprite_palette: None,
    }
  }

  pub fn sprite(path: &'static str, ch: char, color: Color) -> Self {
    Self {
      ch,
      color,
      texture: Some(path),
      sprite_palette: None,
    }
  }

  /// Mask PNG (black / white / alpha); instance colors set how it draws.
  pub fn palette_sprite(path: &'static str, ch: char, primary: Color, secondary: Color) -> Self {
    Self {
      ch,
      color: primary,
      texture: Some(path),
      sprite_palette: Some((primary, secondary)),
    }
  }
}

/// Identity and SS13-style flavor text shown on hover.
#[derive(Component, Clone, Debug)]
pub struct Named {
  pub name:   &'static str,
  pub flavor: &'static str,
}

/// Flat combat stats.
#[derive(Component, Clone, Debug)]
pub struct Stats {
  pub hp:           i32,
  pub max_hp:       i32,
  pub attack:       i32,
  pub move_speed:   f32,
  pub attack_speed: f32,
}

/// What an entity is holding. None = unarmed (has hands, holds nothing).
#[derive(Component, Clone, Debug)]
pub struct Wielding(pub Option<Item>);

/// Armor being worn. None = unarmored.
#[derive(Component, Clone, Debug)]
pub struct Wearing(pub Option<Armor>);

/// Tracks display frames since the entity last acted. Used by enemy AI.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct TimeSinceAction(pub u32);

/// Marker: this entity is affected by gravity and will fall through Air tiles.
#[derive(Component, Clone, Copy, Debug)]
pub struct Gravity;

/// Marker component for the flight console entity.
#[derive(Component, Clone, Copy)]
pub struct FlightConsole;

/// Smooth visual interpolation state for moving entities.
/// Stores the previous position (at move start) and computes a weighted average
/// toward the current logical Location each frame, producing fluid tile-to-tile sliding.
#[derive(Component, Debug)]
pub struct Visuals {
  /// Zone-local tile position when the last move began.
  pub prev: Vec2,
  /// Monotonic render-frame index when the logical tile position last changed. `None` = no active slide.
  pub last_move_start_frame: Option<u64>,
  /// Current interpolated display position (zone-local, recomputed each frame).
  pub display: Vec2,
  /// Last known zone-local position from Location (for change detection).
  pub last_pos: Vec2,
}

// ============ SPAWNABLE ============

/// Composable entity blueprint. Chain constructor fns that delegate to
/// each other, then call `.spawn()`.
///
/// [`Clone`] uses `Arc` internally so the same blueprint can spawn many entities (e.g. prefabs).
///
/// ```ignore
/// player().add(Location::xyz(5, 3, 2)).spawn(&mut commands);
/// ```
#[derive(Clone)]
pub struct Object(Arc<dyn Fn(&mut EntityCommands) + Send + Sync + 'static>);

impl Object {
  fn new(bundle: impl Bundle + Clone + Send + Sync + 'static) -> Self {
    Self(Arc::new(move |e: &mut EntityCommands| {
      e.insert(bundle.clone());
    }))
  }

  pub fn add(self, bundle: impl Bundle + Clone + Send + Sync + 'static) -> Self {
    let prev = self.0.clone();
    Self(Arc::new(move |e: &mut EntityCommands| {
      prev(e);
      e.insert(bundle.clone());
    }))
  }

  pub fn spawn(self, commands: &mut Commands) -> Entity {
    let mut e = commands.spawn_empty();
    (self.0)(&mut e);
    e.id()
  }

  /// NPC base: Neutral faction, non-blocking.
  pub fn npc() -> Self {
    Self::new((Collidable(false), Character, FactionComp(Faction::Neutral), Gravity))
  }

  /// Fully-defined NPC: named, statted, equipped, visible, conversable.
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

  /// Spawn this entity at tile coordinates, inserting Location::Coords.
  pub fn spawn_at(self, commands: &mut Commands, x: i32, y: i32, z: usize) -> Entity {
    let mut e = commands.spawn_empty();
    (self.0)(&mut e);
    e.insert(Location::xyz(x, y, z));
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
  pub fn character(faction: Faction) -> Self  { Self::physical(true).add((Character, FactionComp(faction), Gravity)) }
  pub fn player() -> Self                     { Self::character(Faction::Player).add(Player) }
  pub fn enemy() -> Self                      { Self::character(Faction::Hostile).add((Enemy, TimeSinceAction(0))) }
  pub fn structure(blocks: bool) -> Self      { Self::physical(blocks) }
  pub fn wall(material: Material) -> Self     { Self::structure(true).add(WallComp { material }) }
  pub fn tree() -> Self                       { Self::structure(false).add((
    Tree,
    BlocksSight,
    Glyph::sprite(
      "textures/a_tree.png",
      'T',
      Color::srgb(0.13, 0.55, 0.13),
    ),
    Named { name: "Tree", flavor: "A sturdy tree. Could be chopped for wood." },
  )) }
  pub fn flight_console() -> Self {
    Self::structure(true).add((
        Glyph::ascii('C', Color::srgb(0.3, 0.9, 0.4)),
        Named {
            name: "Flight Console",
            flavor: "Navigation computer. Plot a course to a destination.",
        },
        FlightConsole,
    ))
  }
  pub fn loot_chest() -> Self {
    Self::structure(true).add((
        LootChest { opened: false },
      Glyph::sprite(
        "textures/a_wreck_of_a_sci-fi_flying_vehicle.png",
        '&',
        Color::srgb(0.72, 0.52, 0.28),
      ),
      Named {
        name: "Chest",
        flavor: "Someone stashed supplies here.",
      },
    ))
  }
  pub fn door(open: bool, closed_color: Color) -> Self {
    Self::structure(!open).add(Door { open, closed_color })
  }
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
        Glyph::sprite(
          "textures/shady_looking_guy_in_clothes_meant_for_a_hot_desert.png",
          'r',
          Color::srgb(0.9, 0.6, 0.4),
        ),
      ))
  }

  pub fn armored_rat_soldier() -> Self {
    Self::enemy()
      .add((
        Named {
          name: "Armored Rat Soldier",
          flavor: "A rat-person in battered leather armor, gripping a crude spear. The hide smells worse than the iron.",
        },
        Stats { hp: 10, max_hp: 10, attack: 3, move_speed: 3.0, attack_speed: 1.0 },
        Wielding(Some(Item::Spear)),
        Wearing(Some(Armor::Leather)),
        Glyph::sprite(
          "textures/retro-future_post-apocalyptic_settlement_guard.png",
          'r',
          Color::srgb(0.7, 0.5, 0.3),
        ),
      ))
  }

}
