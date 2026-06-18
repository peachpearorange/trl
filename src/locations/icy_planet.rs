//! Icy planet "Brume": naturalistic glacial terrain from layered noise, plus
//! a small settled village with a hand-dug cellar level beneath it.
//!
//! Terrain model (same philosophy as `natural_planet`):
//!
//! * Elevation decides the band: open polar water → frozen lake ice →
//!   snowfield → glacier wall.
//! * Snowdrifts intrude onto lake ice where detail noise runs high, so
//!   frozen lakes have ragged, wind-blown edges instead of clean contours.
//! * Gravel shorelines ring every lake; permafrost scars and boulder fields
//!   break up the snow; meandering glacier/rock ridges limit movement.
//! * Trees only grow in a mid-elevation taiga belt — valleys are bare ice,
//!   peaks are bare glacier, and the forest sits in between like a treeline.
//!
//! The village picks the buildable patch of snow nearest the dock: six timber
//! houses with residents and furniture, trampled-ground paths converging on a
//! lamplit plaza, a fenced livestock pen, and a cellar level dug beneath the
//! oldest house (reachable by its indoor stairs or an outdoor hatch). The
//! shared cave system (`cave_gen`) runs underneath everything, with its own
//! surface entrances out in the wilds.

use {super::{cave_gen,
             natural_planet::{NoiseField, place_ship_dock, smoothstep}},
     crate::{entities::*,
             galaxy::{Location, LocationId},
             level::{Item, Level, LocationType, Tile},
             prefabs::{Prefab, prefab}},
     bevy::prelude::Color,
     rand::{Rng, RngCore, SeedableRng, rngs::SmallRng},
     std::{borrow::Cow, collections::VecDeque}};

pub const ID: LocationId = (14, 0, 0);
pub const NAME: &str = "Brume";
pub const SEED: u64 = 0x1CE0_FA11;
const SIZE: usize = 300;

// Elevation band thresholds (0..1 normalized noise).
const ELEV_SEA: f32 = 0.30; // open water at lake cores
const ELEV_ICE: f32 = 0.43; // frozen lake ice
const ELEV_GLACIER: f32 = 0.78; // glacier walls at the peaks

// The taiga belt: trees only grow at mid elevation, between the frozen
// valleys and the bare glacier heights.
const TAIGA_LO: f32 = 0.48;
const TAIGA_HI: f32 = 0.72;
const TREE_BAND_LO: f32 = 0.40;
const TREE_BAND_HI: f32 = 0.70;
const TREE_MAX_PROB: f32 = 0.55;

const ROCK_MASK_THRESHOLD: f32 = 0.83;
const ROCK_DETAIL_THRESHOLD: f32 = 0.55;
const SNOWDRIFT_THRESHOLD: f32 = 0.66;

const RIDGE_COUNT: usize = 4;
const RIDGE_THICKNESS: i32 = 2;
const RIDGE_GAP_CHANCE: f64 = 0.10;

const VILLAGE_W: i32 = 46;
const VILLAGE_H: i32 = 34;

const CELLAR_CACHE: &[(Item, u32)] = &[
  (Item::HealthPotion, 2),
  (Item::Torch, 2),
  (Item::Wood, 4),
  (Item::GoldCoin, 12)
];

fn classify(e: f32) -> Tile {
  if e < ELEV_SEA {
    Tile::DeepWater
  } else if e < ELEV_ICE {
    Tile::IceFloor
  } else if e < ELEV_GLACIER {
    Tile::BrightGround
  } else {
    Tile::IceWall
  }
}

fn snowy(t: Tile) -> bool {
  matches!(t, Tile::BrightGround | Tile::Ground | Tile::SmallRocks)
}

// ---------------------------------------------------------------------------
// Village pieces
// ---------------------------------------------------------------------------

const BRUME_QUEST_ID: &str = crate::quest::BRUME_PREDATOR.id;
static BRUME_QUEST_ACCEPT: [QuestAction; 1] = [QuestAction::Start(BRUME_QUEST_ID)];
static BRUME_QUEST_TURN_IN: [QuestAction; 1] = [QuestAction::SetStage(BRUME_QUEST_ID, 100)];

static VILLAGER_DIALOGUE: DialogueTree = dialogue_tree(&[
  node(
    "root",
    "Off-worlder. We get one ship a season here, and you weren't on it. \
     Whatever you're after — trade, shelter, plain warmth — say it quick. \
     The cold doesn't wait and neither do I.",
    &[
      go("Just passing through.", "passing"),
      go("Tell me about Brume.", "about"),
      DialogueChoice {
        text: "Anything I can help with?",
        next: Some("predator_offer"),
        on_select: &[],
        condition: DialogueCondition::QuestInactive(BRUME_QUEST_ID),
      },
      DialogueChoice {
        text: "Still tracking that thing.",
        next: Some("predator_progress"),
        on_select: &[],
        condition: DialogueCondition::QuestActive(BRUME_QUEST_ID),
      },
      DialogueChoice {
        text: "The matriarch is dead.",
        next: Some("predator_turn_in"),
        on_select: &[],
        condition: DialogueCondition::QuestStageAtLeast(BRUME_QUEST_ID, 20),
      },
      end("Never mind. Stay warm.")
    ]
  ),
  node(
    "passing",
    "Then pass through. The plaza's heated, the houses aren't yours. \
     Don't bother the sheep.",
    &[end("Understood.")]
  ),
  node(
    "about",
    "Six houses, one cellar, more cold than people. \
     We hunt, we mend, we wait for the supply runner to be on time once \
     in our lives. He never is. \
     ...If you've come from Iron Ring, keep that to yourself. \
     We don't like what those tin things are growing up there.",
    &[end("I'll keep it to myself.")]
  ),
  node(
    "predator_offer",
    "Truth is, yes. Something's been taking sheep. Not a wolf — wolves we know. \
     This thing leaves prints with six toes and drags the carcass clean off. \
     The old hunters call it a FROSTMAW. \
     Its den's out past the ridges, opposite side of the snowfield from us. \
     If you can find the matriarch and put her down, we'd consider the debt \
     paid in full. And then some.",
    &[
      DialogueChoice {
        text: "I'll deal with it.",
        next: Some("predator_accepted"),
        on_select: &BRUME_QUEST_ACCEPT,
        condition: DialogueCondition::Always,
      },
      end("Not my line of work. Sorry.")
    ]
  ),
  node(
    "predator_accepted",
    "Then go before the next lamb does. Look for snow that doesn't lie \
     right — drifts that have been packed down by something heavy moving \
     in and out. The matriarch's the big one. You'll know.",
    &[end("I'll know.")]
  ),
  node(
    "predator_progress",
    "Still out there. We hear it some nights. \
     Be careful out past the ridges — the pups hunt in a pack, but the \
     matriarch hunts alone, and she's the one that matters.",
    &[end("Understood.")]
  ),
  node(
    "predator_turn_in",
    "...You're sure? \
     *long breath* \
     Then we owe you. Properly. Take this — it's not much, but it's the \
     warm half of what we have. \
     Tonight the lamps stay lit for you, off-worlder.",
    &[
      DialogueChoice {
        text: "Glad it's done.",
        next: None,
        on_select: &BRUME_QUEST_TURN_IN,
        condition: DialogueCondition::Always,
      }
    ]
  )
]);

static PELL_DIALOGUE: DialogueTree = dialogue_tree(&[
  node(
    "root",
    "Hey — you're from off-world, right? You must have seen things. \
     Real things. Not sheep and snow and the same six faces every day.",
    &[
      go("You seem restless.", "restless"),
      go("Know anything useful around here?", "useful"),
      end("Just passing through.")
    ]
  ),
  node(
    "restless",
    "Restless? I'm going to lose my mind. There's something under the \
     ice — I can feel it. The old caves go deeper than anyone admits. \
     But nobody here wants to talk about that.",
    &[end("Good luck with that.")]
  ),
  node(
    "useful",
    "There's a wizard — well, Brennick calls him a wizard. Lives in a \
     stone tower out past the taiga belt. Name's Veradis. Keeps to himself \
     mostly, but he knows things. If you're looking for work that's not \
     sheep-related, he's your best bet.",
    &[end("I'll check it out.")]
  ),
]);

