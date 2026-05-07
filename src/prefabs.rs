use crate::{entities::{Glyph, Named, Object, Stats},
            level::{Level, Tile}};
use bevy::prelude::Color;

pub type ObjectFactory = fn() -> Object;
pub type Assoc = (char, Tile, Vec<ObjectFactory>);

pub struct PrefabObject {
  pub object: Object,
  pub x: i32,
  pub y: i32
}

impl PrefabObject {
  pub fn spawn_at_z(
    self,
    commands: &mut bevy::prelude::Commands,
    z: usize
  ) -> bevy::prelude::Entity {
    self.object.spawn_at(commands, self.x, self.y, z)
  }
}

pub fn prefab_area(assocs: &[Assoc], layout: &str) -> (Level, Vec<PrefabObject>) {
  let raw_lines: Vec<&str> = layout.lines().filter(|l| !l.trim().is_empty()).collect();
  let indent = raw_lines
    .iter()
    .filter_map(|line| {
      line.char_indices().find(|(_, ch)| !ch.is_whitespace()).map(|(i, _)| i)
    })
    .min()
    .unwrap_or(0);
  let lines: Vec<&str> =
    raw_lines.iter().map(|line| line.get(indent..).unwrap_or(line)).collect();
  let height = lines.len();
  let width = lines.iter().map(|l| l.len()).max().unwrap_or(0);

  let mut level = Level::new(width, height, Tile::Vacuum);
  let mut spawns: Vec<PrefabObject> = Vec::new();

  for (y, line) in lines.iter().enumerate() {
    for (x, ch) in line.chars().enumerate() {
      if let Some(&(_, tile, ref factories)) = assocs.iter().find(|&&(assoc_ch, ..)| assoc_ch == ch)
      {
        level.set(x as i32, y as i32, tile);
        for factory in factories {
          spawns.push(PrefabObject { object: factory(), x: x as i32, y: y as i32 });
        }
      }
    }
  }

  (level, spawns)
}

fn resident() -> Object {
  Object::npc().add((
    Named {
      name: "Resident",
      flavor: "Someone trying to keep a small place livable.",
    },
    Stats { hp: 8, max_hp: 8, attack: 1, move_speed: 3.0, attack_speed: 1.0 },
    Glyph::ascii('@', Color::srgb(0.7, 0.9, 1.0)),
  ))
}

/// Crew on the small-ship prefab; neutral, non-blocking.
fn ship_pilot() -> Object {
  Object::npc().add((
    Named {
      name: "Pilot",
      flavor: "Ticks through a short pre-flight list. Coffee stains on the console manual.",
    },
    Stats { hp: 10, max_hp: 10, attack: 1, move_speed: 3.0, attack_speed: 1.0 },
    Glyph::ascii('@', Color::srgb(0.55, 0.82, 0.95)),
  ))
}

pub fn small_building_with_npc() -> (Level, Vec<PrefabObject>) {
  prefab_area(
    &[
      ('w', Tile::StationWall, vec![]),
      ('f', Tile::StationFloor, vec![]),
      ('d', Tile::Door, vec![]),
      ('n', Tile::StationFloor, vec![resident]),
    ],
    "
    wwwww
    wfffw
    wfnfw
    wwdfw
    wwwww
    "
  )
}

/// Compact vessel: bulkhead shell, viewport row, flight console, aft airlock, one crew NPC.
pub fn small_spaceship() -> (Level, Vec<PrefabObject>) {
  prefab_area(
    &[
      ('b', Tile::Bulkhead, vec![]),
      ('w', Tile::Window, vec![]),
      ('.', Tile::DeckPlate, vec![]),
      ('c', Tile::DeckPlate, vec![Object::flight_console]),
      ('a', Tile::AirlockDoor, vec![]),
      ('p', Tile::DeckPlate, vec![ship_pilot]),
    ],
    "
    bbbbbbb
    bwwwwwb
    b..p..b
    b..c..b
    b.....b
    b..a..b
    bbbbbbb
    ",
  )
}

#[cfg(test)]
mod tests {
  use super::*;

  fn chest() -> Object { Object::loot_chest() }

  fn enemy() -> Object { Object::rat_soldier() }

  #[test]
  fn builds_tiles_and_multiple_objects_from_layout() {
    let (level, spawns) = prefab_area(
      &[
        ('k', Tile::Floor, vec![chest, enemy]),
        ('w', Tile::Wall, vec![]),
      ],
      "
            www
            wkw
            www
            "
    );

    assert_eq!(level.width, 3);
    assert_eq!(level.height, 3);
    assert_eq!(level.get(1, 1), Some(Tile::Floor));
    assert_eq!(level.get(0, 0), Some(Tile::Wall));
    assert_eq!(spawns.len(), 2);
    assert!(spawns.iter().all(|spawn| spawn.x == 1 && spawn.y == 1));
  }

  #[test]
  fn unknown_chars_stay_as_default_tile() {
    let (level, spawns) = prefab_area(&[('.', Tile::DeckPlate, vec![])], ".x");

    assert_eq!(level.get(0, 0), Some(Tile::DeckPlate));
    assert_eq!(level.get(1, 0), Some(Tile::Vacuum));
    assert!(spawns.is_empty());
  }

  #[test]
  fn accepts_associated_object_constructors() {
    let (_, spawns) = prefab_area(&[('c', Tile::DeckPlate, vec![Object::loot_chest])], "c");

    assert_eq!(spawns.len(), 1);
  }

  #[test]
  fn small_building_contains_walls_door_and_npc() {
    let (level, spawns) = small_building_with_npc();

    assert_eq!(level.width, 5);
    assert_eq!(level.height, 5);
    assert_eq!(level.get(0, 0), Some(Tile::StationWall));
    assert_eq!(level.get(2, 3), Some(Tile::Door));
    assert_eq!(level.get(2, 2), Some(Tile::StationFloor));
    assert_eq!(spawns.len(), 1);
    assert_eq!((spawns[0].x, spawns[0].y), (2, 2));
  }

  #[test]
  fn small_spaceship_has_airlock_console_and_pilot() {
    let (level, spawns) = small_spaceship();

    assert_eq!(level.width, 7);
    assert_eq!(level.height, 7);
    assert_eq!(level.get(3, 5), Some(Tile::AirlockDoor));
    assert_eq!(level.get(3, 3), Some(Tile::DeckPlate));
    assert_eq!(spawns.len(), 2);
    let origins: Vec<(i32, i32)> = spawns.iter().map(|s| (s.x, s.y)).collect();
    assert!(origins.contains(&(3, 2)));
    assert!(origins.contains(&(3, 3)));
  }
}
