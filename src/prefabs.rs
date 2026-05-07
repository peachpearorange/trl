use std::collections::HashMap;

use crate::entities::{Glyph, Named, Object, Stats};
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

/// ASCII layout plus `.assoc` chains; yields spawn offsets only (tiles live in your [`crate::level::Level`]).
pub struct Prefab {
  layout: String,
  assocs: HashMap<char, Vec<Object>>,
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

  /// Each [`Object`] is [`Clone`]d per matching grid cell.
  pub fn assoc<const N: usize>(
    mut self,
    marker: impl AssocMarker,
    templates: [Object; N],
  ) -> Self {
    self
      .assocs
      .insert(marker.assoc_char(), Vec::from(templates));
    self
  }

  /// Local `(x, y)` offsets from the prefab’s top-left (same orientation as layout rows).
  pub fn build(self) -> Vec<PrefabObject> {
    compile_prefab(&self.layout, &self.assocs)
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

fn compile_prefab(layout: &str, assocs: &HashMap<char, Vec<Object>>) -> Vec<PrefabObject> {
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

  let mut spawns: Vec<PrefabObject> = Vec::new();

  for (y, line) in lines.iter().enumerate() {
    for (x, ch) in line.chars().enumerate() {
      if ch.is_whitespace() {
        continue;
      }
      if let Some(templates) = assocs.get(&ch) {
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

  spawns
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

pub fn small_building_with_npc() -> Vec<PrefabObject> {
  prefab(
    "
    wwwww
    wfffw
    wfnfw
    wwdfw
    wwwww
    ",
  )
  .assoc('w', [])
  .assoc('f', [])
  .assoc('d', [])
  .assoc('n', [resident()])
  .build()
}

pub fn small_spaceship() -> Vec<PrefabObject> {
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
  .assoc('b', [])
  .assoc('w', [])
  .assoc('.', [])
  .assoc('c', [Object::flight_console()])
  .assoc('a', [])
  .assoc('p', [ship_pilot()])
  .build()
}

#[cfg(test)]
mod tests {
  use super::*;

  fn chest() -> Object { Object::loot_chest() }

  fn enemy() -> Object { Object::rat_soldier() }

  #[test]
  fn builds_multiple_objects_at_same_cell() {
    let spawns = prefab(
      "
            www
            wkw
            www
            ",
    )
    .assoc('k', [chest(), enemy()])
    .assoc('w', [])
    .build();

    assert_eq!(spawns.len(), 2);
    assert!(spawns.iter().all(|spawn| spawn.x == 1 && spawn.y == 1));
  }

  #[test]
  fn assoc_accepts_one_char_string() {
    let spawns = prefab(
      "
aa
aa
",
    )
    .assoc("a", [])
    .build();
    assert!(spawns.is_empty());
  }

  #[test]
  fn unknown_chars_emit_error_and_spawn_nothing_for_that_cell() {
    let spawns = prefab(".x").assoc('.', []).build();

    assert!(spawns.is_empty());
  }

  #[test]
  fn accepts_object_templates() {
    let spawns = prefab("c").assoc('c', [Object::loot_chest()]).build();

    assert_eq!(spawns.len(), 1);
  }

  #[test]
  fn small_building_has_one_npc_at_expected_offset() {
    let spawns = small_building_with_npc();

    assert_eq!(spawns.len(), 1);
    assert_eq!((spawns[0].x, spawns[0].y), (2, 2));
  }

  #[test]
  fn small_spaceship_has_console_and_pilot_offsets() {
    let spawns = small_spaceship();

    assert_eq!(spawns.len(), 2);
    let origins: Vec<(i32, i32)> = spawns.iter().map(|s| (s.x, s.y)).collect();
    assert!(origins.contains(&(3, 2)));
    assert!(origins.contains(&(3, 3)));
  }
}