// ---------------------------------------------------------------------------
// Frostmaws — the sheep-killing predators that hide out in the snow den.
// ---------------------------------------------------------------------------

pub const FROSTMAW_MATRIARCH_NAME: &str = "Frostmaw Matriarch";

static FROSTMAW_GEAR: [GearSlot; 1] = [
  GearSlot::stacked(Gear::Loot(Item::OrganicMaterial), 1),
];
static MATRIARCH_GEAR: [GearSlot; 2] = [
  GearSlot::stacked(Gear::Loot(Item::OrganicMaterial), 3),
  GearSlot::stacked(Gear::Loot(Item::GoldCoin), 10),
];

fn frostmaw() -> Object {
  Object::ENEMY_BASE
    .with(CreatureKind::Alien)
    .with(Named::s("Frostmaw", "A long, gaunt thing built low to the snow — six pale legs, two rows of \
               needle teeth, eyes the color of frozen sky. It smells of old blood and \
               wet wool."))
    .with(Stats { hp: 8, max_hp: 8, attack: 3, move_speed: 3.5, attack_speed: 1.1 })
    .with(Glyph::from_char('f', Color::srgb(0.88, 0.94, 1.0)))
    .with(Loadout::from_gear(&FROSTMAW_GEAR))
}

fn frostmaw_matriarch() -> Object {
  Object::ENEMY_BASE
    .with(CreatureKind::Alien)
    .with(Named::s(
      FROSTMAW_MATRIARCH_NAME,
      "The big one. Twice the bulk of the others, hide thick with old scars and \
               wool fibers caught in the seams. The Brume hunters used to draw it on \
               cave walls before they stopped going out at dusk."
    ))
    .with(Stats { hp: 28, max_hp: 28, attack: 6, move_speed: 2.6, attack_speed: 0.9 })
    .with(Glyph::from_char('F', Color::srgb(0.74, 0.86, 1.0)))
    .with(Loadout::from_gear(&MATRIARCH_GEAR))
}

static VILLAGER_GEAR: [GearSlot; 2] = [
  GearSlot::passive(Gear::Weapon(Item::CopperKnife)),
  GearSlot::stacked(Gear::Loot(Item::GoldCoin), 2),
];

fn villager(name: &'static str, flavor: &'static str, coat: Color) -> Object {
  Object::NPC_BASE
    .with(Named { name: Cow::Borrowed(name), flavor: Cow::Borrowed(flavor) })
    .with(Stats { hp: 14, max_hp: 14, attack: 2, move_speed: 3.0, attack_speed: 1.0 })
    .with(npc_person_glyph('@', coat, Color::srgb(0.85, 0.88, 0.92)))
    .with(Collidable(true))
    .with(CreatureKind::Human)
    .with(Dialogue(&VILLAGER_DIALOGUE))
    .with(Loadout::from_gear(&VILLAGER_GEAR))
}

fn lamp_post() -> Object {
  Object::STRUCTURE
    .with(Glyph::from_char('!', Color::srgb(1.0, 0.85, 0.45)))
    .with(Named::s("Lamp Post", "An oil lamp on a frost-cracked pole. Someone refills it every night."))
    .with(LightSource { radius: 7 })
}

/// Indoor stair head / outdoor hatch down to the cellar level. Both ends sit
/// at the same (x, y), so descending feels like going straight down.
fn cellar_down(cellar_z: usize, x: i32, y: i32, name: &'static str, flavor: &'static str) -> Object {
  Object::STRUCTURE_PASSABLE
    .with(Glyph::recolor_sprite(
      "textures/space_qud/stairs.png",
      '>',
      Color::srgb(0.42, 0.34, 0.22),
      Color::srgb(0.62, 0.52, 0.34)
    ))
    .with(Named { name: Cow::Borrowed(name), flavor: Cow::Borrowed(flavor) })
    .with(Elevator { current_z: 0, floors: Cow::Owned(vec![(0, x, y), (cellar_z, x, y)]) })
    .with(ShowOnCompass)
}

/// Counterpart of [`cellar_down`], placed on the cellar level.
fn cellar_up(cellar_z: usize, x: i32, y: i32) -> Object {
  Object::STRUCTURE_PASSABLE
    .with(Glyph::recolor_sprite(
      "textures/space_qud/stairs up.png",
      '<',
      Color::srgb(0.62, 0.52, 0.34),
      Color::srgb(0.42, 0.34, 0.22)
    ))
    .with(Named::s("Stairs Up", "Lamplight and woodsmoke drift down from above."))
    .with(Elevator {
      current_z: cellar_z,
      floors: Cow::Owned(vec![(0, x, y), (cellar_z, x, y)])
    })
    .with(ShowOnCompass)
}

// House layouts. `s` marks the cellar stair tile; doors face the plaza, so
// the top row of houses opens south and the bottom row opens north.

fn house_with_cellar(resident: Object) -> Prefab {
  prefab(
    "
    wwwwwww
    wsffBfw
    wffffLw
    wfTcffw
    wfffnfw
    wwwdwww
    "
  )
  .assoc('w', (Tile::WoodWall, []))
  .assoc('f', (Tile::WoodFloor, []))
  .assoc('s', (Tile::WoodFloor, []))
  .assoc('B', (Tile::WoodFloor, [Object::BED]))
  .assoc('L', (Tile::WoodFloor, [Object::LOCKER]))
  .assoc('T', (Tile::WoodFloor, [Object::TABLE]))
  .assoc('c', (Tile::WoodFloor, [Object::CHAIR]))
  .assoc('n', (Tile::WoodFloor, [resident]))
  .assoc('d', (Tile::WoodFloor, [Object::DOOR]))
}

fn house_small_south(resident: Object) -> Prefab {
  prefab(
    "
    wwwww
    wBfLw
    wfnTw
    wwdww
    "
  )
  .assoc('w', (Tile::WoodWall, []))
  .assoc('f', (Tile::WoodFloor, []))
  .assoc('B', (Tile::WoodFloor, [Object::BED]))
  .assoc('L', (Tile::WoodFloor, [Object::LOCKER]))
  .assoc('T', (Tile::WoodFloor, [Object::TABLE]))
  .assoc('n', (Tile::WoodFloor, [resident]))
  .assoc('d', (Tile::WoodFloor, [Object::DOOR]))
}

fn house_long_south(resident: Object) -> Prefab {
  prefab(
    "
    wwwwwwww
    wBfffkBw
    wfffffLw
    wnfTTcfw
    wwwwdwww
    "
  )
  .assoc('w', (Tile::WoodWall, []))
  .assoc('f', (Tile::WoodFloor, []))
  .assoc('B', (Tile::WoodFloor, [Object::BED]))
  .assoc('k', (Tile::WoodFloor, [Object::CRATE_OBJ]))
  .assoc('L', (Tile::WoodFloor, [Object::LOCKER]))
  .assoc('T', (Tile::WoodFloor, [Object::TABLE]))
  .assoc('c', (Tile::WoodFloor, [Object::CHAIR]))
  .assoc('n', (Tile::WoodFloor, [resident]))
  .assoc('d', (Tile::WoodFloor, [Object::DOOR]))
}

fn house_hearth_north(resident: Object) -> Prefab {
  prefab(
    "
    wwdwww
    wffffw
    wWfTcw
    wfnfBw
    wwwwww
    "
  )
  .assoc('w', (Tile::WoodWall, []))
  .assoc('f', (Tile::WoodFloor, []))
  .assoc('W', (Tile::WoodFloor, [Object::CRAFTING_TABLE]))
  .assoc('T', (Tile::WoodFloor, [Object::TABLE]))
  .assoc('c', (Tile::WoodFloor, [Object::CHAIR]))
  .assoc('B', (Tile::WoodFloor, [Object::BED]))
  .assoc('n', (Tile::WoodFloor, [resident]))
  .assoc('d', (Tile::WoodFloor, [Object::DOOR]))
}

fn house_small_north(resident: Object) -> Prefab {
  prefab(
    "
    wwdww
    wTnfw
    wLfBw
    wwwww
    "
  )
  .assoc('w', (Tile::WoodWall, []))
  .assoc('f', (Tile::WoodFloor, []))
  .assoc('T', (Tile::WoodFloor, [Object::TABLE]))
  .assoc('L', (Tile::WoodFloor, [Object::LOCKER]))
  .assoc('B', (Tile::WoodFloor, [Object::BED]))
  .assoc('n', (Tile::WoodFloor, [resident]))
  .assoc('d', (Tile::WoodFloor, [Object::DOOR]))
}

