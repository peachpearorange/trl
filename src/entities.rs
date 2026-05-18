//! Entity types and spawnable definitions for the game.

use {crate::faction::Faction, bevy::prelude::*, std::collections::VecDeque, std::sync::Arc};

// ============ DIALOGUE ============

/// A flat list of named nodes that forms one NPC's conversation.
#[derive(Debug)]
pub struct DialogueTree {
  pub nodes: &'static [DialogueNode]
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
  pub name: &'static str,
  pub text: &'static str,
  pub choices: &'static [DialogueChoice]
}

/// One response option the player can pick.
#[derive(Debug)]
pub struct DialogueChoice {
  /// Button label shown to the player.
  pub text: &'static str,
  /// Name of the next node, or `None` to end the conversation.
  pub next: Option<&'static str>
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
  choices: &'static [DialogueChoice]
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
      _ => None
    }
  }
}

// ============ GEAR / LOADOUT ============

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Gear {
  Weapon(crate::level::Item),
  Armor(crate::level::Item),
  Grenade(crate::level::Item),
  InnateGun { damage: i32 },
  InnateGrenadeThrow { min_range: i32 },
  InnateSporeEmit,
  NaturalArmor { dr: i32 },
}

impl Gear {
  pub fn is_weapon(self) -> bool { matches!(self, Gear::Weapon(_)) }
  pub fn is_armor(self) -> bool { matches!(self, Gear::Armor(_) | Gear::NaturalArmor { .. }) }
  pub fn is_grenade(self) -> bool { matches!(self, Gear::Grenade(_)) }
  pub fn is_ability(self) -> bool {
    matches!(self, Gear::InnateGun { .. } | Gear::InnateGrenadeThrow { .. } | Gear::InnateSporeEmit)
  }

  pub fn weapon_capacity_bonus(self) -> u32 { 0 }
  pub fn grenade_capacity_bonus(self) -> u32 { 0 }
}

#[derive(Clone, Copy, Debug)]
pub struct GearSlot {
  pub gear: Gear,
  pub count: u32,
  pub cooldown: u32,
  pub timer: u32,
}

impl GearSlot {
  pub fn passive(gear: Gear) -> Self {
    Self { gear, count: 1, cooldown: 0, timer: 0 }
  }

  pub fn ability(gear: Gear, cooldown: u32) -> Self {
    Self { gear, count: 1, cooldown, timer: 0 }
  }

  pub fn stacked(gear: Gear, count: u32) -> Self {
    Self { gear, count, cooldown: 0, timer: 0 }
  }
}

#[derive(Component, Clone, Debug, Default)]
pub struct Loadout {
  pub gear: Vec<GearSlot>,
}

impl Loadout {
  pub fn new(gear: Vec<GearSlot>) -> Self { Self { gear } }

  pub fn weapon(&self) -> Option<crate::level::Item> {
    self.gear.iter().find_map(|s| match s.gear {
      Gear::Weapon(item) => Some(item),
      _ => None
    })
  }

  pub fn weapon_attack_bonus(&self) -> i32 {
    self.weapon().map(|w| w.attack_bonus()).unwrap_or(0)
  }

  pub fn armor_item(&self) -> Option<crate::level::Item> {
    self.gear.iter().find_map(|s| match s.gear {
      Gear::Armor(item) => Some(item),
      _ => None
    })
  }

  pub fn armor_dr(&self) -> i32 {
    self.gear.iter().map(|s| match s.gear {
      Gear::Armor(item) => item.defense_bonus(),
      Gear::NaturalArmor { dr } => dr,
      _ => 0
    }).sum()
  }

  pub fn grenade_slots(&self) -> Vec<(usize, crate::level::Item)> {
    self.gear.iter().enumerate().filter_map(|(i, s)| match s.gear {
      Gear::Grenade(item) => Some((i, item)),
      _ => None
    }).collect()
  }

  pub fn grenade_at(&self, idx: usize) -> Option<crate::level::Item> {
    self.grenade_slots().get(idx).map(|&(_, item)| item)
  }

  pub fn gun_mut(&mut self) -> Option<&mut GearSlot> {
    self.gear.iter_mut().find(|s| matches!(s.gear, Gear::InnateGun { .. }))
  }

  pub fn grenade_throw_mut(&mut self) -> Option<&mut GearSlot> {
    self.gear.iter_mut().find(|s| matches!(s.gear, Gear::InnateGrenadeThrow { .. }))
  }

