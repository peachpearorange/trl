use crate::{entities::*,
            galaxy::{Location, LocationId},
            level::{LocationType, Tile}};

pub const ID: LocationId = (2, 0, 0);

const SAVE_DATA: &str = include_str!("../../editor_saves/lavaplanetbigger.txt");

fn object_from_idx(idx: u8) -> Option<Object> {
  match idx {
    0 => Some(Object::random_tree()),
    1 => Some(Object::BOULDER),
    2 => Some(Object::DOOR),
    3 => Some(Object::AIRLOCK_DOOR),
    4 => Some(Object::BED),
    5 => Some(Object::TABLE),
    6 => Some(Object::CHAIR),
    7 => Some(Object::CRAFTING_TABLE),
    8 => Some(Object::LOCKER),
    9 => Some(Object::CRATE_OBJ),
    10 => Some(Object::LOOT_CHEST),
    11 => Some(Object::FLIGHT_CONSOLE),
    12 => Some(Object::LOADOUT_CONSOLE),
    13 => Some(Object::SPACE_CAT),
    14 => Some(Object::THRUSTER),
    15 => Some(Object::RAT_SOLDIER),
    16 => Some(Object::ARMORED_RAT_SOLDIER),
    17 => Some(Object::ROBOT),
    18 => Some(Object::WACK_ROBOT),
    19 => Some(Object::ALIEN_RUNNER),
    20 => Some(Object::LAVA_CRAB),
    21 => Some(Object::MANTIS_ALIEN),
    22 => Some(Object::CRAB_ALIEN),
    23 => Some(Object::MUSHROOM_CREATURE),
    24 => Some(Object::GRENADE_THROWER),
    25 => Some(Object::GUNMAN),
    26 => Some(Object::LASER_SWORD),
    27 => Some(Object::TURRET),
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