fn house_long_north(resident: Object) -> Prefab {
  prefab(
    "
    wwwwdwww
    wffffffw
    wBfTTcnw
    wBkffffw
    wwwwwwww
    "
  )
  .assoc('w', (Tile::WoodWall, []))
  .assoc('f', (Tile::WoodFloor, []))
  .assoc('B', (Tile::WoodFloor, [Object::BED]))
  .assoc('k', (Tile::WoodFloor, [Object::CRATE_OBJ]))
  .assoc('T', (Tile::WoodFloor, [Object::TABLE]))
  .assoc('c', (Tile::WoodFloor, [Object::CHAIR]))
  .assoc('n', (Tile::WoodFloor, [resident]))
  .assoc('d', (Tile::WoodFloor, [Object::DOOR]))
}

/// L-shaped trampled-ground path (vertical leg first) that only overwrites
/// open snow, so it weaves between houses instead of cutting through them.
fn carve_path(level: &mut Level, from: (i32, i32), to: (i32, i32)) {
  let mut tread = |x: i32, y: i32| {
    if level.get(x, y).is_some_and(snowy) {
      level.set(x, y, Tile::Ground);
    }
  };
  let (mut x, mut y) = from;
  while y != to.1 {
    tread(x, y);
    y += (to.1 - y).signum();
  }
  while x != to.0 {
    tread(x, y);
    x += (to.0 - x).signum();
  }
  tread(x, y);
}

fn carve_room(level: &mut Level, x0: i32, y0: i32, x1: i32, y1: i32, floor: Tile) {
  for y in y0..=y1 {
    for x in x0..=x1 {
      level.set(x, y, floor);
    }
  }
}

/// 2-wide L-shaped cellar corridor (horizontal leg first, to mirror the
/// surface paths so the two layouts don't shadow each other exactly).
fn carve_corridor(level: &mut Level, from: (i32, i32), to: (i32, i32)) {
  let mut dig = |x: i32, y: i32| {
    level.set(x, y, Tile::CaveFloor);
    level.set(x + 1, y, Tile::CaveFloor);
    level.set(x, y + 1, Tile::CaveFloor);
  };
  let (mut x, mut y) = from;
  while x != to.0 {
    dig(x, y);
    x += (to.0 - x).signum();
  }
  while y != to.1 {
    dig(x, y);
    y += (to.1 - y).signum();
  }
  dig(x, y);
}

/// Buildable patch of open ground nearest the dock: every tile in the
/// footprint (plus a 2-tile apron) must be open snow, and no cave entrance
/// may sit inside it (entrances pair with exits below — they can't move).
fn find_village_site(loc: &Location, dock: (i32, i32)) -> (i32, i32) {
  let level = loc.level(0);
  let entrances: Vec<(i32, i32)> = loc
    .spawn_objects
    .iter()
    .filter(|&&(_, _, z, ref o)| z == 0 && Has::<Elevator>::get(o).is_some())
    .map(|&(x, y, _, _)| (x, y))
    .collect();
  let site_ok = |ox: i32, oy: i32, tile_ok: &dyn Fn(Tile) -> bool| {
    (oy - 2..oy + VILLAGE_H + 2).all(|y| {
      (ox - 2..ox + VILLAGE_W + 2).all(|x| level.get(x, y).is_some_and(tile_ok))
    }) && entrances.iter().all(|&(ex, ey)| {
      ex < ox - 2 || ex >= ox + VILLAGE_W + 2 || ey < oy - 2 || ey >= oy + VILLAGE_H + 2
    })
  };
  let best = |tile_ok: &dyn Fn(Tile) -> bool| {
    (4..SIZE as i32 - VILLAGE_H - 4)
      .step_by(2)
      .flat_map(|oy| {
        (4..SIZE as i32 - VILLAGE_W - 4).step_by(2).map(move |ox| (ox, oy))
      })
      .filter(|&(ox, oy)| site_ok(ox, oy, tile_ok))
      .min_by_key(|&(ox, oy)| {
        (ox + VILLAGE_W / 2 - dock.0).abs() + (oy + VILLAGE_H / 2 - dock.1).abs()
      })
  };
  best(&snowy)
    .or_else(|| best(&|t| t.walkable() && !t.is_liquid() && t != Tile::ShipDock))
    .unwrap_or((
      (dock.0 + 8).clamp(4, SIZE as i32 - VILLAGE_W - 4),
      (dock.1 + 8).clamp(4, SIZE as i32 - VILLAGE_H - 4)
    ))
}