  pub fn spore_mut(&mut self) -> Option<&mut GearSlot> {
    self.gear.iter_mut().find(|s| matches!(s.gear, Gear::InnateSporeEmit))
  }

  pub fn weapon_count(&self) -> u32 {
    self.gear.iter().filter(|s| s.gear.is_weapon()).count() as u32
  }

  pub fn grenade_count(&self) -> u32 {
    self.gear.iter().filter(|s| s.gear.is_grenade()).map(|s| s.count).sum()
  }

  pub fn max_weapons(&self) -> u32 {
    1 + self.gear.iter().map(|s| s.gear.weapon_capacity_bonus()).sum::<u32>()
  }

  pub fn max_grenades(&self) -> u32 {
    3 + self.gear.iter().map(|s| s.gear.grenade_capacity_bonus()).sum::<u32>()
  }

  pub fn is_valid(&self) -> bool {
    self.weapon_count() <= self.max_weapons()
      && self.grenade_count() <= self.max_grenades()
  }

  pub fn can_add(&self, gear: Gear) -> bool {
    match gear {
      Gear::Weapon(_) => self.weapon_count() < self.max_weapons(),
      Gear::Grenade(_) => self.grenade_count() < self.max_grenades(),
      _ => true
    }
  }

  pub fn equip_weapon(&mut self, item: crate::level::Item) {
    self.gear.retain(|s| !s.gear.is_weapon());
    self.gear.push(GearSlot::passive(Gear::Weapon(item)));
  }

  pub fn unequip_weapon(&mut self) -> Option<crate::level::Item> {
    let w = self.weapon();
    self.gear.retain(|s| !s.gear.is_weapon());
    w
  }

  pub fn equip_armor(&mut self, item: crate::level::Item) {
    self.gear.retain(|s| !matches!(s.gear, Gear::Armor(_)));
    self.gear.push(GearSlot::passive(Gear::Armor(item)));
  }

  pub fn unequip_armor(&mut self) -> Option<crate::level::Item> {
    let a = self.armor_item();
    self.gear.retain(|s| !matches!(s.gear, Gear::Armor(_)));
    a
  }

  pub fn equip_grenade(&mut self, item: crate::level::Item) {
    self.gear.push(GearSlot::passive(Gear::Grenade(item)));
  }

  pub fn unequip_grenade_at(&mut self, slot_idx: usize) -> Option<crate::level::Item> {
    let slots: Vec<usize> = self.gear.iter().enumerate()
      .filter(|(_, s)| s.gear.is_grenade())
      .map(|(i, _)| i)
      .collect();
    slots.get(slot_idx).map(|&real_idx| {
      let item = match self.gear[real_idx].gear {
        Gear::Grenade(item) => item,
        _ => unreachable!()
      };
      self.gear.remove(real_idx);
      item
    })
  }

