//! Entity types and spawnable definitions for the game.

use {crate::faction::Faction,
     bevy::prelude::*,
     std::{borrow::Cow, collections::VecDeque, sync::Arc}};

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
pub const fn dialogue_tree(nodes: &'static [DialogueNode]) -> DialogueTree {
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
#[derive(Component, Clone, Debug, PartialEq)]
pub enum Location {
  /// At specific tile coordinates on z-level `z`.
  Coords { x: i32, y: i32, z: usize },
  /// In another entity's inventory.
  Inventory(Entity),
  /// Not placed anywhere (template, UI preview, etc.).
  Nowhere
}

impl Location {
  pub fn xyz(x: i32, y: i32, z: usize) -> Self {
    Location::Coords { x, y, z }
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
  Device(crate::level::Item),
  Loot(crate::level::Item),
  InnateGun { damage: i32 },
  InnateGrenadeThrow { min_range: i32 },
  InnateSporeEmit,
  InnateGrab,
  NaturalArmor { dr: i32 }
}

impl Gear {
  pub fn is_weapon(self) -> bool { matches!(self, Gear::Weapon(_)) }
  pub fn is_armor(self) -> bool {
    matches!(self, Gear::Armor(_) | Gear::NaturalArmor { .. })
  }
  pub fn is_grenade(self) -> bool { matches!(self, Gear::Grenade(_)) }
  pub fn is_ability(self) -> bool {
    matches!(
      self,
      Gear::InnateGun { .. }
        | Gear::InnateGrenadeThrow { .. }
        | Gear::InnateSporeEmit
        | Gear::InnateGrab
    )
  }

  pub fn weapon_capacity_bonus(self) -> u32 { 0 }
  pub fn grenade_capacity_bonus(self) -> u32 { 0 }
}

#[derive(Clone, Copy, Debug)]
pub struct GearSlot {
  pub gear: Gear,
  pub count: u32,
  pub cooldown: u32,
  pub timer: u32
}

impl GearSlot {
  pub const fn passive(gear: Gear) -> Self { Self { gear, count: 1, cooldown: 0, timer: 0 } }

  pub const fn ability(gear: Gear, cooldown: u32) -> Self {
    Self { gear, count: 1, cooldown, timer: 0 }
  }

  pub const fn stacked(gear: Gear, count: u32) -> Self {
    Self { gear, count, cooldown: 0, timer: 0 }
  }
}

#[derive(Component, Clone, Debug, Default)]
pub struct Loadout {
  pub gear: Cow<'static, [GearSlot]>
}

impl Loadout {
  pub fn new(gear: impl Into<Cow<'static, [GearSlot]>>) -> Self { Self { gear: gear.into() } }

  pub const fn from_gear(gear: &'static [GearSlot]) -> Self { Self { gear: Cow::Borrowed(gear) } }

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
    self
      .gear
      .iter()
      .map(|s| match s.gear {
        Gear::Armor(item) => item.defense_bonus(),
        Gear::NaturalArmor { dr } => dr,
        _ => 0
      })
      .sum()
  }

  pub fn grenade_slots(&self) -> Vec<(usize, crate::level::Item)> {
    self
      .gear
      .iter()
      .enumerate()
      .filter_map(|(i, s)| match s.gear {
        Gear::Grenade(item) => Some((i, item)),
        _ => None
      })
      .collect()
  }

  pub fn grenade_at(&self, idx: usize) -> Option<crate::level::Item> {
    self.grenade_slots().get(idx).map(|&(_, item)| item)
  }

  pub fn device_slots(&self) -> Vec<(usize, crate::level::Item)> {
    self
      .gear
      .iter()
      .enumerate()
      .filter_map(|(i, s)| match s.gear {
        Gear::Device(item) => Some((i, item)),
        _ => None
      })
      .collect()
  }

  pub fn gun_mut(&mut self) -> Option<&mut GearSlot> {
    self.gear.to_mut().iter_mut().find(|s| matches!(s.gear, Gear::InnateGun { .. }))
  }

  pub fn grenade_throw_mut(&mut self) -> Option<&mut GearSlot> {
    self
      .gear
      .to_mut()
      .iter_mut()
      .find(|s| matches!(s.gear, Gear::InnateGrenadeThrow { .. }))
  }

  pub fn spore_mut(&mut self) -> Option<&mut GearSlot> {
    self.gear.to_mut().iter_mut().find(|s| matches!(s.gear, Gear::InnateSporeEmit))
  }

  pub fn grab_mut(&mut self) -> Option<&mut GearSlot> {
    self.gear.to_mut().iter_mut().find(|s| matches!(s.gear, Gear::InnateGrab))
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

  pub fn can_add(&self, gear: Gear) -> bool { self.rejection_reason(gear).is_none() }

  pub fn rejection_reason(&self, gear: Gear) -> Option<String> {
    match gear {
      Gear::Weapon(_) if self.weapon_count() >= self.max_weapons() => {
        Some(format!("weapon slot full ({}/{})", self.weapon_count(), self.max_weapons()))
      }
      Gear::Armor(_) if self.armor_item().is_some() => Some("armor slot full".into()),
      Gear::Grenade(_) if self.grenade_count() >= self.max_grenades() => Some(format!(
        "grenade slots full ({}/{})",
        self.grenade_count(),
        self.max_grenades()
      )),
      _ => None
    }
  }

  pub fn equip_weapon(&mut self, item: crate::level::Item) {
    self.gear.to_mut().push(GearSlot::passive(Gear::Weapon(item)));
  }

  pub fn unequip_weapon(&mut self) -> Option<crate::level::Item> {
    let w = self.weapon();
    self.gear.to_mut().retain(|s| !s.gear.is_weapon());
    w
  }

  pub fn equip_armor(&mut self, item: crate::level::Item) {
    self.gear.to_mut().push(GearSlot::passive(Gear::Armor(item)));
  }

  pub fn unequip_armor(&mut self) -> Option<crate::level::Item> {
    let a = self.armor_item();
    self.gear.to_mut().retain(|s| !matches!(s.gear, Gear::Armor(_)));
    a
  }

  pub fn equip_grenade(&mut self, item: crate::level::Item) {
    self.gear.to_mut().push(GearSlot::passive(Gear::Grenade(item)));
  }

  pub fn unequip_grenade_at(&mut self, slot_idx: usize) -> Option<crate::level::Item> {
    let slots: Vec<usize> = self
      .gear
      .iter()
      .enumerate()
      .filter(|(_, s)| s.gear.is_grenade())
      .map(|(i, _)| i)
      .collect();
    slots.get(slot_idx).map(|&real_idx| {
      let gear = self.gear.to_mut();
      let item = match gear[real_idx].gear {
        Gear::Grenade(item) => item,
        _ => unreachable!()
      };
      gear.remove(real_idx);
      item
    })
  }

  pub fn equip_device(&mut self, item: crate::level::Item) {
    self.gear.to_mut().push(GearSlot::stacked(Gear::Device(item), 1));
  }

  pub fn unequip_device_at(&mut self, slot_idx: usize) -> Option<crate::level::Item> {
    let slots: Vec<usize> = self
      .gear
      .iter()
      .enumerate()
      .filter(|(_, s)| matches!(s.gear, Gear::Device(_)))
      .map(|(i, _)| i)
      .collect();
    slots.get(slot_idx).map(|&real_idx| {
      let gear = self.gear.to_mut();
      let item = match gear[real_idx].gear {
        Gear::Device(item) => item,
        _ => unreachable!()
      };
      gear.remove(real_idx);
      item
    })
  }

  pub fn remove_grenade_by_item(&mut self, item: crate::level::Item) {
    if let Some(idx) = self.gear.iter().position(|s| s.gear == Gear::Grenade(item)) {
      self.gear.to_mut().remove(idx);
    }
  }

  pub fn remove_device_by_item(&mut self, item: crate::level::Item) {
    if let Some(idx) = self.gear.iter().position(|s| s.gear == Gear::Device(item)) {
      self.gear.to_mut().remove(idx);
    }
  }

  pub fn retain_gear(&mut self, keep: impl FnMut(&GearSlot) -> bool) {
    self.gear.to_mut().retain(keep);
  }

  pub fn lootable_items(&self) -> Vec<(crate::level::Item, u32)> {
    self
      .gear
      .iter()
      .filter_map(|s| match s.gear {
        Gear::Weapon(item)
        | Gear::Armor(item)
        | Gear::Grenade(item)
        | Gear::Device(item)
        | Gear::Loot(item) => Some((item, s.count)),
        _ => None
      })
      .collect()
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

/// A dead creature whose inventory can be looted.
#[derive(Component, Clone, Debug)]
pub struct Corpse {
  pub loot: Vec<(crate::level::Item, u32)>,
  pub looted: bool
}

/// A tree entity.
#[derive(Component, Clone, Copy)]
pub struct Tree;

/// A bed the player can sleep in to save their game.
#[derive(Component, Clone, Copy)]
pub struct Bed;

/// An elevator that transports the player to another z-level.
/// `floors` lists every connected deck as (deck_index, local_x, local_y).
#[derive(Component, Clone)]
pub struct Elevator {
  pub current_z: usize,
  pub floors: Cow<'static, [(usize, i32, i32)]>
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

/// Sprite animation. `idle` is the standing texture; `idle_frames` cycle with `idle` when
/// stationary (idle → idle_0 → idle → idle_1 → …). `walk_frames` cycle the same way while
/// moving. Either list can be empty to just show `idle`.
#[derive(Component, Clone, Debug)]
pub struct WalkAnim {
  pub idle: &'static str,
  pub idle_frames: &'static [&'static str],
  pub walk_frames: &'static [&'static str],
  pub interval: u64,
  pub idle_interval: u64
}

/// Visual for a grid entity: optional PNG (tile-sized sprite) or [`Text2d`] from `ch` + `color`.
#[derive(Component, Clone, Copy, Debug)]
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
  pub const fn palette_sprite(
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
#[derive(Component, Clone, Copy, Debug)]
pub struct Named {
  pub name: &'static str,
  pub flavor: &'static str
}

/// Flat combat stats.
#[derive(Component, Clone, Copy, Debug)]
pub struct Stats {
  pub hp: i32,
  pub max_hp: i32,
  pub attack: i32,
  pub move_speed: f32,
  pub attack_speed: f32
}

/// Tracks sim steps since the entity last attacked / moved. Used by enemy AI.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct TimeSinceAction {
  pub attack: u32,
  pub movement: u32
}

/// Marker: this entity is affected by gravity and will fall through Air tiles.
#[derive(Component, Clone, Copy, Debug)]
pub struct Gravity;

/// Per-move probability of stepping to a random walkable neighbor instead of toward the player.
#[derive(Component, Clone, Copy, Debug)]
pub struct DriftChance(pub f32);

/// The player (or entity) is grabbed by another entity and cannot move.
/// Decremented each sim step; removed when it reaches 0.
#[derive(Component, Clone, Copy, Debug)]
pub struct Grabbed {
  pub by: Entity,
  pub turns_remaining: u32
}

/// Entity is invisible: enemies ignore it, rendered translucent.
/// Decremented each sim step; removed when it reaches 0.
#[derive(Component, Clone, Copy, Debug)]
pub struct Invisible(pub u32);

#[derive(Component, Clone, Copy, Debug)]
pub struct Phasing(pub u32);

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

#[derive(Component, Clone, Copy)]
pub struct CraftingTable;

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

// (old Object, ObjectConst, const_blueprint!, object_const! removed — replaced by Object above)

// ============ OBJECT (data-driven entity blueprint) ============

pub const trait FieldOf<S> {
  fn apply_to(self, obj: S) -> S;
}

pub trait Has<T> {
  fn get(&self) -> Option<&T>;
  fn get_mut(&mut self) -> Option<&mut T>;
  fn set(&mut self, val: T);
}

macro_rules! object_data {
  (pub struct $name:ident ( $($ty:ty),* $(,)? )) => {
    object_data!(
      @pair $name;
      [];
      @idx (0 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 19 20 21 22 23 24 25 26 27 28 29 30 31 32 33 34 35 36 37 38 39);
      @var (a b c d e f g h i j k l m n o p q r s t u v w x y z aa ab ac ad ae af ag ah ai aj ak al am an);
      @ty [ $($ty),+ ] @end
    );
  };

  (
    @pair $name:ident;
    [$(($i:tt, $t:ty, $v:ident))*];
    @idx ($idx:tt $($rest_idx:tt)*);
    @var ($var:ident $($rest_var:ident)*);
    @ty [ $ty:ty, $($rest:ty),+ ] @end
  ) => {
    object_data!(
      @pair $name;
      [$(($i, $t, $v))* ($idx, $ty, $var)];
      @idx ($($rest_idx)*);
      @var ($($rest_var)*);
      @ty [ $($rest),+ ] @end
    );
  };

  (
    @pair $name:ident;
    [$(($i:tt, $t:ty, $v:ident))*];
    @idx ($idx:tt $($rest_idx:tt)*);
    @var ($var:ident $($rest_var:ident)*);
    @ty [ $ty:ty ] @end
  ) => {
    object_data!(@def $name; [ $(($i, $t, $v),)* ($idx, $ty, $var), ]);
  };

  (@pair $name:ident; $pairs:tt; @idx (); @var (); @ty [ $ty:ty $(,)? ] @end) => {
    compile_error!("object_data! supports at most 40 component types");
  };

  (@def $name:ident; [ $(($idx:tt, $ty:ty, $var:ident),)* ]) => {
    #[derive(Clone)]
    pub struct $name($(pub Option<$ty>),*);

    impl $name {
      pub const EMPTY: Self = Self($(None::<$ty>,)*);

      pub const fn with<T: ~const FieldOf<Self>>(self, val: T) -> Self { val.apply_to(self) }

      pub fn delegate(self, other: &Self) -> Self {
        Self {
          $( $idx: self.$idx.or_else(|| other.$idx.clone()), )*
        }
      }

      pub fn insert_into(&self, e: &mut EntityCommands) {
        $(if let Some(val) = self.$idx.clone() { e.insert(val); })*
      }
    }

    $(
      impl Has<$ty> for $name {
        fn get(&self) -> Option<&$ty> { self.$idx.as_ref() }
        fn get_mut(&mut self) -> Option<&mut $ty> { self.$idx.as_mut() }
        fn set(&mut self, val: $ty) { self.$idx = Some(val); }
      }
    )*

    object_data!(@field_impls $name; @before []; @rest [ $(($idx, $ty, $var),)* ]);
  };

  (@field_impls $name:ident;
    @before [ $(($bi:tt, $bt:ty, $bv:ident),)* ];
    @rest [ ($ci:tt, $ct:ty, $cv:ident), $(($ri:tt, $rt:ty, $rv:ident),)* ]
  ) => {
    #[allow(forgetting_copy_types)]
    impl const FieldOf<$name> for $ct {
      fn apply_to(self, $name($($bv,)* _old, $($rv,)*): $name) -> $name {
        std::mem::forget(_old);
        $name($($bv,)* Some(self), $($rv,)*)
      }
    }
    object_data!(@field_impls $name;
      @before [ $(($bi, $bt, $bv),)* ($ci, $ct, $cv), ];
      @rest [ $(($ri, $rt, $rv),)* ]
    );
  };

  (@field_impls $name:ident; @before $b:tt; @rest []) => {};
}

object_data! {
  pub struct ObjectData(
    Named, Stats, Glyph, Loadout, Collidable, Character, FactionComp, Gravity,
    Enemy, Player, TimeSinceAction, DriftChance, WalkAnim, DamageCloud, Dialogue,
    WalkAroundRandomly, BlocksSight,
    Door, Bed, CraftingTable, FlightConsole, LoadoutConsole, LootChest, AirlockDoor,
    Tree, GroundItem, LightSource, WallComp, Elevator, FixedChestLoot,
    FollowerState, FollowerData, Path
  )
}

#[derive(Clone)]
pub struct Object {
  pub data: ObjectData,
  extras: Option<Arc<dyn Fn(&mut EntityCommands) + Send + Sync>>
}

impl Object {
  pub const EMPTY: Self = Self { data: ObjectData::EMPTY, extras: None };

  pub const fn with<T: ~const FieldOf<ObjectData>>(self, val: T) -> Self {
    let Self { data, extras } = self;
    Self { data: data.with(val), extras }
  }

  pub fn add(self, bundle: impl Bundle + Clone + Send + Sync + 'static) -> Self {
    let prev = self.extras;
    Self {
      data: self.data,
      extras: Some(Arc::new(move |e: &mut EntityCommands| {
        if let Some(p) = &prev { p(e); }
        e.insert(bundle.clone());
      }))
    }
  }

  pub fn delegate(self, other: &Self) -> Self {
    let extras = self.extras.clone().or_else(|| other.extras.clone());
    Self { data: self.data.delegate(&other.data), extras }
  }

  pub fn insert_into(&self, e: &mut EntityCommands) {
    self.data.insert_into(e);
    if let Some(extras) = &self.extras { extras(e); }
    if Has::<Enemy>::get(&self.data).is_some() {
      e.insert(Path::default());
    }
  }

  pub fn spawn(&self, commands: &mut Commands) -> Entity {
    let mut e = commands.spawn_empty();
    self.insert_into(&mut e);
    e.id()
  }

  pub fn spawn_at(&self, commands: &mut Commands, x: i32, y: i32, z: usize) -> Entity {
    let mut e = commands.spawn_empty();
    self.insert_into(&mut e);
    e.insert(Location::xyz(x, y, z));
    e.id()
  }
}

impl<T> Has<T> for Object where ObjectData: Has<T> {
  fn get(&self) -> Option<&T> { self.data.get() }
  fn get_mut(&mut self) -> Option<&mut T> { self.data.get_mut() }
  fn set(&mut self, val: T) { self.data.set(val); }
}

// ---- Object blueprints: associated constants and factory methods ----

impl Object {
  // ---- const bases ----

  pub const ENEMY_BASE: Self = Self::EMPTY
    .with(Collidable(true))
    .with(Character)
    .with(FactionComp(Faction::Hostile))
    .with(Gravity)
    .with(Enemy)
    .with(TimeSinceAction { attack: 0, movement: 0 });

  pub const NPC_BASE: Self = Self::EMPTY
    .with(Collidable(false))
    .with(Character)
    .with(FactionComp(Faction::Neutral))
    .with(Gravity)
    .with(WalkAroundRandomly { timer: 0, interval: 8 });

  pub const STRUCTURE: Self = Self::EMPTY.with(Collidable(true));
  pub const STRUCTURE_PASSABLE: Self = Self::EMPTY.with(Collidable(false));

  // ---- enemy definitions ----

  pub const RAT_SOLDIER: Self = Self::ENEMY_BASE
    .with(Named {
      name: "Rat Soldier",
      flavor: "A wiry rat-person clutching a crude spear. Smells like wet fur and old iron.",
    })
    .with(Stats { hp: 10, max_hp: 10, attack: 3, move_speed: 2.1, attack_speed: 1.0 })
    .with(Glyph::palette_sprite(
      "textures/space_qud/gunman .png", 'r',
      Color::srgb(0.72, 0.48, 0.28), Color::srgb(0.95, 0.78, 0.55),
    ))
    .with(Loadout::from_gear(&[
      GearSlot::passive(Gear::Weapon(crate::level::Item::CombatSpear)),
      GearSlot::stacked(Gear::Loot(crate::level::Item::GoldCoin), 2),
    ]));

  pub const ARMORED_RAT_SOLDIER: Self = Self::ENEMY_BASE
    .with(Named {
      name: "Armored Rat Soldier",
      flavor: "A rat-person in battered leather armor, gripping a crude spear. The hide smells worse than the iron.",
    })
    .with(Stats { hp: 10, max_hp: 10, attack: 3, move_speed: 1.9, attack_speed: 1.0 })
    .with(Glyph::palette_sprite(
      "textures/space_qud/mogussy.png", 'r',
      Color::srgb(0.55, 0.42, 0.28), Color::srgb(0.82, 0.68, 0.45),
    ))
    .with(Loadout::from_gear(&[
      GearSlot::passive(Gear::Weapon(crate::level::Item::CombatSpear)),
      GearSlot::passive(Gear::NaturalArmor { dr: 1 }),
      GearSlot::stacked(Gear::Loot(crate::level::Item::GoldCoin), 3),
    ]));

  pub const ROBOT: Self = Self::ENEMY_BASE
    .with(Named {
      name: "Robot",
      flavor: "A damaged security robot. Its threat-response routines are still very much active.",
    })
    .with(Stats { hp: 15, max_hp: 15, attack: 4, move_speed: 2.0, attack_speed: 0.8 })
    .with(Glyph::palette_sprite(
      "textures/space_qud/robo.png", 'R',
      Color::srgb(0.28, 0.52, 0.58), Color::srgb(0.55, 0.82, 0.88),
    ))
    .with(Loadout::from_gear(&[
      GearSlot::stacked(Gear::Loot(crate::level::Item::GoldCoin), 4),
    ]));

  pub const WACK_ROBOT: Self = Self::ENEMY_BASE
    .with(Named {
      name: "Salvage Bot",
      flavor: "A repurposed salvage drone running corrupted directives. Approaches everything as scrap.",
    })
    .with(Stats { hp: 8, max_hp: 8, attack: 3, move_speed: 2.3, attack_speed: 1.2 })
    .with(Glyph::palette_sprite(
      "textures/space_qud/wack robo.png", 'R',
      Color::srgb(0.62, 0.38, 0.18), Color::srgb(0.88, 0.68, 0.32),
    ))
    .with(Loadout::from_gear(&[
      GearSlot::stacked(Gear::Loot(crate::level::Item::GoldCoin), 2),
    ]));

  pub const ALIEN_RUNNER: Self = Self::ENEMY_BASE
    .with(Named {
      name: "Xel-Naran Hunter",
      flavor: "A fast-moving predator native to Xel-Nara IV. Moves in bursts. Closes distance before you can react.",
    })
    .with(Stats { hp: 5, max_hp: 5, attack: 3, move_speed: 12.0, attack_speed: 1.5 })
    .with(DriftChance(0.3))
    .with(Glyph::palette_sprite(
      "textures/space_qud/alien1.png", 'x',
      Color::srgb(0.18, 0.72, 0.22), Color::srgb(0.92, 0.82, 0.18),
    ))
    .with(Loadout::from_gear(&[
      GearSlot::stacked(Gear::Device(crate::level::Item::StealthDevice), 1),
      GearSlot::stacked(Gear::Loot(crate::level::Item::GoldCoin), 1),
    ]))
    .with(WalkAnim {
      idle: "textures/space_qud/alien1.png",
      idle_frames: &["textures/space_qud/alien1 frame 2.png"],
      walk_frames: &["textures/space_qud/alien1 frame 2.png"],
      interval: 20,
      idle_interval: 20,
    });

  pub const LAVA_CRAB: Self = Self::ENEMY_BASE
    .with(Named {
      name: "Scorch Crawler",
      flavor: "A heat-adapted crustacean from Pyros Maw. Its shell has fused with volcanic rock over generations. Barely slowed by flame.",
    })
    .with(Stats { hp: 14, max_hp: 14, attack: 5, move_speed: 4.0, attack_speed: 0.9 })
    .with(DriftChance(0.05))
    .with(Glyph::palette_sprite(
      "textures/space_qud/crab alien.png", 'c',
      Color::srgb(0.85, 0.25, 0.05), Color::srgb(1.0, 0.55, 0.0),
    ))
    .with(Loadout::from_gear(&[
      GearSlot::ability(Gear::InnateGrab, 8),
      GearSlot::passive(Gear::NaturalArmor { dr: 3 }),
      GearSlot::stacked(Gear::Loot(crate::level::Item::GoldCoin), 3),
    ]));

  pub const MANTIS_ALIEN: Self = Self::ENEMY_BASE
    .with(Named {
      name: "Crystal Mantis",
      flavor: "A translucent predator that haunts crystal caves, nearly invisible until it strikes. Razor forelegs. Extremely fast.",
    })
    .with(Stats { hp: 6, max_hp: 6, attack: 5, move_speed: 10.0, attack_speed: 2.0 })
    .with(DriftChance(0.5))
    .with(Glyph::palette_sprite(
      "textures/space_qud/mantis alien.png", 'M',
      Color::srgb(0.65, 0.90, 0.95), Color::srgb(0.20, 0.55, 0.70),
    ))
    .with(Loadout::from_gear(&[
      GearSlot::stacked(Gear::Device(crate::level::Item::StealthDevice), 1),
      GearSlot::stacked(Gear::Loot(crate::level::Item::GoldCoin), 2),
    ]))
    .with(WalkAnim {
      idle: "textures/space_qud/mantis alien.png",
      idle_frames: &["textures/space_qud/mantis alien frame 2.png"],
      walk_frames: &["textures/space_qud/mantis alien frame 2.png"],
      interval: 20,
      idle_interval: 20,
    });

  pub const CRAB_ALIEN: Self = Self::ENEMY_BASE
    .with(Named {
      name: "Xel-Naran Crawler",
      flavor: "A broad-shelled crustacean that lurks in alien undergrowth. Its claws can crush bone. Slow but armored.",
    })
    .with(Stats { hp: 10, max_hp: 10, attack: 4, move_speed: 3.5, attack_speed: 0.8 })
    .with(DriftChance(0.1))
    .with(Glyph::palette_sprite(
      "textures/space_qud/crab alien.png", 'c',
      Color::srgb(0.55, 0.18, 0.72), Color::srgb(0.92, 0.72, 0.18),
    ))
    .with(Loadout::from_gear(&[
      GearSlot::ability(Gear::InnateGrab, 8),
      GearSlot::passive(Gear::NaturalArmor { dr: 1 }),
      GearSlot::stacked(Gear::Loot(crate::level::Item::GoldCoin), 2),
    ]));

  pub const MUSHROOM_CREATURE: Self = Self::ENEMY_BASE
    .with(Named {
      name: "Mycelid",
      flavor: "An ambulatory fungal mass. Moves with unsettling purpose. Its gills swell with spores.",
    })
    .with(Stats { hp: 6, max_hp: 6, attack: 2, move_speed: 2.0, attack_speed: 0.6 })
    .with(Glyph::palette_sprite(
      "textures/space_qud/mushroom.png", 'm',
      Color::srgb(0.42, 0.28, 0.18), Color::srgb(0.82, 0.72, 0.55),
    ))
    .with(Loadout::from_gear(&[
      GearSlot::ability(Gear::InnateSporeEmit, 40),
      GearSlot::stacked(Gear::Loot(crate::level::Item::GoldCoin), 1),
    ]));

  pub const GRENADE_THROWER: Self = Self::ENEMY_BASE
    .with(Named {
      name: "Grenadier",
      flavor: "A wiry soldier bristling with grenades. Keeps its distance.",
    })
    .with(Stats { hp: 8, max_hp: 8, attack: 2, move_speed: 2.0, attack_speed: 0.8 })
    .with(Glyph::palette_sprite(
      "textures/space_qud/gunman .png", 'g',
      Color::srgb(0.22, 0.48, 0.22), Color::srgb(0.60, 0.78, 0.42),
    ))
    .with(Loadout::from_gear(&[
      GearSlot::ability(Gear::InnateGrenadeThrow { min_range: 3 }, 25),
      GearSlot::stacked(Gear::Loot(crate::level::Item::GoldCoin), 3),
    ]));

  pub const GUNMAN: Self = Self::ENEMY_BASE
    .with(Named {
      name: "Gunman",
      flavor: "A sharp-eyed mercenary with a revolver. Shoots first.",
    })
    .with(Stats { hp: 8, max_hp: 8, attack: 3, move_speed: 2.0, attack_speed: 1.0 })
    .with(Glyph::palette_sprite(
      "textures/space_qud/gunman .png", 'g',
      Color::srgb(0.42, 0.52, 0.68), Color::srgb(0.72, 0.82, 0.92),
    ))
    .with(Loadout::from_gear(&[
      GearSlot::ability(Gear::InnateGun { damage: 4 }, 15),
      GearSlot::stacked(Gear::Loot(crate::level::Item::GoldCoin), 4),
    ]));

  pub const ROBOT_DOG: Self = Self::ENEMY_BASE
    .with(Named {
      name: "Guard Dog",
      flavor: "A battered patrol drone on four legs. Its mounted gun tracks movement.",
    })
    .with(Stats { hp: 10, max_hp: 10, attack: 2, move_speed: 3.0, attack_speed: 1.0 })
    .with(Glyph::palette_sprite(
      "textures/space_qud/robot dog with gun.png", 'd',
      Color::srgb(0.15, 0.15, 0.18), Color::srgb(0.85, 0.75, 0.15),
    ))
    .with(Loadout::from_gear(&[
      GearSlot::ability(Gear::InnateGun { damage: 3 }, 12),
      GearSlot::stacked(Gear::Loot(crate::level::Item::GoldCoin), 3),
    ]));

  pub const TURRET: Self = Self::ENEMY_BASE
    .with(Named {
      name: "Turret",
      flavor: "A ceiling-mounted autoturret. It can't move, but its tracking is relentless.",
    })
    .with(Stats { hp: 12, max_hp: 12, attack: 1, move_speed: 0.0, attack_speed: 1.0 })
    .with(Glyph::palette_sprite(
      "textures/space_qud/turret1.png", 't',
      Color::srgb(0.5, 0.5, 0.5), Color::srgb(0.8, 0.2, 0.2),
    ))
    .with(Loadout::from_gear(&[
      GearSlot::ability(Gear::InnateGun { damage: 5 }, 10),
    ]));

  // ---- zero-arg .with()-only structures → associated constants ----

  pub const PLAYER: Self = Self::EMPTY
    .with(Collidable(true))
    .with(Character)
    .with(FactionComp(Faction::Player))
    .with(Gravity)
    .with(Player);

  pub const SPACE_CAT: Self = Self::STRUCTURE_PASSABLE
    .with(Glyph::palette_sprite(
      "textures/space_qud/space cat.png", 'c',
      Color::srgb(0.92, 0.82, 0.62), Color::srgb(0.52, 0.36, 0.26),
    ))
    .with(Named { name: "Space cat", flavor: "Judges your piloting from a warm bulkhead. Offers no corrections." });

  pub const BOULDER: Self = Self::STRUCTURE
    .with(Glyph::palette_sprite(
      "textures/space_qud/rock.png", 'o',
      Color::srgb(0.32, 0.30, 0.28), Color::srgb(0.58, 0.55, 0.50),
    ))
    .with(Named { name: "Boulder", flavor: "A massive rock. Immovable." });

  pub const THRUSTER: Self = Self::STRUCTURE
    .with(Glyph::palette_sprite(
      "textures/space_qud/thruster.png", '>',
      Color::srgb(0.72, 0.38, 0.08), Color::srgb(0.75, 0.75, 0.72),
    ))
    .with(Named { name: "Thruster", flavor: "A directional thruster assembly. Keeps the ship moving." });

  pub const SPORE_CLOUD: Self = Self::EMPTY
    .with(Collidable(false))
    .with(Glyph::palette_sprite(
      "textures/space_qud/checkerboard pattern.png", '*',
      Color::srgb(0.30, 0.72, 0.22), Color::srgb(0.18, 0.48, 0.12),
    ))
    .with(Named { name: "Spore Cloud", flavor: "A drifting cloud of toxic fungal spores." })
    .with(DamageCloud { damage_per_tick: 1, ticks_remaining: 4, tick_interval: 5, tick_timer: 0 });

  pub const EXPLOSION_CLOUD: Self = Self::EMPTY
    .with(Collidable(false))
    .with(Glyph::palette_sprite(
      "textures/space_qud/checkerboard pattern.png", '*',
      Color::srgb(0.95, 0.55, 0.10), Color::srgb(0.72, 0.22, 0.06),
    ))
    .with(Named { name: "Explosion", flavor: "Roiling flame and shrapnel." })
    .with(DamageCloud { damage_per_tick: 3, ticks_remaining: 2, tick_interval: 2, tick_timer: 0 });

  pub const LASER_SWORD: Self = Self::STRUCTURE_PASSABLE
    .with(Glyph::palette_sprite(
      "textures/space_qud/laser sword.png", '/',
      Color::srgb(0.18, 0.08, 0.52), Color::srgb(0.42, 0.82, 0.98),
    ))
    .with(Named { name: "Laser Sword", flavor: "An energy blade, dormant. Still hums faintly." });

  pub const TABLE: Self = Self::STRUCTURE
    .with(Glyph::palette_sprite(
      "textures/space_qud/table.png", 't',
      Color::srgb(0.48, 0.34, 0.18), Color::srgb(0.72, 0.58, 0.36),
    ))
    .with(Named { name: "Table", flavor: "A sturdy table." });

  pub const CHAIR: Self = Self::STRUCTURE_PASSABLE
    .with(Glyph::palette_sprite(
      "textures/space_qud/chair (1).png", 'h',
      Color::srgb(0.60, 0.62, 0.65), Color::srgb(0.72, 0.18, 0.14),
    ))
    .with(Named { name: "Chair", flavor: "A chair. Something to sit on." });

  pub const LOCKER: Self = Self::STRUCTURE
    .with(Glyph::palette_sprite(
      "textures/space_qud/locker (2).png", 'l',
      Color::srgb(0.32, 0.38, 0.42), Color::srgb(0.62, 0.68, 0.72),
    ))
    .with(Named { name: "Locker", flavor: "A metal locker. Whatever was inside is long gone." });

  pub const CRATE_OBJ: Self = Self::STRUCTURE
    .with(Glyph::palette_sprite(
      "textures/space_qud/crate.png", 'c',
      Color::srgb(0.42, 0.32, 0.18), Color::srgb(0.72, 0.60, 0.38),
    ))
    .with(Named { name: "Crate", flavor: "A battered storage crate. Probably empty." });

  pub const DOOR: Self = Self::STRUCTURE
    .with(Glyph::palette_sprite(
      "textures/space_qud/door closed (1).png", '+',
      DOOR_CLOSED_PRI, DOOR_CLOSED_SEC,
    ))
    .with(Named { name: "Door", flavor: "Press Space to open." })
    .with(BlocksSight)
    .with(Door { open: false, closed_color: DOOR_CLOSED_PRI });

  pub const AIRLOCK_DOOR: Self = Self::DOOR
    .with(Glyph::palette_sprite(
      "textures/space_qud/airlock closed.png", '+',
      crate::AIRLOCK_PRI, crate::AIRLOCK_SEC,
    ))
    .with(AirlockDoor { opened_at_sim_time: None });

  pub const FLIGHT_CONSOLE: Self = Self::STRUCTURE
    .with(Glyph::palette_sprite(
      "textures/space_qud/computer .png", 'C',
      Color::srgb(0.18, 0.34, 0.52), Color::srgb(0.32, 0.88, 0.45),
    ))
    .with(Named { name: "Flight Console", flavor: "Navigation computer. Plot a course to a destination." })
    .with(FlightConsole);

  pub const LOADOUT_CONSOLE: Self = Self::STRUCTURE
    .with(Glyph::palette_sprite(
      "textures/space_qud/locker (1).png", 'Q',
      Color::srgb(0.25, 0.38, 0.52), Color::srgb(0.55, 0.75, 0.88),
    ))
    .with(Named { name: "Loadout Console", flavor: "Manage your equipped weapon and armor from your collected gear." })
    .with(LoadoutConsole);

  pub const LOOT_CHEST: Self = Self::STRUCTURE
    .with(Glyph::palette_sprite(
      "textures/space_qud/crate.png", '&',
      Color::srgb(0.72, 0.52, 0.28), Color::srgb(0.42, 0.32, 0.22),
    ))
    .with(Named { name: "Chest", flavor: "Someone stashed supplies here." })
    .with(LootChest { opened: false });

  pub const BED: Self = Self::STRUCTURE
    .with(Glyph::palette_sprite(
      "textures/space_qud/bed.png", 'b',
      Color::srgb(0.52, 0.38, 0.22), Color::srgb(0.88, 0.84, 0.72),
    ))
    .with(Named { name: "Bed", flavor: "A place to sleep. Looks like it hasn't been used in a while." })
    .with(Bed);

  pub const CRAFTING_TABLE: Self = Self::STRUCTURE
    .with(Glyph::palette_sprite(
      "textures/space_qud/crafting table.png", 'C',
      Color::srgb(0.38, 0.42, 0.48), Color::srgb(0.62, 0.62, 0.62),
    ))
    .with(Named { name: "Crafting Table", flavor: "A workbench for assembling equipment from salvaged parts." })
    .with(CraftingTable);

  // ---- const fn factories (take args, only .with()) ----

  pub const fn mushroom(primary: Color, secondary: Color, name: &'static str) -> Self {
    Self::STRUCTURE_PASSABLE
      .with(Glyph::palette_sprite("textures/space_qud/mushroom.png", 'm', primary, secondary))
      .with(Named { name, flavor: "A large fungal growth rooted in the alien soil." })
  }

  pub const fn torch(radius: u32) -> Self {
    Self::EMPTY.with(LightSource { radius })
  }

  pub const fn wall(material: Material) -> Self {
    Self::STRUCTURE.with(WallComp { material })
  }

  pub const fn defined_npc(
    named: Named,
    stats: Stats,
    loadout: Loadout,
    glyph: Glyph,
    dialogue: &'static DialogueTree
  ) -> Self {
    Self::NPC_BASE
      .with(named)
      .with(stats)
      .with(loadout)
      .with(glyph)
      .with(Dialogue(dialogue))
  }

  pub const fn as_follower(obj: Self) -> Self {
    obj
      .with(FollowerState::Available)
      .with(FollowerData { home: (0, 0, 0), move_timer: 0 })
      .with(Path { steps: VecDeque::new(), cached_goal: None })
  }

  pub const fn supply_cache(contents: &'static [(crate::level::Item, u32)]) -> Self {
    Self::EMPTY
      .with(Collidable(true))
      .with(Glyph::palette_sprite(
        "textures/space_qud/crate.png", 'S',
        Color::srgb(0.28, 0.42, 0.52), Color::srgb(0.52, 0.75, 0.88),
      ))
      .with(Named { name: "Supply Cache", flavor: "A sealed cache. Whoever left this behind had plans they didn't finish." })
      .with(LootChest { opened: false })
      .with(FixedChestLoot(contents))
  }

  // ---- fn factories (need rand, runtime Vec, etc.) ----

  pub const TREE: Self = Self::STRUCTURE_PASSABLE
    .with(Glyph::palette_sprite(
      "textures/space_qud/tree.png", 'T',
      Color::srgb(0.14, 0.42, 0.16), Color::srgb(0.38, 0.62, 0.24),
    ))
    .with(Named { name: "Tree", flavor: "A sturdy tree. Could be chopped for wood." })
    .with(BlocksSight)
    .with(Tree);

  pub const TREE2: Self = Self::STRUCTURE_PASSABLE
    .with(Glyph::palette_sprite(
      "textures/space_qud/tree2.png", 'T',
      Color::srgb(0.14, 0.42, 0.16), Color::srgb(0.38, 0.62, 0.24),
    ))
    .with(Named { name: "Tree", flavor: "A sturdy tree. Could be chopped for wood." })
    .with(BlocksSight)
    .with(Tree);

  pub fn random_tree() -> Self {
    if rand::random::<bool>() { Self::TREE } else { Self::TREE2 }
  }

  pub fn elevator(current_z: usize, floors: Vec<(usize, i32, i32)>) -> Self {
    Self::STRUCTURE
      .with(Glyph::palette_sprite(
        "textures/space_qud/elevator.png", 'E',
        Color::srgb(0.42, 0.46, 0.50), Color::srgb(1.0, 0.85, 0.10),
      ))
      .with(Named { name: "Elevator", flavor: "Vertical transport. Choose a deck." })
      .with(Elevator { current_z, floors: Cow::Owned(floors) })
  }

  pub fn cave_entrance(surface_x: i32, surface_y: i32, cave_x: i32, cave_y: i32) -> Self {
    Self::STRUCTURE_PASSABLE
      .with(Glyph::palette_sprite(
        "textures/space_qud/stairs.png", '>',
        Color::srgb(0.35, 0.32, 0.28), Color::srgb(0.55, 0.50, 0.40),
      ))
      .with(Named { name: "Cave Entrance", flavor: "A dark opening leads underground." })
      .with(Elevator {
        current_z: 0,
        floors: Cow::Owned(vec![(0, surface_x, surface_y), (1, cave_x, cave_y)])
      })
  }

  pub fn cave_exit(surface_x: i32, surface_y: i32, cave_x: i32, cave_y: i32) -> Self {
    Self::STRUCTURE_PASSABLE
      .with(Glyph::palette_sprite(
        "textures/space_qud/stairs up.png", '<',
        Color::srgb(0.55, 0.50, 0.40), Color::srgb(0.35, 0.32, 0.28),
      ))
      .with(Named { name: "Cave Exit", flavor: "Daylight filters in from above." })
      .with(Elevator {
        current_z: 1,
        floors: Cow::Owned(vec![(0, surface_x, surface_y), (1, cave_x, cave_y)])
      })
  }

  pub const fn ground_item(item: crate::level::Item) -> Self {
    let (primary, secondary) = item.loot_colors();
    Self::EMPTY
      .with(Glyph::palette_sprite(item.loot_texture(), '*', primary, secondary))
      .with(GroundItem(item))
  }
}

pub fn npc_person_glyph(ch: char, primary: Color, secondary: Color) -> Glyph {
  Glyph::palette_sprite("textures/space_qud/person (2).png", ch, primary, secondary)
}

pub fn npc_robo_glyph(ch: char, primary: Color, secondary: Color) -> Glyph {
  Glyph::palette_sprite("textures/space_qud/robo (1).png", ch, primary, secondary)
}