/// Stamp the village and dig its cellar. Returns the footprint origin so the
/// creature scatter can keep hostiles away from it.
fn build_village(loc: &mut Location, dock: (i32, i32)) -> (i32, i32) {
  let (ox, oy) = find_village_site(loc, dock);

  // Claim the footprint: drop scattered trees/creatures inside it (cave
  // entrances were avoided by site selection) and normalize any leftover
  // unbuildable tile to snow.
  loc.spawn_objects.retain(|&(x, y, z, ref o)| {
    z != 0
      || x < ox || x >= ox + VILLAGE_W || y < oy || y >= oy + VILLAGE_H
      || Has::<Elevator>::get(o).is_some()
  });
  {
    let level = loc.level_mut(0);
    for y in oy..oy + VILLAGE_H {
      for x in ox..ox + VILLAGE_W {
        let t = level.get(x, y).unwrap_or(Tile::Wall);
        if !snowy(t) && t != Tile::ShipDock {
          level.set(x, y, Tile::BrightGround);
        }
      }
    }
  }

  let houses: Vec<((i32, i32), Prefab)> = vec![
    (
      (ox + 3, oy + 3),
      house_with_cellar(villager(
        "Old Brennick",
        "The first one to dig here. Won't say what made him stop.",
        Color::srgb(0.62, 0.48, 0.32)
      ))
    ),
    (
      (ox + 15, oy + 4),
      house_small_south(villager(
        "Suvi",
        "Mends nets nobody fishes with anymore. The lakes froze through years ago.",
        Color::srgb(0.40, 0.55, 0.70)
      ))
    ),
    (
      (ox + 25, oy + 3),
      house_long_south(villager(
        "Harrow",
        "Counts the lamps every dusk. Says the count has to come out even.",
        Color::srgb(0.55, 0.42, 0.58)
      ))
    ),
    (
      (ox + 6, oy + 24),
      house_hearth_north(villager(
        "Wren the Smith",
        "Keeps the workbench warm. Trades repairs for stories from offworld.",
        Color::srgb(0.70, 0.38, 0.28)
      ))
    ),
    (
      (ox + 18, oy + 25),
      house_small_north(
        Object::NPC_BASE
          .with(Named::s("Pell", "Young, restless, and sure there's something under the ice worth finding."))
          .with(Stats { hp: 14, max_hp: 14, attack: 2, move_speed: 3.0, attack_speed: 1.0 })
          .with(npc_person_glyph('@', Color::srgb(0.38, 0.62, 0.45), Color::srgb(0.85, 0.88, 0.92)))
          .with(Collidable(true))
          .with(CreatureKind::Human)
          .with(Dialogue(&PELL_DIALOGUE))
          .with(Loadout::from_gear(&VILLAGER_GEAR))
      )
    ),
    (
      (ox + 28, oy + 24),
      house_long_north(villager(
        "Mother Ilsa",
        "Feeds anyone who knocks. Asks that you don't whistle after dark.",
        Color::srgb(0.78, 0.68, 0.40)
      ))
    ),
  ];

  let plaza = (ox + 21, oy + 16);
  for (origin, house) in &houses {
    house.stamp_level(loc.level_mut(0), origin.0, origin.1);
    house.for_each_assoc_object(|lx, ly, obj| {
      loc.spawn_objects.push((origin.0 + lx, origin.1 + ly, 0, obj.clone()));
    });
    if let Some((dx, dy)) = house.find_char('d') {
      let door = (origin.0 + dx, origin.1 + dy);
      // Step one tile out of the doorway (toward the plaza) before pathing.
      let front = (door.0, door.1 + (plaza.1 - door.1).signum());
      carve_path(loc.level_mut(0), front, plaza);
    }
  }

  // The plaza itself: a trampled clearing with lamps at its corners.
  {
    let level = loc.level_mut(0);
    for y in plaza.1 - 2..=plaza.1 + 2 {
      for x in plaza.0 - 3..=plaza.0 + 3 {
        if level.get(x, y).is_some_and(snowy) {
          level.set(x, y, Tile::Ground);
        }
      }
    }
  }
  for (lx, ly) in [(-3, -2), (3, -2), (-3, 2), (3, 2)] {
    loc.spawn_objects.push((plaza.0 + lx, plaza.1 + ly, 0, lamp_post()));
  }

  // Livestock pen: fence ring east of the plaza, trampled inside, sheep within.
  {
    let (px0, py0, px1, py1) = (ox + 37, oy + 12, ox + 43, oy + 17);
    let level = loc.level_mut(0);
    for y in py0..=py1 {
      for x in px0..=px1 {
        let edge = x == px0 || x == px1 || y == py0 || y == py1;
        level.set(x, y, if edge { Tile::Fence } else { Tile::Ground });
      }
    }
    for (sx, sy) in [(px0 + 1, py0 + 2), (px0 + 3, py0 + 3), (px0 + 4, py0 + 1)] {
      loc.spawn_objects.push((sx, sy, 0, Object::POLYCHROMATIC_SHEEP));
    }
  }

  // --- The cellar: a level of its own beneath the village. The cave system
  // already claimed levels 1.., so the cellar gets a fresh level appended at
  // the end — depth index doesn't have to match physical depth.
  let cellar_z = loc.levels.len();
  loc.levels.push(Level::new(loc.width, loc.height, Tile::CaveWall));
  loc.depth = loc.levels.len();

  // Stair head inside Old Brennick's house; hatch in the open ground between
  // the house rows. Both drop straight down to the same (x, y).
  let stairs = {
    let (hx, hy) = houses[0].0;
    let (sx, sy) = houses[0].1.find_char('s').unwrap();
    (hx + sx, hy + sy)
  };
  let hatch = (ox + 13, oy + 16);
  loc.spawn_objects.push((
    stairs.0,
    stairs.1,
    0,
    cellar_down(cellar_z, stairs.0, stairs.1, "Cellar Stairs", "Worn wooden steps lead down below the house.")
  ));
  loc.spawn_objects.push((stairs.0, stairs.1, cellar_z, cellar_up(cellar_z, stairs.0, stairs.1)));
  loc.spawn_objects.push((
    hatch.0,
    hatch.1,
    0,
    cellar_down(cellar_z, hatch.0, hatch.1, "Cellar Hatch", "A heavy timber hatch set into the frozen ground.")
  ));
  loc.spawn_objects.push((hatch.0, hatch.1, cellar_z, cellar_up(cellar_z, hatch.0, hatch.1)));

  // Rooms: a timbered landing under the house, a corridor to the storage
  // room under the hatch, and a rougher half-dug chamber beyond it.
  {
    let level = loc.level_mut(cellar_z);
    carve_room(level, stairs.0 - 1, stairs.1 - 1, stairs.0 + 5, stairs.1 + 4, Tile::WoodTile);
    carve_corridor(level, (stairs.0 + 3, stairs.1 + 4), (hatch.0, hatch.1));
    carve_room(level, hatch.0 - 3, hatch.1 - 2, hatch.0 + 3, hatch.1 + 3, Tile::WoodTile);
    carve_corridor(level, (hatch.0 + 3, hatch.1 + 1), (hatch.0 + 8, hatch.1 + 3));
    carve_room(level, hatch.0 + 8, hatch.1, hatch.0 + 14, hatch.1 + 5, Tile::CaveFloor);
  }

  // Landing room: a lamp and spare crates.
  loc.spawn_objects.push((stairs.0 + 4, stairs.1, cellar_z, lamp_post()));
  loc.spawn_objects.push((stairs.0 + 4, stairs.1 + 3, cellar_z, Object::CRATE_OBJ));

  // Storage room under the hatch: the village stores, watched over.
  loc.spawn_objects.push((hatch.0 - 2, hatch.1 - 1, cellar_z, Object::LOCKER));
  loc.spawn_objects.push((hatch.0 - 2, hatch.1 + 2, cellar_z, Object::LOCKER));
  loc.spawn_objects.push((hatch.0 + 2, hatch.1 - 1, cellar_z, Object::CRATE_OBJ));
  loc.spawn_objects.push((hatch.0 + 2, hatch.1 + 2, cellar_z, Object::supply_cache(CELLAR_CACHE)));
  loc.spawn_objects.push((hatch.0, hatch.1 + 2, cellar_z, lamp_post()));
  loc.spawn_objects.push((
    hatch.0 - 1,
    hatch.1 + 1,
    cellar_z,
    villager(
      "Cellar-Keeper Odd",
      "Tallies the stores by lamplight. Sleeps down here. Prefers it.",
      Color::srgb(0.45, 0.45, 0.55)
    )
  ));

  // The half-dug room: where the digging stopped. No lamp.
  loc.spawn_objects.push((hatch.0 + 12, hatch.1 + 2, cellar_z, Object::LOOT_CHEST));
  loc.spawn_objects.push((
    hatch.0 + 10,
    hatch.1 + 4,
    cellar_z,
    Object::mushroom(Color::srgb(0.16, 0.55, 0.48), Color::srgb(0.45, 0.95, 0.78), "Glowcap")
  ));
  loc.spawn_objects.push((hatch.0 + 13, hatch.1 + 1, cellar_z, Object::ground_item(Item::Crystal)));

  (ox, oy)
}

/// Stamp a small Frostmaw den somewhere in the wilds, opposite the village so
/// the player has to actually travel out there to clear it. Returns where the
/// matriarch is dropped (for quest hooks).
fn build_lair(loc: &mut Location, village: (i32, i32), dock: (i32, i32), seed: u64) -> (i32, i32) {
  let mut rng = SmallRng::seed_from_u64(seed ^ 0x1A12_5EED);
  let village_center = (village.0 + VILLAGE_W / 2, village.1 + VILLAGE_H / 2);
  let mirror = (SIZE as i32 - village_center.0, SIZE as i32 - village_center.1);

  let far_enough = |x: i32, y: i32| {
    (x - village_center.0).abs() + (y - village_center.1).abs() > 110
      && (x - dock.0).abs() + (y - dock.1).abs() > 40
  };

  // Spiral outward from the mirror point looking for a clear snow tile.
  let center = (0..3000)
    .find_map(|i| {
      let r = (i as f32).sqrt() * 1.8;
      let ang = i as f32 * 2.4;
      let x = mirror.0 + (r * ang.cos()) as i32;
      let y = mirror.1 + (r * ang.sin()) as i32;
      let in_bounds = x >= 8 && y >= 8 && x < SIZE as i32 - 8 && y < SIZE as i32 - 8;
      (in_bounds && far_enough(x, y) && loc.level(0).get(x, y).is_some_and(snowy))
        .then_some((x, y))
    })
    .unwrap_or(mirror);

  // Carve a rough oval chamber, ring it with ice walls, leave one gap as a mouth.
  {
    let level = loc.level_mut(0);
    let (rx, ry) = (5i32, 4i32);
    for dy in -ry - 1..=ry + 1 {
      for dx in -rx - 1..=rx + 1 {
        let nx = (dx as f32) / (rx as f32 + 0.5);
        let ny = (dy as f32) / (ry as f32 + 0.5);
        let d = nx * nx + ny * ny;
        let (x, y) = (center.0 + dx, center.1 + dy);
        if d <= 1.0 {
          level.set(x, y, Tile::CaveFloor);
        } else if d <= 1.35 {
          // Don't seal the south side — that's the mouth of the den.
          if !(dy >= ry - 1 && dx.abs() <= 2) {
            level.set(x, y, Tile::IceWall);
          }
        }
      }
    }
    // Trampled approach path of snowy ground heading south from the mouth.
    for step in 0..6 {
      let (x, y) = (center.0, center.1 + ry + step);
      level.set(x, y, Tile::Ground);
      level.set(x - 1, y, Tile::Ground);
    }
  }

  // Pups scattered around the chamber, matriarch dead center.
  let pup_spots = [(-3, -1), (3, -1), (-2, 2), (2, 2)];
  for &(dx, dy) in &pup_spots {
    loc.spawn_objects.push((center.0 + dx, center.1 + dy, 0, frostmaw()));
  }
  loc.spawn_objects.push((center.0, center.1, 0, frostmaw_matriarch()));

  // Bone cairn at the den mouth — also the compass marker for the lair, since
  // the matriarch's char glyph has no texture for the compass to project.
  loc.spawn_objects.push((
    center.0,
    center.1 + 1,
    0,
    Object::STRUCTURE_PASSABLE
      .with(Glyph::sprite("textures/space_qud/bones.png", '%', Color::srgb(0.92, 0.90, 0.82)))
      .with(Named::s("Bone Cairn", "A heap of cracked sheep bones piled at the threshold of the den. \
                 Some are still wet."))
      .with(ShowOnCompass)
  ));

  // A bone-littered loot chest at the back wall.
  loc.spawn_objects.push((
    center.0,
    center.1 - 3,
    0,
    Object::LOOT_CHEST
  ));

  // A few ground items hinting at the den's diet — sheep wool, scattered.
  for &(dx, dy) in &[(-2, 0), (1, -2), (-1, 3)] {
    let _ = rng.next_u32();
    loc.spawn_objects.push((
      center.0 + dx,
      center.1 + dy,
      0,
      Object::ground_item(Item::OrganicMaterial)
    ));
  }

  center
}

