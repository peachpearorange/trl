use crate::{entities::Object,
            galaxy::{Location, LocationId},
            level::{LocationType, Tile}};

pub const ID: LocationId = (2, 0, 0);

const SAVE_DATA: &str = include_str!("../../editor_saves/lavaplanetbigger.txt");

fn object_from_idx(idx: u8) -> Option<Object> {
  match idx {
    0 => Some(Object::tree()),
    1 => Some(Object::boulder()),
    2 => Some(Object::door()),
    3 => Some(Object::airlock_door()),
    4 => Some(Object::bed()),
    5 => Some(Object::table()),
    6 => Some(Object::chair()),
    7 => Some(Object::crafting_table()),
    8 => Some(Object::locker()),
    9 => Some(Object::crate_obj()),
    10 => Some(Object::loot_chest()),
    11 => Some(Object::flight_console()),
    12 => Some(Object::loadout_console()),
    13 => Some(Object::space_cat()),
    14 => Some(Object::thruster()),
    15 => Some(Object::rat_soldier()),
    16 => Some(Object::armored_rat_soldier()),
    17 => Some(Object::robot()),
    18 => Some(Object::wack_robot()),
    19 => Some(Object::alien_runner()),
    20 => Some(Object::lava_crab()),
    21 => Some(Object::mantis_alien()),
    22 => Some(Object::crab_alien()),
    23 => Some(Object::mushroom_creature()),
    24 => Some(Object::grenade_thrower()),
    25 => Some(Object::gunman()),
    26 => Some(Object::laser_sword()),
    27 => Some(Object::turret()),
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
