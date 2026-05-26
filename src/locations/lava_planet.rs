use crate::{entities::*,
            galaxy::{Location, LocationId},
            level::{LocationType, Tile}};

pub const ID: LocationId = (2, 0, 0);

const SAVE_DATA: &str = include_str!("../../editor_saves/lavaplanetbigger.txt");

fn object_from_idx(idx: u8) -> Option<Object> {
  match idx {
    0 => Some(Object::tree()),
    1 => Some(Object::BOULDER.clone()),
    2 => Some(Object::DOOR.clone()),
    3 => Some(Object::AIRLOCK_DOOR.clone()),
    4 => Some(Object::BED.clone()),
    5 => Some(Object::TABLE.clone()),
    6 => Some(Object::CHAIR.clone()),
    7 => Some(Object::CRAFTING_TABLE.clone()),
    8 => Some(Object::LOCKER.clone()),
    9 => Some(Object::CRATE_OBJ.clone()),
    10 => Some(Object::LOOT_CHEST.clone()),
    11 => Some(Object::FLIGHT_CONSOLE.clone()),
    12 => Some(Object::LOADOUT_CONSOLE.clone()),
    13 => Some(Object::SPACE_CAT.clone()),
    14 => Some(Object::THRUSTER.clone()),
    15 => Some(Object::RAT_SOLDIER.clone()),
    16 => Some(Object::ARMORED_RAT_SOLDIER.clone()),
    17 => Some(Object::ROBOT.clone()),
    18 => Some(Object::WACK_ROBOT.clone()),
    19 => Some(Object::ALIEN_RUNNER.clone()),
    20 => Some(Object::LAVA_CRAB.clone()),
    21 => Some(Object::MANTIS_ALIEN.clone()),
    22 => Some(Object::CRAB_ALIEN.clone()),
    23 => Some(Object::MUSHROOM_CREATURE.clone()),
    24 => Some(Object::GRENADE_THROWER.clone()),
    25 => Some(Object::GUNMAN.clone()),
    26 => Some(Object::LASER_SWORD.clone()),
    27 => Some(Object::TURRET.clone()),
    _ => None
  }
}

pub fn generate() -> Location {
  let mut nums = SAVE_DATA.split_whitespace();
  let w: usize = nums.next().and_then(|s| s.parse().ok()).unwrap_or(0);
  let h: usize = nums.next().and_then(|s| s.parse().ok()).unwrap_or(0);
  let _: i32 = nums.next().and_then(|s| s.parse().ok()).unwrap_or(0);
  let _: i32 = nums.next().and_then(|s| s.parse().ok()).unwrap_or(0);

  let mut loc = Location::new("Lava Planet", w, h, 1, LocationType::PlanetSurface { breathable: false }, Tile::Ash);
  let level = loc.level_mut(0);
  let mut spawns = Vec::new();

  for y in 0..h {
    for x in 0..w {
      if let (Some(ts), Some(os)) =
        (nums.next().and_then(|s| s.parse::<u16>().ok()),
         nums.next().and_then(|s| s.parse::<i16>().ok()))
      {
        if let Ok(tile) = Tile::try_from(ts) {
          level.set(x as i32, y as i32, tile);
        }
        if os >= 0 {
          if let Some(obj) = object_from_idx(os as u8) {
            spawns.push((x as i32, y as i32, 0, obj));
          }
        }
      }
    }
  }
  loc.spawn_objects = spawns;

  loc
}