// ---------------------------------------------------------------------------
// Wizard — hermit arcanist living in a tower out in the wilds
// ---------------------------------------------------------------------------

const WIZARD_QUEST_ID: &str = crate::quest::BRUME_WIZARD.id;
static WIZARD_QUEST_ACCEPT: [QuestAction; 1] = [QuestAction::Start(WIZARD_QUEST_ID)];
static WIZARD_REWARD: [(Item, u32); 3] = [
  (Item::FrostScroll, 2),
  (Item::LightningScroll, 1),
  (Item::VoidScroll, 1),
];
static WIZARD_TURN_IN: [QuestAction; 3] = [
  QuestAction::TakeItem(Item::ResonanceLens),
  QuestAction::SetStage(WIZARD_QUEST_ID, 100),
  QuestAction::GiveItems(&WIZARD_REWARD),
];

static WIZARD_DIALOGUE: DialogueTree = dialogue_tree(&[
  node(
    "root",
    "You stink of ship fuel and spent shell casings. Not many find the tower. \
     Most who do were looking for someone else. \
     ...You may call me Veradis. What is it you want?",
    &[
      go("What is this place?", "tower"),
      go("The villagers mentioned you.", "villagers"),
      DialogueChoice {
        text: "I have something of yours.",
        next: Some("turn_in"),
        on_select: &[],
        condition: DialogueCondition::QuestStageAtLeast(WIZARD_QUEST_ID, 20),
      },
      DialogueChoice {
        text: "Still looking for your lens.",
        next: Some("progress"),
        on_select: &[],
        condition: DialogueCondition::QuestActive(WIZARD_QUEST_ID),
      },
      DialogueChoice {
        text: "Thanks for the scrolls.",
        next: Some("after"),
        on_select: &[],
        condition: DialogueCondition::QuestCompleted(WIZARD_QUEST_ID),
      },
      end("Wrong tower. Sorry.")
    ]
  ),
  node(
    "tower",
    "My home. I study resonance — the frequency at which intent \
     becomes substance. You would call it magic. I would call it \
     patience with a very particular kind of noise.",
    &[
      DialogueChoice {
        text: "Can you teach me?",
        next: Some("teach"),
        on_select: &[],
        condition: DialogueCondition::QuestInactive(WIZARD_QUEST_ID),
      },
      end("Interesting. I'll leave you to it.")
    ]
  ),
  node(
    "villagers",
    "Brennick's lot. They tolerate me. I keep the ridge-beasts from \
     wandering too close and they leave bread at the tower base every \
     third day. It is a functional arrangement.",
    &[
      DialogueChoice {
        text: "They said you might have work for me.",
        next: Some("work"),
        on_select: &[],
        condition: DialogueCondition::QuestInactive(WIZARD_QUEST_ID),
      },
      end("Good to know.")
    ]
  ),
  node(
    "teach",
    "Teach? No. But I could give you something. Scrolls — \
     intent crystallized into parchment. Throw one and the pattern \
     completes on impact. Frost, lightning, void. Simple things. \
     But I need my lens back first.",
    &[go("What lens?", "lens")]
  ),
  node(
    "work",
    "Work, no. A trade, perhaps. I lost something in the caves beneath \
     the snowfield and I am too old to go crawling after it.",
    &[go("What did you lose?", "lens")]
  ),
  node(
    "lens",
    "A RESONANCE LENS. Small, looks like a crystal disc with light \
     trapped inside. I dropped it in the deeper caves — third level \
     down, maybe fourth. The creatures didn't take it; they can't \
     touch it. But I can't get past them anymore. \
     Bring it back and I'll fill your pack with scrolls.",
    &[
      DialogueChoice {
        text: "I'll find it.",
        next: Some("accepted"),
        on_select: &WIZARD_QUEST_ACCEPT,
        condition: DialogueCondition::Always,
      },
      end("Not right now.")
    ]
  ),
  node(
    "accepted",
    "Deep caves, below the snowfield. You'll know the lens when you \
     see it — it hums. Don't try to use it yourself. \
     You wouldn't like what it shows you.",
    &[end("Understood.")]
  ),
  node(
    "progress",
    "Still searching? The caves go deep. The lens will be on a \
     ledge or in a dead end — it rolled until it stopped. \
     Listen for the hum.",
    &[end("I'll keep looking.")]
  ),
  node(
    "turn_in",
    "*takes the lens, holds it to the light* \
     ...Yes. Still intact. Still singing. \
     *long pause* \
     You've earned this. These scrolls are crude work — \
     a shape the lens taught me to fold into paper. Throw them \
     like grenades. The pattern does the rest.",
    &[
      DialogueChoice {
        text: "Much appreciated.",
        next: None,
        on_select: &WIZARD_TURN_IN,
        condition: DialogueCondition::Always,
      }
    ]
  ),
  node(
    "after",
    "The scrolls serve you well? Good. I may have more, in time. \
     The lens is slow to teach and I am slow to learn. \
     Come back when the snows change.",
    &[end("I will.")]
  ),
]);

fn wizard_npc() -> Object {
  Object::NPC_BASE
    .with(Named::s("Veradis", "A gaunt figure in layered robes the color of old ice. His eyes don't \
               quite focus on you — they're looking at something behind you, or \
               inside you, or nowhere at all."))
    .with(Stats { hp: 20, max_hp: 20, attack: 1, move_speed: 3.0, attack_speed: 1.0 })
    .with(npc_person_glyph('@', Color::srgb(0.4, 0.45, 0.75), Color::srgb(0.7, 0.72, 0.85)))
    .with(Collidable(true))
    .with(CreatureKind::Human)
    .with(Dialogue(&WIZARD_DIALOGUE))
    .with(Loadout::from_gear(&[]))
}

fn wizard_tower() -> Prefab {
  prefab(
    "
    ___sssss___
    __swwwwws__
    _swfffffws_
    swfffffffws
    swfffBfffws
    swffffnffws
    swfffffffws
    _swfffffws_
    __swwdwws__
    ___ssfss___
    "
  )
  .assoc('_', (Tile::BrightGround, []))
  .assoc('s', (Tile::SmallRocks, []))
  .assoc('w', (Tile::CaveWall, []))
  .assoc('f', (Tile::WoodFloor, []))
  .assoc('n', (Tile::WoodFloor, []))
  .assoc('B', (Tile::WoodFloor, [Object::TABLE]))
  .assoc('d', (Tile::WoodFloor, [Object::DOOR]))
}

