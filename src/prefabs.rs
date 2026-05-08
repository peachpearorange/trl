use std::collections::HashMap;

use crate::entities::{Glyph, Named, Object, Stats};
use crate::level::{Level, Tile};
use bevy::prelude::{Color, Commands};

/// ASCII layout plus `.assoc(…, (Tile, […]))` chains. Apply with [`Prefab::stamp_level`] /
/// [`Prefab::stamp_entities`].
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

  /// Full starter ship deck (`SHIP_WIDTH` × `SHIP_HEIGHT`), matching the former procedural layout.
  pub fn starting_ship() -> Self {
    prefab(
      "###WWWWWWWWWWWWWW###
#..................#
#.........C........#
W..................W
W.....a............W
W.....#............W
W.....#............W
W..................W
W............#.....W
W............a..=..W
W............#..==.W
W..................W
#..................#
#..................#
##########a#########",
    )
    .assoc('#', (Tile::Bulkhead, []))
    .assoc('.', (Tile::DeckPlate, []))
    .assoc('W', (Tile::Window, []))
    .assoc('a', (Tile::AirlockDoor, []))
    .assoc('=', (Tile::Conduit, []))
    .assoc('C', (Tile::DeckPlate, [Object::flight_console()]))
  }

  /// Write tiles into `level` at `(ox + x, oy + y)` for each layout cell.
  pub fn stamp_level(&self, level: &mut Level, ox: i32, oy: i32) {
    self.visit_cells(|lx, ly, tile, _templates| {
      level.set(ox + lx, oy + ly, tile);
    });
  }

  /// Spawn assoc objects at world coords `(ox + x, oy + y, z)`.
  pub fn stamp_entities(&self, commands: &mut Commands, ox: i32, oy: i32, z: usize) {
    self.visit_cells(|lx, ly, _tile, templates| {
      let wx = ox + lx;
      let wy = oy + ly;
      for template in templates {
        template.clone().spawn_at(commands, wx, wy, z);
      }
    });
  }

  /// Visit each `(x, y)` assoc object template (layout-local coords).
  pub fn for_each_assoc_object(&self, mut f: impl FnMut(i32, i32, &Object)) {
    self.visit_cells(|lx, ly, _, templates| {
      for t in templates {
        f(lx, ly, t);
      }
    });
  }

  fn visit_cells<F: FnMut(i32, i32, Tile, &[Object])>(&self, mut f: F) {
    let lines = normalized_lines(&self.layout);
    for (y, line) in lines.iter().enumerate() {
      let y = y as i32;
      for (x, ch) in line.chars().enumerate() {
        let x = x as i32;
        if ch.is_whitespace() {
        } else if let Some((tile, templates)) = self.assocs.get(&ch) {
          f(x, y, *tile, templates.as_slice());
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

fn normalized_lines(layout: &str) -> Vec<&str> {
  let raw_lines: Vec<&str> = layout.lines().filter(|l| !l.trim().is_empty()).collect();
  let indent = raw_lines
    .iter()
    .filter_map(|line| {
      line.char_indices().find(|(_, ch)| !ch.is_whitespace()).map(|(i, _)| i)
    })
    .min()
    .unwrap_or(0);
  raw_lines
    .iter()
    .map(|line| line.get(indent..).unwrap_or(line))
    .collect()
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
mod ship_legacy_reference {
  use crate::level::{Level, Tile};
  use crate::ship::{SHIP_HEIGHT, SHIP_WIDTH};

  pub fn legacy_fill_ship(deck: &mut Level) {
    for y in 0..SHIP_HEIGHT as i32 {
      for x in 0..SHIP_WIDTH as i32 {
        let is_edge =
          x == 0 || x == SHIP_WIDTH as i32 - 1 || y == 0 || y == SHIP_HEIGHT as i32 - 1;
        deck.set(
          x,
          y,
          if is_edge {
            Tile::Bulkhead
          } else {
            Tile::DeckPlate
          },
        );
      }
    }
    deck.set(10, 14, Tile::AirlockDoor);
    for x in 3..17 {
      deck.set(x, 0, Tile::Window);
    }
    for y in 3..12 {
      deck.set(0, y, Tile::Window);
      deck.set(SHIP_WIDTH as i32 - 1, y, Tile::Window);
    }
    for y in 4..7 {
      deck.set(6, y, Tile::Bulkhead);
    }
    deck.set(6, 4, Tile::AirlockDoor);
    for y in 8..11 {
      deck.set(13, y, Tile::Bulkhead);
    }
    deck.set(13, 9, Tile::AirlockDoor);
    deck.set(16, 10, Tile::Conduit);
    deck.set(17, 10, Tile::Conduit);
    deck.set(16, 9, Tile::Conduit);
  }

  #[test]
  fn starting_ship_matches_legacy_tiles() {
    use super::Prefab;

    let mut legacy = Level::new(SHIP_WIDTH, SHIP_HEIGHT, Tile::Vacuum);
    legacy_fill_ship(&mut legacy);
    let mut stamped = Level::new(SHIP_WIDTH, SHIP_HEIGHT, Tile::Vacuum);
    Prefab::starting_ship().stamp_level(&mut stamped, 0, 0);
    assert_eq!(legacy.tiles, stamped.tiles);
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::level::{Level, Tile};

  fn chest() -> Object { Object::loot_chest() }

  fn enemy() -> Object { Object::rat_soldier() }

  #[test]
  fn builds_multiple_objects_at_same_cell() {
    let p = prefab(
      "
            www
            wkw
            www
            ",
    )
    .assoc('k', (Tile::Floor, [chest(), enemy()]))
    .assoc('w', (Tile::Wall, []));

    let mut level = Level::new(3, 3, Tile::Vacuum);
    p.stamp_level(&mut level, 0, 0);
    let mut n_tiles = 0usize;
    for y in 0..3 {
      for x in 0..3 {
        if level.get(x, y).is_some_and(|t| t != Tile::Vacuum) {
          n_tiles += 1;
        }
      }
    }
    let mut spawn_cells: Vec<(i32, i32)> = Vec::new();
    p.for_each_assoc_object(|x, y, _| {
      spawn_cells.push((x, y));
    });

    assert_eq!(n_tiles, 9);
    assert_eq!(spawn_cells.len(), 2);
    assert!(spawn_cells.iter().all(|&(x, y)| x == 1 && y == 1));
  }

  #[test]
  fn assoc_accepts_one_char_string() {
    let p = prefab(
      "
aa
aa
",
    )
    .assoc("a", (Tile::DeckPlate, []));

    let mut deck = Level::new(2, 2, Tile::Vacuum);
    p.stamp_level(&mut deck, 0, 0);
    let mut n = 0usize;
    for y in 0..2 {
      for x in 0..2 {
        if deck.get(x, y) == Some(Tile::DeckPlate) {
          n += 1;
        }
      }
    }
    let mut objects = 0usize;
    p.for_each_assoc_object(|_, _, _| {
      objects += 1;
    });
    assert_eq!(objects, 0);
    assert_eq!(n, 4);
  }

  #[test]
  fn unknown_chars_emit_error_and_spawn_nothing_for_that_cell() {
    let p = prefab(".x").assoc('.', (Tile::DeckPlate, []));
    let mut level = Level::new(2, 1, Tile::Vacuum);
    p.stamp_level(&mut level, 0, 0);

    assert_eq!(level.get(0, 0), Some(Tile::DeckPlate));
    assert_eq!(level.get(1, 0), Some(Tile::Vacuum));
  }

  #[test]
  fn accepts_object_templates() {
    let p = prefab("c").assoc('c', (Tile::DeckPlate, [Object::loot_chest()]));
    let mut n = 0usize;
    p.for_each_assoc_object(|_, _, _| {
      n += 1;
    });

    assert_eq!(n, 1);
  }

  #[test]
  fn small_building_has_one_npc_at_expected_offset() {
    let mut found = None;
    Prefab::small_building_with_npc().for_each_assoc_object(|x, y, _| {
      found = Some((x, y));
    });

    assert_eq!(found, Some((2, 2)));
  }

  #[test]
  fn small_spaceship_has_console_and_pilot_offsets() {
    let mut origins = Vec::new();
    Prefab::small_spaceship().for_each_assoc_object(|x, y, _| {
      origins.push((x, y));
    });

    assert_eq!(origins.len(), 2);
    assert!(origins.contains(&(3, 2)));
    assert!(origins.contains(&(3, 3)));
  }
}
