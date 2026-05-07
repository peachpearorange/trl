use crate::{entities::{Glyph, Named, Object, Stats},
            level::{Level, Tile}};
use bevy::prelude::Color;

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

/// ASCII layout plus `.assoc` chains; pass `[Object::flight_console()]`-style templates ([`Object`] is cheaply cloned per grid cell).
pub fn prefab(layout: impl Into<String>) -> PrefabBuilder {
  PrefabBuilder {
    layout: layout.into(),
    assocs: Vec::new(),
  }
}

pub struct PrefabBuilder {
  layout: String,
  assocs: Vec<(char, Tile, Vec<Object>)>,
}

pub trait AssocMarker {
  fn assoc_char(self) -> char;
}

impl AssocMarker for char {
  fn assoc_char(self) -> char {
    self
  }
}

impl AssocMarker for &str {
  fn assoc_char(self) -> char {
    let mut it = self.chars();
    let c = it.next().expect("prefab assoc key must not be empty");
    assert!(it.next().is_none(), "prefab assoc key must be one character");
    c
  }
}

impl PrefabBuilder {
  /// Each [`Object`] is [`Clone`]d per matching grid cell (spawn templates from `Object::npc()`, …).
  pub fn assoc<const N: usize>(
    mut self,
    marker: impl AssocMarker,
    (tile, templates): (Tile, [Object; N]),
  ) -> Self {
    self
      .assocs
      .push((marker.assoc_char(), tile, Vec::from(templates)));
    self
  }

  pub fn build(self) -> (Level, Vec<PrefabObject>) {
    compile_prefab(&self.layout, &self.assocs)
  }
}

fn compile_prefab(layout: &str, assocs: &[(char, Tile, Vec<Object>)]) -> (Level, Vec<PrefabObject>) {
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
        for template in factories {
          spawns.push(PrefabObject {
            object: template.clone(),
            x: x as i32,
            y: y as i32,
          });
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
  prefab(
    "
    wwwww
    wfffw
    wfnfw
    wwdfw
    wwwww
    ",
  )
  .assoc('w', (Tile::StationWall, []))
  .assoc('f', (Tile::StationFloor, []))
  .assoc('d', (Tile::Door, []))
  .assoc('n', (Tile::StationFloor, [resident()]))
  .build()
}

/// Compact vessel: bulkhead shell, viewport row, flight console, aft airlock, one crew NPC.
pub fn small_spaceship() -> (Level, Vec<PrefabObject>) {
  prefab(
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
  .assoc('b', (Tile::Bulkhead, []))
  .assoc('w', (Tile::Window, []))
  .assoc('.', (Tile::DeckPlate, []))
  .assoc('c', (Tile::DeckPlate, [Object::flight_console()]))
  .assoc('a', (Tile::AirlockDoor, []))
  .assoc('p', (Tile::DeckPlate, [ship_pilot()]))
  .build()
}

#[cfg(test)]
mod tests {
  use super::*;

  fn chest() -> Object { Object::loot_chest() }

  fn enemy() -> Object { Object::rat_soldier() }

  #[test]
  fn builds_tiles_and_multiple_objects_from_layout() {
    let (level, spawns) = prefab(
      "
            www
            wkw
            www
            ",
    )
    .assoc('k', (Tile::Floor, [chest(), enemy()]))
    .assoc('w', (Tile::Wall, []))
    .build();

    assert_eq!(level.width, 3);
    assert_eq!(level.height, 3);
    assert_eq!(level.get(1, 1), Some(Tile::Floor));
    assert_eq!(level.get(0, 0), Some(Tile::Wall));
    assert_eq!(spawns.len(), 2);
    assert!(spawns.iter().all(|spawn| spawn.x == 1 && spawn.y == 1));
  }

  #[test]
  fn assoc_accepts_one_char_string() {
    let (level, _) = prefab(
      "
aa
aa
",
    )
    .assoc("a", (Tile::DeckPlate, []))
    .build();
    assert_eq!(level.get(0, 0), Some(Tile::DeckPlate));
  }

  #[test]
  fn unknown_chars_stay_as_default_tile() {
    let (level, spawns) = prefab(".x").assoc('.', (Tile::DeckPlate, [])).build();

    assert_eq!(level.get(0, 0), Some(Tile::DeckPlate));
    assert_eq!(level.get(1, 0), Some(Tile::Vacuum));
    assert!(spawns.is_empty());
  }

  #[test]
  fn accepts_associated_object_constructors() {
    let (_, spawns) = prefab("c")
      .assoc('c', (Tile::DeckPlate, [Object::loot_chest()]))
      .build();

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