fn build_wizard_tower(loc: &mut Location, village: (i32, i32), dock: (i32, i32), _seed: u64) -> (i32, i32) {
  let village_center = (village.0 + VILLAGE_W / 2, village.1 + VILLAGE_H / 2);

  // Place the tower at moderate distance from the village — not as far as the
  // lair, but far enough to feel like a trek. Search in a spiral from a point
  // offset from the village in a direction away from the dock.
  let offset_dir = (
    (village_center.0 - dock.0).signum().max(1),
    (village_center.1 - dock.1).signum().max(1)
  );
  let seed_point = (
    (village_center.0 + offset_dir.0 * 50).clamp(20, SIZE as i32 - 20),
    (village_center.1 + offset_dir.1 * 50).clamp(20, SIZE as i32 - 20)
  );

  let center = (0..3000)
    .find_map(|i| {
      let r = (i as f32).sqrt() * 1.5;
      let ang = i as f32 * 2.4;
      let x = seed_point.0 + (r * ang.cos()) as i32;
      let y = seed_point.1 + (r * ang.sin()) as i32;
      let in_bounds = x >= 10 && y >= 10 && x < SIZE as i32 - 10 && y < SIZE as i32 - 10;
      let far_enough = (x - village_center.0).abs() + (y - village_center.1).abs() > 40
        && (x - dock.0).abs() + (y - dock.1).abs() > 20;
      let area_clear = (-6..=6).all(|dy| (-6..=6).all(|dx|
        loc.level(0).get(x + dx, y + dy).is_some_and(snowy)
      ));
      (in_bounds && far_enough && area_clear).then_some((x, y))
    })
    .unwrap_or(seed_point);

  let tower = wizard_tower();
  let origin = (center.0 - 5, center.1 - 5);
  tower.stamp_level(loc.level_mut(0), origin.0, origin.1);
  tower.for_each_assoc_object(|lx, ly, obj| {
    loc.spawn_objects.push((origin.0 + lx, origin.1 + ly, 0, obj.clone()));
  });

  // Place the wizard NPC at the 'n' char position
  if let Some((nx, ny)) = tower.find_char('n') {
    loc.spawn_objects.push((origin.0 + nx, origin.1 + ny, 0, wizard_npc()));
  }

  // Lamp posts flanking the door
  loc.spawn_objects.push((center.0 - 2, center.1 + 5, 0, lamp_post()));
  loc.spawn_objects.push((center.0 + 2, center.1 + 5, 0, lamp_post()));

  // Compass marker outside the door
  loc.spawn_objects.push((
    center.0,
    center.1 + 5,
    0,
    Object::STRUCTURE_PASSABLE
      .with(Glyph::recolor_sprite(
        "textures/space_qud/crystal.png",
        '*',
        Color::srgb(0.5, 0.4, 0.9),
        Color::srgb(0.3, 0.2, 0.6)
      ))
      .with(Named::s("Wizard's Tower", "A squat stone tower. Faint light pulses in the windows."))
      .with(ShowOnCompass)
  ));

  center
}

/// Place the Resonance Lens in a deep cave level, far from the entrance stairs.
fn place_resonance_lens(loc: &mut Location, seed: u64) {
  let mut rng = SmallRng::seed_from_u64(seed ^ 0x1E45_0000);
  // Find the deepest cave level (before the cellar, which is the last level).
  // Cave levels start at index 1; the cellar is the last level.
  let cave_levels: Vec<usize> = (1..loc.levels.len())
    .filter(|&z| {
      // Cave levels have CaveWall fill; the cellar has CaveWall too but we want
      // to pick a level with cave entrances (stairs from above).
      loc.spawn_objects.iter().any(|&(_, _, sz, ref o)| {
        sz == z && Has::<Elevator>::get(o).is_some()
      })
    })
    .collect();

  let target_z = cave_levels.last().copied().unwrap_or(1);
  let level = loc.level(target_z);

  // BFS from all stair positions on this level to find the most remote walkable cell.
  let stair_positions: Vec<(i32, i32)> = loc.spawn_objects.iter()
    .filter(|&&(_, _, z, ref o)| z == target_z && Has::<Elevator>::get(o).is_some())
    .map(|&(x, y, _, _)| (x, y))
    .collect();

  let mut dist = vec![-1i32; SIZE * SIZE];
  let mut queue = VecDeque::new();
  for &(sx, sy) in &stair_positions {
    if sx >= 0 && sy >= 0 && (sx as usize) < SIZE && (sy as usize) < SIZE {
      dist[sy as usize * SIZE + sx as usize] = 0;
      queue.push_back((sx, sy));
    }
  }
  while let Some((x, y)) = queue.pop_front() {
    for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
      let (nx, ny) = (x + dx, y + dy);
      if nx >= 0 && ny >= 0 && (nx as usize) < SIZE && (ny as usize) < SIZE
        && dist[ny as usize * SIZE + nx as usize] < 0
        && level.walkable(nx, ny)
      {
        dist[ny as usize * SIZE + nx as usize] = dist[y as usize * SIZE + x as usize] + 1;
        queue.push_back((nx, ny));
      }
    }
  }

  // Pick a far-away walkable cell — from the top 20% most remote cells, pick randomly.
  let mut candidates: Vec<(i32, i32, i32)> = Vec::new();
  for y in 0..SIZE as i32 {
    for x in 0..SIZE as i32 {
      let d = dist[y as usize * SIZE + x as usize];
      if d > 0 {
        candidates.push((x, y, d));
      }
    }
  }
  candidates.sort_by_key(|&(_, _, d)| std::cmp::Reverse(d));
  let top = (candidates.len() / 5).max(1);
  let &(lx, ly, _) = &candidates[rng.gen_range(0..top)];

  loc.spawn_objects.push((lx, ly, target_z, Object::ground_item(Item::ResonanceLens).with(ShowOnCompass)));
}

/// Weighted random pick: `weights` are relative (don't need to sum to 1).
fn pick_weighted<T: Clone>(options: &[T], weights: &[u32], rng: &mut SmallRng) -> T {
  let total: u32 = weights.iter().sum();
  let roll = rng.gen_range(0..total);
  options
    .iter()
    .zip(weights.iter())
    .scan(0u32, |acc, (opt, w)| {
      *acc += w;
      Some((opt, *acc))
    })
    .find_map(|(opt, acc)| (roll < acc).then(|| opt.clone()))
    .unwrap_or_else(|| options.last().unwrap().clone())
}

fn pick_creature(tile: Tile, rng: &mut SmallRng) -> Object {
  let (options, weights): (Vec<Object>, Vec<u32>) = match tile {
    // Crystal mantises stalk the frozen lakes.
    Tile::IceFloor => (vec![Object::MANTIS_ALIEN, Object::CRAB_ALIEN], vec![60, 40]),
    _ => (
      vec![
        Object::MANTIS_ALIEN,
        Object::CRAB_ALIEN,
        Object::ALIEN_RUNNER,
        Object::RAT_SOLDIER,
        Object::ROBOT_DOG
      ],
      vec![30, 20, 20, 15, 15]
    )
  };
  pick_weighted(&options, &weights, rng)
}

/// Hostile fauna across snow and lake ice, kept clear of the village, dock, tower.
fn scatter_creatures(
  loc: &mut Location,
  village: (i32, i32),
  dock: (i32, i32),
  lair: (i32, i32),
  tower: (i32, i32),
  seed: u64
) {
  let mut rng = SmallRng::seed_from_u64(seed ^ 0xC01D_FEE7);
  let level = loc.level(0);
  let safe = |x: i32, y: i32| {
    let in_village = x >= village.0 - 6
      && x < village.0 + VILLAGE_W + 6
      && y >= village.1 - 6
      && y < village.1 + VILLAGE_H + 6;
    let near_dock = (x - dock.0).abs().max((y - dock.1).abs()) <= 15;
    let near_lair = (x - lair.0).abs().max((y - lair.1).abs()) <= 10;
    let near_tower = (x - tower.0).abs().max((y - tower.1).abs()) <= 10;
    !in_village && !near_dock && !near_lair && !near_tower
  };
  let mut snow_cells: Vec<(i32, i32)> = Vec::new();
  let mut ice_cells: Vec<(i32, i32)> = Vec::new();
  for y in 2..SIZE as i32 - 2 {
    for x in 2..SIZE as i32 - 2 {
      if safe(x, y) {
        match level.get(x, y) {
          Some(Tile::BrightGround) | Some(Tile::Ground) => snow_cells.push((x, y)),
          Some(Tile::IceFloor) => ice_cells.push((x, y)),
          _ => ()
        }
      }
    }
  }
  let mut spawns = Vec::new();
  for (cells, divisor, lo, hi) in
    [(&snow_cells, 2500, 8usize, 30usize), (&ice_cells, 2000, 3, 12)]
  {
    let count = (cells.len() / divisor).clamp(lo, hi);
    for _ in 0..count {
      if !cells.is_empty() {
        let (x, y) = cells[rng.gen_range(0..cells.len())];
        let tile = level.get(x, y).unwrap();
        spawns.push((x, y, 0, pick_creature(tile, &mut rng)));
      }
    }
  }
  loc.spawn_objects.extend(spawns);
}

