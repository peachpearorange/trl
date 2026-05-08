use std::collections::HashMap;

use crate::entities::{Glyph, Named, Object, Stats};
use crate::level::Tile;
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

/// Placing a [`Prefab`] into a level: each cell’s [`Tile`] plus any [`PrefabObject`] spawns (local offsets).
pub struct PrefabBuild {
  pub tiles: Vec<(i32, i32, Tile)>,
  pub spawns: Vec<PrefabObject>,
}

/// ASCII layout plus `.assoc(…, (Tile, […]))` chains. Call [`.build`](Prefab::build) for local tile/spawn data.
pub struct Prefab {
  layout: String,
  assocs: HashMap<char, (Tile, Vec<Object>)>,
}

pub fn prefab(layout: impl Into<String>) -> Prefab {
  Prefab::new(layout)
}

impl Prefab {
  pub fn new(layout: impl Into<String>) -> Self {
    Self {
      layout: layout.into(),
      assocs: HashMap::new(),
    }
  }

  /// `(tile, object templates …)` per layout character. Each [`Object`] is [`Clone`]d per grid cell.
  pub fn assoc<const N: usize>(
    mut self,
    marker: impl AssocMarker,
    (tile, templates): (Tile, [Object; N]),
  ) -> Self {
    self
      .assocs
      .insert(marker.assoc_char(), (tile, Vec::from(templates)));
    self
  }

  /// Local `(x, y)` from the layout’s top-left, same row order as the string.
  pub fn build(self) -> PrefabBuild {
    compile_prefab(&self.layout, &self.assocs)
  }

  pub fn small_building_with_npc() -> Self {
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
  }

  pub fn small_spaceship() -> Self {
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
  }
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

fn compile_prefab(
  layout: &str,
  assocs: &HashMap<char, (Tile, Vec<Object>)>,
) -> PrefabBuild {
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

  let mut tiles = Vec::new();
  let mut spawns = Vec::new();

  for (y, line) in lines.iter().enumerate() {
    for (x, ch) in line.chars().enumerate() {
      if ch.is_whitespace() {
        continue;
      }
      if let Some((tile, templates)) = assocs.get(&ch) {
        tiles.push((x as i32, y as i32, *tile));
        for template in templates {
          spawns.push(PrefabObject {
            object: template.clone(),
            x: x as i32,
            y: y as i32,
          });
        }
      } else {
        bevy::log::error!(
          "prefab: layout character {:?} at ({}, {}) has no .assoc — ignored",
          ch,
          x,
          y
        );
      }
    }
  }

  PrefabBuild { tiles, spawns }
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

#[cfg(test)]
mod tests {
  use super::*;

  fn chest() -> Object { Object::loot_chest() }

  fn enemy() -> Object { Object::rat_soldier() }

  #[test]
  fn builds_multiple_objects_at_same_cell() {
    let PrefabBuild { tiles, spawns } = prefab(
      "
            www
            wkw
            www
            ",
    )
    .assoc('k', (Tile::Floor, [chest(), enemy()]))
    .assoc('w', (Tile::Wall, []))
    .build();

    assert_eq!(tiles.len(), 9);
    assert_eq!(spawns.len(), 2);
    assert!(spawns.iter().all(|spawn| spawn.x == 1 && spawn.y == 1));
  }

  #[test]
  fn assoc_accepts_one_char_string() {
    let PrefabBuild { tiles, spawns } = prefab(
      "
aa
aa
",
    )
    .assoc("a", (Tile::DeckPlate, []))
    .build();
    assert!(spawns.is_empty());
    assert_eq!(tiles.len(), 4);
    assert!(tiles.iter().all(|(_, _, t)| *t == Tile::DeckPlate));
  }

  #[test]
  fn unknown_chars_emit_error_and_spawn_nothing_for_that_cell() {
    let PrefabBuild { tiles, spawns } = prefab(".x")
      .assoc('.', (Tile::DeckPlate, []))
      .build();

    assert_eq!(tiles.len(), 1);
    assert!(spawns.is_empty());
  }

  #[test]
  fn accepts_object_templates() {
    let PrefabBuild { spawns, .. } = prefab("c")
      .assoc('c', (Tile::DeckPlate, [Object::loot_chest()]))
      .build();

    assert_eq!(spawns.len(), 1);
  }

  #[test]
  fn small_building_has_one_npc_at_expected_offset() {
    let PrefabBuild { spawns, .. } = Prefab::small_building_with_npc().build();

    assert_eq!(spawns.len(), 1);
    assert_eq!((spawns[0].x, spawns[0].y), (2, 2));
  }

  #[test]
  fn small_spaceship_has_console_and_pilot_offsets() {
    let PrefabBuild { spawns, .. } = Prefab::small_spaceship().build();

    assert_eq!(spawns.len(), 2);
    let origins: Vec<(i32, i32)> = spawns.iter().map(|s| (s.x, s.y)).collect();
    assert!(origins.contains(&(3, 2)));
    assert!(origins.contains(&(3, 3)));
  }
}