  pub fn remove_grenade_by_item(&mut self, item: crate::level::Item) {
    if let Some(idx) = self.gear.iter().position(|s| s.gear == Gear::Grenade(item)) {
      self.gear.remove(idx);
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
pub struct GroundItem(pub crate::level::Item);

/// A door that can be opened/closed.
#[derive(Component, Clone)]
pub struct Door {
  pub open: bool,
  /// Original colour when closed; restored on close.
  pub closed_color: Color
}

/// Marks a door as an airlock: auto-closes after a delay.
#[derive(Component, Clone)]
pub struct AirlockDoor {
  pub opened_at_sim_time: Option<u64>
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

/// An elevator that transports the player to another z-level.
/// `floors` lists every connected deck as (deck_index, local_x, local_y).
#[derive(Component, Clone)]
pub struct Elevator {
  pub current_z: usize,
  pub floors: Vec<(usize, i32, i32)>,
}

/// Placed loot container; blocks the tile until emptied.
#[derive(Component, Clone, Debug)]
pub struct LootChest {
  pub opened: bool
}

/// Optional override for a [`LootChest`]: gives exactly these items instead of proc-gen loot.
#[derive(Component, Clone, Debug)]
pub struct FixedChestLoot(pub &'static [(crate::level::Item, u32)]);

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
  pub sprite_palette: Option<(Color, Color)>
}

impl Glyph {
  pub fn ascii(ch: char, color: Color) -> Self {
    Self { ch, color, texture: None, sprite_palette: None }
  }

  pub fn sprite(path: &'static str, ch: char, color: Color) -> Self {
    Self { ch, color, texture: Some(path), sprite_palette: None }
  }

  /// Mask PNG (black / white / alpha); instance colors set how it draws.
  pub fn palette_sprite(
    path: &'static str,
    ch: char,
    primary: Color,
    secondary: Color
  ) -> Self {
    Self {
      ch,
      color: primary,
      texture: Some(path),
      sprite_palette: Some((primary, secondary))
    }
  }
}

/// Identity and SS13-style flavor text shown on hover.
#[derive(Component, Clone, Debug)]
pub struct Named {
  pub name: &'static str,
  pub flavor: &'static str
}

/// Flat combat stats.
#[derive(Component, Clone, Debug)]
pub struct Stats {
  pub hp: i32,
  pub max_hp: i32,
  pub attack: i32,
  pub move_speed: f32,
  pub attack_speed: f32
}


/// Tracks display frames since the entity last acted. Used by enemy AI.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct TimeSinceAction(pub u32);

/// Marker: this entity is affected by gravity and will fall through Air tiles.
#[derive(Component, Clone, Copy, Debug)]
pub struct Gravity;

/// Per-move probability of stepping to a random walkable neighbor instead of toward the player.
#[derive(Component, Clone, Copy, Debug)]
pub struct DriftChance(pub f32);

/// NPC wander behavior: move to a random adjacent passable tile every `interval` sim steps.
#[derive(Component, Clone, Copy, Debug)]
pub struct WalkAroundRandomly {
  pub timer: u32,
  pub interval: u32
}

/// Recruitable NPC companion state. A separate component so it can be swapped via `Commands::insert`.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub enum FollowerState {
  Available,
  Following,
  Dismissed
}

/// Movement data for a recruitable companion. Home is set by `init_follower_homes` at startup.
#[derive(Component, Clone, Debug)]
pub struct FollowerData {
  /// Tile coords the NPC returns to when dismissed.
  pub home: (i32, i32, usize),
  pub move_timer: u32
}

/// Cached A* path for an entity. Holds the next steps toward the goal and the goal position
/// used to compute them. Cleared when the goal moves far enough to warrant a recompute.
#[derive(Component, Clone, Debug, Default)]
pub struct Path {
  pub steps: VecDeque<(i32, i32)>,
  pub cached_goal: Option<(i32, i32)>
}

/// Marker component for the flight console entity.
#[derive(Component, Clone, Copy)]
pub struct FlightConsole;

/// Marker for the loadout console in the player ship.
#[derive(Component, Clone, Copy)]
pub struct LoadoutConsole;


/// Lingering area-of-effect cloud that damages the player each tick while they share a tile.
/// Used by both spore clouds and explosion clouds.
#[derive(Component, Clone, Copy, Debug)]
pub struct DamageCloud {
  pub damage_per_tick: i32,
  pub ticks_remaining: u32,
  pub tick_interval: u32,
  pub tick_timer: u32
}


/// A grenade lobbed by the player, traveling tile-by-tile toward its target.
/// On each sim step it advances `tiles_per_turn` along `path`; when it reaches the end
/// it detonates (spawns an explosion at the final tile) and despawns.
#[derive(Component, Debug)]
pub struct GrenadeInFlight {
  pub path: Vec<(i32, i32)>,
  pub step: usize,
  pub tiles_per_turn: usize,
  pub z: usize
}

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
  pub last_pos: Vec2
}

// ============ SPAWNABLE ============

const DOOR_CLOSED_PRI: Color = Color::srgb(0.34, 0.37, 0.41);
const DOOR_CLOSED_SEC: Color = Color::srgb(0.52, 0.55, 0.58);

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

/// Space Qud–style NPC silhouette mask (`person (2).png`).
pub fn npc_person_glyph(ch: char, primary: Color, secondary: Color) -> Glyph {
  Glyph::palette_sprite("textures/space_qud/person (2).png", ch, primary, secondary)
}

/// Space Qud–style robot silhouette mask (`robo (1).png`).
pub fn npc_robo_glyph(ch: char, primary: Color, secondary: Color) -> Glyph {
  Glyph::palette_sprite("textures/space_qud/robo (1).png", ch, primary, secondary)
}

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

  /// NPC base: Neutral faction, non-blocking, wanders slowly.
  pub fn npc() -> Self {
    Self::new((
      Collidable(false),
      Character,
      FactionComp(Faction::Neutral),
      Gravity,
      WalkAroundRandomly { timer: 0, interval: 8 }
    ))
  }

  /// Mark this NPC as a recruitable follower. `init_follower_homes` sets the home position at startup.
  pub fn as_follower(self) -> Self {
    self.add(FollowerState::Available)
        .add(FollowerData { home: (0, 0, 0), move_timer: 0 })
        .add(Path::default())
  }

  /// Fully-defined NPC: named, statted, equipped, visible, conversable.
  pub fn defined_npc(
    named: Named,
    stats: Stats,
    loadout: Loadout,
    glyph: Glyph,
    dialogue: &'static DialogueTree
  ) -> Self {
    Self::npc()
      .add(named)
      .add(stats)
      .add(loadout)
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

  pub fn physical(blocks: bool) -> Self { Self::new(Collidable(blocks)) }
  pub fn character(faction: Faction) -> Self {
    Self::physical(true).add((Character, FactionComp(faction), Gravity))
  }
  pub fn player() -> Self { Self::character(Faction::Player).add(Player) }
  pub fn enemy() -> Self {
    Self::character(Faction::Hostile).add((Enemy, TimeSinceAction(0), Path::default()))
  }
  pub fn structure(blocks: bool) -> Self { Self::physical(blocks) }
  pub fn wall(material: Material) -> Self {
    Self::structure(true).add(WallComp { material })
  }
  pub fn tree() -> Self {
    Self::structure(false).add((
      Tree,
      BlocksSight,
      Glyph::palette_sprite(
        "textures/space_qud/tree.png",
        'T',
        Color::srgb(0.14, 0.42, 0.16),
        Color::srgb(0.38, 0.62, 0.24)
      ),
      Named { name: "Tree", flavor: "A sturdy tree. Could be chopped for wood." }
    ))
  }
  pub fn flight_console() -> Self {
    Self::structure(true).add((
      Glyph::palette_sprite(
        "textures/space_qud/computer .png",
        'C',
        Color::srgb(0.18, 0.34, 0.52),
        Color::srgb(0.32, 0.88, 0.45)
      ),
      Named {
        name: "Flight Console",
        flavor: "Navigation computer. Plot a course to a destination."
      },
      FlightConsole
    ))
  }
  pub fn loadout_console() -> Self {
    Self::structure(true).add((
      Glyph::palette_sprite(
        "textures/space_qud/locker (1).png",
        'Q',
        Color::srgb(0.25, 0.38, 0.52),
        Color::srgb(0.55, 0.75, 0.88)
      ),
      Named {
        name: "Loadout Console",
        flavor: "Manage your equipped weapon and armor from your collected gear."
      },
      LoadoutConsole
    ))
  }

  pub fn space_cat() -> Self {
    Self::structure(false).add((
      Glyph::palette_sprite(
        "textures/space_qud/space cat.png",
        'c',
        Color::srgb(0.92, 0.82, 0.62),
        Color::srgb(0.52, 0.36, 0.26)
      ),
      Named {
        name: "Space cat",
        flavor: "Judges your piloting from a warm bulkhead. Offers no corrections."
      }
    ))
  }
  pub fn elevator(current_z: usize, floors: Vec<(usize, i32, i32)>) -> Self {
    Self::structure(true)
      .add(Elevator { current_z, floors })
      .add(Glyph::palette_sprite(
        "textures/space_qud/elevator.png",
        'E',
        Color::srgb(0.42, 0.46, 0.50),
        Color::srgb(1.0, 0.85, 0.10),
      ))
      .add(Named { name: "Elevator", flavor: "Vertical transport. Choose a deck." })
  }

  pub fn loot_chest() -> Self {
    Self::structure(true).add((
      LootChest { opened: false },
      Glyph::palette_sprite(
        "textures/space_qud/crate.png",
        '&',
        Color::srgb(0.72, 0.52, 0.28),
        Color::srgb(0.42, 0.32, 0.22)
      ),
      Named { name: "Chest", flavor: "Someone stashed supplies here." }
    ))
  }
  pub fn door() -> Self {
    Self::structure(true)
      .add(Door { open: false, closed_color: DOOR_CLOSED_PRI })
      .add(BlocksSight)
      .add(Glyph::palette_sprite(
        "textures/space_qud/door closed (1).png",
        '+',
        DOOR_CLOSED_PRI,
        DOOR_CLOSED_SEC,
      ))
      .add(Named { name: "Door", flavor: "Press Space to open." })
  }

  pub fn airlock_door() -> Self {
    Self::door()
      .add(AirlockDoor { opened_at_sim_time: None })
      .add(Glyph::palette_sprite(
        "textures/space_qud/airlock closed.png",
        '+',
        crate::AIRLOCK_PRI,
        crate::AIRLOCK_SEC
      ))
  }
  pub fn ground_item(item: crate::level::Item) -> Self { Self::new(GroundItem(item)) }
  pub fn torch(radius: u32) -> Self { Self::new(LightSource { radius }) }

  pub fn rat_soldier() -> Self {
    Self::enemy()
      .add((
        Named {
          name: "Rat Soldier",
          flavor: "A wiry rat-person clutching a crude spear. Smells like wet fur and old iron.",
        },
        Stats { hp: 10, max_hp: 10, attack: 3, move_speed: 2.1, attack_speed: 1.0 },
        Loadout::new(vec![GearSlot::passive(Gear::Weapon(crate::level::Item::CombatSpear))]),
        Glyph::palette_sprite(
          "textures/space_qud/gunman .png",
          'r',
          Color::srgb(0.72, 0.48, 0.28),
          Color::srgb(0.95, 0.78, 0.55),
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
        Stats { hp: 10, max_hp: 10, attack: 3, move_speed: 1.9, attack_speed: 1.0 },
        Loadout::new(vec![
          GearSlot::passive(Gear::Weapon(crate::level::Item::CombatSpear)),
          GearSlot::passive(Gear::NaturalArmor { dr: 1 }),
        ]),
        Glyph::palette_sprite(
          "textures/space_qud/mogussy.png",
          'r',
          Color::srgb(0.55, 0.42, 0.28),
          Color::srgb(0.82, 0.68, 0.45),
        ),
      ))
  }

  pub fn boulder() -> Self {
    Self::structure(true).add((
      Glyph::palette_sprite(
        "textures/space_qud/rock.png",
        'o',
        Color::srgb(0.32, 0.30, 0.28),
        Color::srgb(0.58, 0.55, 0.50)
      ),
      Named { name: "Boulder", flavor: "A massive rock. Immovable." }
    ))
  }

  pub fn bed() -> Self {
    Self::structure(true).add((
      Glyph::palette_sprite(
        "textures/space_qud/bed.png",
        'b',
        Color::srgb(0.52, 0.38, 0.22),
        Color::srgb(0.88, 0.84, 0.72)
      ),
      Named { name: "Bed", flavor: "A place to sleep. Looks like it hasn't been used in a while." }
    ))
  }

  pub fn table() -> Self {
    Self::structure(true).add((
      Glyph::palette_sprite(
        "textures/space_qud/table.png",
        't',
        Color::srgb(0.48, 0.34, 0.18),
        Color::srgb(0.72, 0.58, 0.36)
      ),
      Named { name: "Table", flavor: "A sturdy table." }
    ))
  }

  pub fn chair() -> Self {
    Self::structure(false).add((
      Glyph::palette_sprite(
        "textures/space_qud/chair (1).png",
        'h',
        Color::srgb(0.60, 0.62, 0.65),
        Color::srgb(0.72, 0.18, 0.14)
      ),
      Named { name: "Chair", flavor: "A chair. Something to sit on." }
    ))
  }

  pub fn locker() -> Self {
    Self::structure(true).add((
      Glyph::palette_sprite(
        "textures/space_qud/locker (2).png",
        'l',
        Color::srgb(0.32, 0.38, 0.42),
        Color::srgb(0.62, 0.68, 0.72)
      ),
      Named { name: "Locker", flavor: "A metal locker. Whatever was inside is long gone." }
    ))
  }

  pub fn crate_obj() -> Self {
    Self::structure(true).add((
      Glyph::palette_sprite(
        "textures/space_qud/crate.png",
        'c',
        Color::srgb(0.42, 0.32, 0.18),
        Color::srgb(0.72, 0.60, 0.38)
      ),
      Named { name: "Crate", flavor: "A battered storage crate. Probably empty." }
    ))
  }

  /// A ship-side supply cache — loot chest with fixed starter gear.
  pub fn supply_cache(contents: &'static [(crate::level::Item, u32)]) -> Self {
    Self::new((
      Collidable(true),
      LootChest { opened: false },
      FixedChestLoot(contents),
      Glyph::palette_sprite(
        "textures/space_qud/crate.png",
        'S',
        Color::srgb(0.28, 0.42, 0.52),
        Color::srgb(0.52, 0.75, 0.88)
      ),
      Named {
        name: "Supply Cache",
        flavor: "A sealed cache. Whoever left this behind had plans they didn't finish."
      }
    ))
  }

  pub fn robot() -> Self {
    Self::enemy().add((
      Named {
        name: "Robot",
        flavor: "A damaged security robot. Its threat-response routines are still very much active."
      },
      Stats { hp: 15, max_hp: 15, attack: 4, move_speed: 2.0, attack_speed: 0.8 },
      Loadout::default(),
      Glyph::palette_sprite(
        "textures/space_qud/robo.png",
        'R',
        Color::srgb(0.28, 0.52, 0.58),
        Color::srgb(0.55, 0.82, 0.88)
      )
    ))
  }

  pub fn wack_robot() -> Self {
    Self::enemy().add((
      Named {
        name: "Salvage Bot",
        flavor: "A repurposed salvage drone running corrupted directives. Approaches everything as scrap."
      },
      Stats { hp: 8, max_hp: 8, attack: 3, move_speed: 2.3, attack_speed: 1.2 },
      Loadout::default(),
      Glyph::palette_sprite(
        "textures/space_qud/wack robo.png",
        'R',
        Color::srgb(0.62, 0.38, 0.18),
        Color::srgb(0.88, 0.68, 0.32)
      )
    ))
  }

  pub fn alien_runner() -> Self {
    Self::enemy().add((
      Named {
        name: "Xel-Naran Hunter",
        flavor: "A fast-moving predator native to Xel-Nara IV. Moves in bursts. Closes distance before you can react."
      },
      Stats { hp: 5, max_hp: 5, attack: 3, move_speed: 12.0, attack_speed: 1.5 },
      DriftChance(0.3),
      Loadout::default(),
      Glyph::palette_sprite(
        "textures/space_qud/alien1.png",
        'x',
        Color::srgb(0.18, 0.72, 0.22),
        Color::srgb(0.92, 0.82, 0.18)
      ),
    ))
  }

  pub fn lava_crab() -> Self {
    Self::enemy().add((
      Named {
        name: "Scorch Crawler",
        flavor: "A heat-adapted crustacean from Pyros Maw. Its shell has fused with volcanic rock over generations. Barely slowed by flame."
      },
      Stats { hp: 14, max_hp: 14, attack: 5, move_speed: 4.0, attack_speed: 0.9 },
      DriftChance(0.05),
      Loadout::new(vec![GearSlot::passive(Gear::NaturalArmor { dr: 3 })]),
      Glyph::palette_sprite(
        "textures/space_qud/crab alien.png",
        'c',
        Color::srgb(0.85, 0.25, 0.05),
        Color::srgb(1.0, 0.55, 0.0)
      ),
    ))
  }

  pub fn mantis_alien() -> Self {
    Self::enemy().add((
      Named {
        name: "Crystal Mantis",
        flavor: "A translucent predator that haunts crystal caves, nearly invisible until it strikes. Razor forelegs. Extremely fast."
      },
      Stats { hp: 6, max_hp: 6, attack: 5, move_speed: 10.0, attack_speed: 2.0 },
      DriftChance(0.5),
      Loadout::default(),
      Glyph::palette_sprite(
        "textures/space_qud/mantis alien.png",
        'M',
        Color::srgb(0.65, 0.90, 0.95),
        Color::srgb(0.20, 0.55, 0.70)
      ),
    ))
  }

  pub fn crab_alien() -> Self {
    Self::enemy().add((
      Named {
        name: "Xel-Naran Crawler",
        flavor: "A broad-shelled crustacean that lurks in alien undergrowth. Its claws can crush bone. Slow but armored."
      },
      Stats { hp: 10, max_hp: 10, attack: 4, move_speed: 3.5, attack_speed: 0.8 },
      DriftChance(0.1),
      Loadout::new(vec![GearSlot::passive(Gear::NaturalArmor { dr: 1 })]),
      Glyph::palette_sprite(
        "textures/space_qud/crab alien.png",
        'c',
        Color::srgb(0.55, 0.18, 0.72),
        Color::srgb(0.92, 0.72, 0.18)
      ),
    ))
  }

  pub fn mushroom_creature() -> Self {
    Self::enemy().add((
      Named {
        name: "Mycelid",
        flavor: "An ambulatory fungal mass. Moves with unsettling purpose. Its gills swell with spores."
      },
      Stats { hp: 6, max_hp: 6, attack: 2, move_speed: 2.0, attack_speed: 0.6 },
      Loadout::new(vec![GearSlot::ability(Gear::InnateSporeEmit, 40)]),
      Glyph::palette_sprite(
        "textures/space_qud/mushroom.png",
        'm',
        Color::srgb(0.42, 0.28, 0.18),
        Color::srgb(0.82, 0.72, 0.55)
      ),
    ))
  }

  fn damage_cloud(
    glyph: Glyph,
    name: &'static str,
    flavor: &'static str,
    damage_per_tick: i32,
    ticks_remaining: u32,
    tick_interval: u32
  ) -> Self {
    Self::new((
      Collidable(false),
      DamageCloud { damage_per_tick, ticks_remaining, tick_interval, tick_timer: 0 },
      glyph,
      Named { name, flavor }
    ))
  }

  pub fn spore_cloud() -> Self {
    Self::damage_cloud(
      Glyph::palette_sprite(
        "textures/space_qud/checkerboard pattern.png",
        '*',
        Color::srgb(0.30, 0.72, 0.22),
        Color::srgb(0.18, 0.48, 0.12)
      ),
      "Spore Cloud",
      "A drifting cloud of toxic fungal spores.",
      1, 4, 5
    )
  }

  pub fn explosion_cloud() -> Self {
    Self::damage_cloud(
      Glyph::palette_sprite(
        "textures/space_qud/checkerboard pattern.png",
        '*',
        Color::srgb(0.95, 0.55, 0.10),
        Color::srgb(0.72, 0.22, 0.06)
      ),
      "Explosion",
      "Roiling flame and shrapnel.",
      3, 2, 2
    )
  }

  pub fn grenade_thrower() -> Self {
    Self::enemy().add((
      Named {
        name: "Grenadier",
        flavor: "A wiry soldier bristling with grenades. Keeps its distance."
      },
      Stats { hp: 8, max_hp: 8, attack: 2, move_speed: 2.0, attack_speed: 0.8 },
      Loadout::new(vec![GearSlot::ability(Gear::InnateGrenadeThrow { min_range: 3 }, 25)]),
      Glyph::palette_sprite(
        "textures/space_qud/gunman .png",
        'g',
        Color::srgb(0.22, 0.48, 0.22),
        Color::srgb(0.60, 0.78, 0.42)
      ),
    ))
  }

  pub fn gunman() -> Self {
    Self::enemy().add((
      Named {
        name: "Gunman",
        flavor: "A sharp-eyed mercenary with a revolver. Shoots first."
      },
      Stats { hp: 8, max_hp: 8, attack: 3, move_speed: 2.0, attack_speed: 1.0 },
      Loadout::new(vec![GearSlot::ability(Gear::InnateGun { damage: 4 }, 15)]),
      Glyph::palette_sprite(
        "textures/space_qud/gunman .png",
        'g',
        Color::srgb(0.42, 0.52, 0.68),
        Color::srgb(0.72, 0.82, 0.92)
      ),
    ))
  }

  pub fn mushroom(primary: Color, secondary: Color, name: &'static str) -> Self {
    Self::structure(false).add((
      Glyph::palette_sprite("textures/space_qud/mushroom.png", 'm', primary, secondary),
      Named { name, flavor: "A large fungal growth rooted in the alien soil." }
    ))
  }

  pub fn laser_sword() -> Self {
    Self::structure(false).add((
      Glyph::palette_sprite(
        "textures/space_qud/laser sword.png",
        '/',
        Color::srgb(0.18, 0.08, 0.52),
        Color::srgb(0.42, 0.82, 0.98)
      ),
      Named { name: "Laser Sword", flavor: "An energy blade, dormant. Still hums faintly." }
    ))
  }
}