pub fn generate() -> Location {
  let elev = NoiseField::new(SEED as u32, 0.0050, 4, 0.5);
  let detail = NoiseField::new((SEED ^ 0xBF58_476D) as u32, 0.025, 3, 0.45);
  let rock_mask = NoiseField::new((SEED ^ 0x94D0_49BB) as u32, 0.018, 2, 0.5);
  let tree_mask = NoiseField::new((SEED ^ 0x1234_5678) as u32, 0.020, 4, 0.5);

  let mut loc = Location::new(
    NAME,
    SIZE,
    SIZE,
    2,
    LocationType::PlanetSurface { breathable: true },
    Tile::BrightGround
  );
  let mut rng = SmallRng::seed_from_u64(SEED);

  // Pass 1: classify every cell by elevation.
  {
    let level = loc.level_mut(0);
    for y in 0..SIZE {
      for x in 0..SIZE {
        let e = elev.sample01(x as f64, y as f64) as f32;
        level.set(x as i32, y as i32, classify(e));
      }
    }
  }

  // Pass 1.5: snowdrifts blow onto lake ice where detail noise runs high —
  // but only in the shallower ice band, so lake cores stay clean ice and
  // the ragged drift edge reads as wind-blown shoreline.
  {
    let level = loc.level_mut(0);
    for y in 0..SIZE {
      for x in 0..SIZE {
        let e = elev.sample01(x as f64, y as f64) as f32;
        let d = detail.sample01(x as f64, y as f64) as f32;
        if level.get(x as i32, y as i32) == Some(Tile::IceFloor)
          && e > ELEV_SEA + 0.07
          && d > SNOWDRIFT_THRESHOLD
        {
          level.set(x as i32, y as i32, Tile::BrightGround);
        }
      }
    }
  }

  // Pass 2: gravel shoreline. Snow tiles touching ice or open water become
  // bare ground — the wind scours the snow off at every lake edge, which is
  // what makes a frozen shore read as a shore.
  {
    let level = loc.level_mut(0);
    let mut to_gravel = vec![false; SIZE * SIZE];
    for y in 1..SIZE - 1 {
      for x in 1..SIZE - 1 {
        let t = level.get(x as i32, y as i32).unwrap_or(Tile::Wall);
        let ice_near = [(1, 0), (-1, 0), (0, 1), (0, -1)].into_iter().any(|(dx, dy)| {
          level
            .get(x as i32 + dx, y as i32 + dy)
            .is_some_and(|n| matches!(n, Tile::IceFloor | Tile::DeepWater))
        });
        if t == Tile::BrightGround && ice_near {
          to_gravel[y * SIZE + x] = true;
        }
      }
    }
    for y in 0..SIZE {
      for x in 0..SIZE {
        if to_gravel[y * SIZE + x] {
          level.set(x as i32, y as i32, Tile::Ground);
        }
      }
    }
  }

  // Pass 3: boulder fields — sparse clusters of walkable scree on the snow.
  {
    let level = loc.level_mut(0);
    for y in 1..SIZE - 1 {
      for x in 1..SIZE - 1 {
        let r = rock_mask.sample01(x as f64, y as f64) as f32;
        let d = detail.sample01(x as f64, y as f64) as f32;
        if level.get(x as i32, y as i32) == Some(Tile::BrightGround)
          && r > ROCK_MASK_THRESHOLD
          && d > ROCK_DETAIL_THRESHOLD
        {
          level.set(x as i32, y as i32, Tile::SmallRocks);
        }
      }
    }
  }

  // Pass 3.5: meandering ridges — noise-steered random walks like the Vera
  // Spera fault lines, alternating glacier ice and dark rock so the barriers
  // don't all read the same.
  {
    let level = loc.level_mut(0);
    for ridge_idx in 0..RIDGE_COUNT {
      let material = if ridge_idx % 2 == 0 { Tile::IceWall } else { Tile::CaveWall };
      let mut x = rng.gen_range(20.0..(SIZE as f64 - 20.0));
      let mut y = rng.gen_range(20.0..(SIZE as f64 - 20.0));
      let mut heading = (rng.next_u64() as f64 / u64::MAX as f64) * std::f64::consts::TAU;
      for _step in 0..(SIZE * 5) {
        let gx = x as i32;
        let gy = y as i32;
        if gx >= 1 && gy >= 1 && gx < (SIZE as i32 - 1) && gy < (SIZE as i32 - 1)
          && !rng.gen_bool(RIDGE_GAP_CHANCE)
        {
          for dx in -RIDGE_THICKNESS..=RIDGE_THICKNESS {
            for dy in -RIDGE_THICKNESS..=RIDGE_THICKNESS {
              let (nx, ny) = (gx + dx, gy + dy);
              let in_footprint = dx.abs() + dy.abs() <= RIDGE_THICKNESS;
              let is_outer = dx.abs() + dy.abs() == RIDGE_THICKNESS;
              if in_footprint
                && nx >= 1 && ny >= 1 && nx < (SIZE as i32 - 1) && ny < (SIZE as i32 - 1)
                && !(is_outer && rng.gen_bool(0.4))
                && level.get(nx, ny).is_some_and(snowy)
              {
                level.set(nx, ny, material);
              }
            }
          }
        }
        let steer = detail.sample01(x, y + ridge_idx as f64 * 137.0);
        let turn = (steer - 0.5) * 1.2;
        let jitter = (rng.next_u64() as f64 / u64::MAX as f64 - 0.5) * 0.3;
        heading += turn + jitter;
        x += heading.cos() * 0.7;
        y += heading.sin() * 0.7;
      }
    }
  }

  // Pass 3.7: permafrost scars — small irregular patches of bare ground in
  // the snow, two thresholds from offset noise for a mix of patch sizes.
  {
    let level = loc.level_mut(0);
    for (offset, threshold) in [((1000.0, 500.0), 0.84), ((2500.0, 3700.0), 0.93)] {
      for y in 0..SIZE {
        for x in 0..SIZE {
          let d = detail.sample01(x as f64 + offset.0, y as f64 + offset.1) as f32;
          if level.get(x as i32, y as i32) == Some(Tile::BrightGround) && d > threshold {
            level.set(x as i32, y as i32, Tile::Ground);
          }
        }
      }
    }
  }

  // Pass 4: ship dock on the largest walkable landmass.
  let dock = place_ship_dock(loc.level_mut(0), Tile::BrightGround);

  // Pass 5: the taiga belt. Trees only at mid elevation, ramped by smoothstep
  // on the tree mask so groves fade into open snow — a treeline both above
  // (against the glaciers) and below (against the frozen valleys).
  {
    let level = loc.level(0);
    let mut trees = Vec::new();
    for y in 1..SIZE - 1 {
      for x in 1..SIZE - 1 {
        let e = elev.sample01(x as f64, y as f64) as f32;
        let density = tree_mask.sample01(x as f64, y as f64) as f32;
        // Fade tree probability to zero at both edges of the belt.
        let belt = smoothstep(TAIGA_LO, TAIGA_LO + 0.06, e)
          * (1.0 - smoothstep(TAIGA_HI - 0.06, TAIGA_HI, e));
        let p = smoothstep(TREE_BAND_LO, TREE_BAND_HI, density) * belt * TREE_MAX_PROB;
        let near_dock =
          (x as i32 - dock.0).abs() <= 1 && (y as i32 - dock.1).abs() <= 1;
        if level.get(x as i32, y as i32) == Some(Tile::BrightGround)
          && !near_dock
          && p > 0.0
          && rng.gen_bool(p as f64)
        {
          trees.push((x as i32, y as i32, 0, Object::random_tree()));
        }
      }
    }
    loc.spawn_objects.extend(trees);
  }

  // Pass 5.5: ice spires — rare crystal formations out on the lake ice.
  {
    let level = loc.level_mut(0);
    for y in 1..SIZE - 1 {
      for x in 1..SIZE - 1 {
        let d = detail.sample01(x as f64 + 4200.0, y as f64 + 900.0) as f32;
        if level.get(x as i32, y as i32) == Some(Tile::IceFloor) && d > 0.93 {
          level.set(x as i32, y as i32, Tile::CrystalFormation);
        }
      }
    }
  }

  // Pass 6: the shared multi-level cave system, entrances out in the wilds.
  cave_gen::generate_caves(&mut loc, SEED);

  // Pass 7: the village (after the caves, so the site can avoid their
  // entrances) and its cellar level.
  let village = build_village(&mut loc, dock);

  // Pass 7.5: the Frostmaw den, far out in the wilds opposite the village.
  let lair = build_lair(&mut loc, village, dock, SEED);

  // Pass 7.6: the wizard's tower, somewhere between the village and the wilds.
  let tower = build_wizard_tower(&mut loc, village, dock, SEED);

  // Pass 7.7: the Resonance Lens, deep in the caves.
  place_resonance_lens(&mut loc, SEED);

  // Pass 8: hostile fauna, kept away from the village, dock, den, and tower.
  scatter_creatures(&mut loc, village, dock, lair, tower, SEED);

  loc
}

#[cfg(test)]
mod tests {
  use {super::*, std::collections::VecDeque};

  fn glyph_for(tile: Tile) -> char {
    match tile {
      Tile::DeepWater => '~',
      Tile::IceFloor => '-',
      Tile::BrightGround => ' ',
      Tile::Ground => ':',
      Tile::SmallRocks => 'o',
      Tile::IceWall => '▒',
      Tile::CaveWall => '▓',
      Tile::CaveFloor => '·',
      Tile::CrystalFormation => '*',
      Tile::WoodWall => '#',
      Tile::WoodFloor | Tile::WoodTile => '.',
      Tile::Fence => '+',
      Tile::ShipDock => 'D',
      _ => '?'
    }
  }

  #[test]
  fn builds_glacial_terrain_village_and_cellar() {
    let loc = generate();
    let level = loc.level(0);

    // Biome mix sanity.
    let count = |wanted: Tile| {
      (0..SIZE as i32)
        .flat_map(|y| (0..SIZE as i32).map(move |x| (x, y)))
        .filter(|&(x, y)| level.get(x, y) == Some(wanted))
        .count()
    };
    let total = (SIZE * SIZE) as f32;
    for t in [
      Tile::BrightGround,
      Tile::IceFloor,
      Tile::DeepWater,
      Tile::Ground,
      Tile::SmallRocks,
      Tile::IceWall,
      Tile::CaveWall,
      Tile::CrystalFormation,
      Tile::WoodWall
    ] {
      eprintln!("  {:?}: {} ({:.1}%)", t, count(t), 100.0 * count(t) as f32 / total);
    }
    assert!(count(Tile::BrightGround) > 20_000, "expected snowfields");
    assert!(count(Tile::IceFloor) > 2_000, "expected frozen lakes");
    assert!(count(Tile::WoodWall) > 30, "expected village houses");
    assert_eq!(count(Tile::ShipDock), 1, "expected one dock");

    // Object census.
    let mut trees = 0;
    let mut doors = 0;
    let mut residents = 0;
    let mut creatures = 0;
    let mut surface_elevators = 0;
    let mut cellar_z = None;
    for &(x, y, z, ref obj) in &loc.spawn_objects {
      if Has::<Tree>::get(obj).is_some() {
        trees += 1;
      } else if Has::<Enemy>::get(obj).is_some() {
        creatures += 1;
      } else if Has::<Named>::get(obj).is_some_and(|n| n.name == "Door") {
        doors += 1;
      }
      if let Some(elevator) = Has::<Elevator>::get(obj) {
        if z == 0 {
          surface_elevators += 1;
        }
        if Has::<Named>::get(obj).is_some_and(|n| n.name == "Cellar Stairs") {
          cellar_z = elevator.floors.iter().find_map(|&(fz, fx, fy)| {
            (fz != 0).then_some((fz, fx, fy))
          });
          assert_eq!((x, y), (cellar_z.unwrap().1, cellar_z.unwrap().2));
        }
      }
      if Has::<Named>::get(obj)
        .is_some_and(|n| matches!(n.name, "Old Brennick" | "Suvi" | "Harrow" | "Wren the Smith" | "Pell" | "Mother Ilsa" | "Cellar-Keeper Odd" | "Veradis"))
      {
        residents += 1;
      }
    }
    eprintln!(
      "  trees {trees}  doors {doors}  residents {residents}  creatures {creatures}  surface elevators {surface_elevators}"
    );
    assert!(trees > 100, "expected a taiga belt, got {trees} trees");
    assert_eq!(doors, 7, "expected seven doors (6 houses + wizard tower)");
    assert_eq!(residents, 8, "expected eight residents (7 villagers + wizard)");
    assert!(creatures > 10, "expected hostile fauna");
    // 2-4 cave entrances + cellar stairs + cellar hatch.
    assert!(surface_elevators >= 4, "expected cave entrances plus cellar access");

    // The cellar must be a real, connected level: BFS from the stairs landing
    // must reach the supply cache and the hatch landing.
    let (cz, sx, sy) = cellar_z.expect("expected cellar stairs");
    let cellar = loc.level(cz);
    let mut dist = vec![-1i32; SIZE * SIZE];
    let mut queue = VecDeque::from([(sx, sy)]);
    dist[(sy as usize) * SIZE + sx as usize] = 0;
    while let Some((x, y)) = queue.pop_front() {
      for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
        let (nx, ny) = (x + dx, y + dy);
        if nx >= 0
          && ny >= 0
          && nx < SIZE as i32
          && ny < SIZE as i32
          && dist[(ny as usize) * SIZE + nx as usize] < 0
          && cellar.walkable(nx, ny)
        {
          dist[(ny as usize) * SIZE + nx as usize] = dist[(y as usize) * SIZE + x as usize] + 1;
          queue.push_back((nx, ny));
        }
      }
    }
    let reachable = |x: i32, y: i32| dist[(y as usize) * SIZE + x as usize] >= 0;
    for &(x, y, z, ref obj) in &loc.spawn_objects {
      if z == cz && Has::<LootChest>::get(obj).is_some() {
        assert!(reachable(x, y), "cellar loot at ({x},{y}) unreachable from stairs");
      }
      if z == cz && Has::<Elevator>::get(obj).is_some() {
        assert!(reachable(x, y), "cellar exit at ({x},{y}) unreachable from stairs");
      }
    }

    // Village closeup: find the footprint via the wood walls.
    let village_min = (0..SIZE as i32)
      .flat_map(|y| (0..SIZE as i32).map(move |x| (x, y)))
      .filter(|&(x, y)| level.get(x, y) == Some(Tile::WoodWall))
      .fold((SIZE as i32, SIZE as i32), |(mx, my), (x, y)| (mx.min(x), my.min(y)));
    let mut closeup = String::new();
    for y in village_min.1 - 4..village_min.1 + VILLAGE_H + 2 {
      for x in village_min.0 - 4..village_min.0 + VILLAGE_W + 2 {
        closeup.push(glyph_for(level.get(x, y).unwrap_or(Tile::Wall)));
      }
      closeup.push('\n');
    }
    eprintln!("village closeup:\n{closeup}");

    let mut cellar_view = String::new();
    for y in village_min.1 - 4..village_min.1 + VILLAGE_H + 2 {
      for x in village_min.0 - 4..village_min.0 + VILLAGE_W + 2 {
        cellar_view.push(glyph_for(cellar.get(x, y).unwrap_or(Tile::Wall)));
      }
      cellar_view.push('\n');
    }
    eprintln!("cellar beneath:\n{cellar_view}");

    // Downsampled overview.
    let mut canvas = String::new();
    for y in (0..SIZE as i32).step_by(4) {
      for x in (0..SIZE as i32).step_by(4) {
        canvas.push(glyph_for(level.get(x, y).unwrap()));
      }
      canvas.push('\n');
    }
    eprintln!("overview (1:4):\n{canvas}");
  }
}
