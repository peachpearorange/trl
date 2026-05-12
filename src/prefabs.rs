use std::collections::HashMap;

use {crate::{entities::{Glyph, Named, Object, Stats},
             level::{Level, Tile},
             npcs},
     bevy::prelude::{Color, Commands}};

pub trait AssocMarker {
  fn assoc_char(self) -> char;
}

impl AssocMarker for char {
  fn assoc_char(self) -> char { self }
}

impl AssocMarker for &str {
  fn assoc_char(self) -> char {
    let mut it = self.chars();
    let c = it.next().expect("prefab assoc key must not be empty");
    assert!(it.next().is_none(), "prefab assoc key must be one character");
    c
  }
}

/// ASCII layout plus `.assoc(…, (Tile, […]))` chains. Apply with [`Prefab::stamp_level`] /
/// [`Prefab::stamp_entities`].
pub struct Prefab {
  layout: String,
  assocs: HashMap<char, (Tile, Vec<Object>)>
}

fn resident() -> Object {
  Object::npc().add((
    Named { name: "Resident", flavor: "Someone trying to keep a small place livable." },
    Stats { hp: 8, max_hp: 8, attack: 1, move_speed: 3.0, attack_speed: 1.0 },
    Glyph::ascii('@', Color::srgb(0.7, 0.9, 1.0))
  ))
}

fn ship_pilot() -> Object {
  Object::npc().add((
    Named {
      name: "Pilot",
      flavor: "Ticks through a short pre-flight list. Coffee stains on the console manual."
    },
    Stats { hp: 10, max_hp: 10, attack: 1, move_speed: 3.0, attack_speed: 1.0 },
    Glyph::ascii('@', Color::srgb(0.55, 0.82, 0.95))
  ))
}

pub fn prefab(layout: impl Into<String>) -> Prefab { Prefab::new(layout) }

impl Prefab {
  fn visit_cells<F: FnMut(i32, i32, Tile, &[Object])>(&self, mut f: F) {
    let lines = {
      let raw_lines =
        self.layout.lines().filter(|l| !l.trim().is_empty()).collect::<Vec<_>>();
      let indent = raw_lines
        .iter()
        .filter_map(|line| {
          line.char_indices().find(|(_, ch)| !ch.is_whitespace()).map(|(i, _)| i)
        })
        .min()
        .unwrap_or(0);
      raw_lines
        .into_iter()
        .map(|line| line.get(indent..).unwrap_or(line))
        .collect::<Vec<_>>()
    };
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

  /// `(width, height)` of the bounding box of the layout (max col+1, row count).
  pub fn dimensions(&self) -> (usize, usize) {
    let raw_lines =
      self.layout.lines().filter(|l| !l.trim().is_empty()).collect::<Vec<_>>();
    let indent = raw_lines
      .iter()
      .filter_map(|line| {
        line.char_indices().find(|(_, ch)| !ch.is_whitespace()).map(|(i, _)| i)
      })
      .min()
      .unwrap_or(0);
    let h = raw_lines.len();
    let w = raw_lines
      .iter()
      .map(|line| line.get(indent..).unwrap_or(line).chars().count())
      .max()
      .unwrap_or(0);
    (w, h)
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

  pub fn new(layout: impl Into<String>) -> Self {
    Self { layout: layout.into(), assocs: HashMap::new() }
  }

  /// `(tile, object templates …)` per layout character. Each [`Object`] is [`Clone`]d per grid cell.
  pub fn assoc<const N: usize>(
    mut self,
    marker: impl AssocMarker,
    (tile, templates): (Tile, [Object; N])
  ) -> Self {
    self.assocs.insert(marker.assoc_char(), (tile, Vec::from(templates)));
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
    "
    )
    .assoc('w', (Tile::StationWall, []))
    .assoc('f', (Tile::StationFloor, []))
    .assoc('d', (Tile::Door, [Object::door()]))
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
    "
    )
    .assoc('b', (Tile::Bulkhead, []))
    .assoc('w', (Tile::Window, []))
    .assoc('.', (Tile::DeckPlate, []))
    .assoc('c', (Tile::DeckPlate, [Object::flight_console()]))
    .assoc('a', (Tile::AirlockDoor, [Object::airlock_door()]))
    .assoc('p', (Tile::DeckPlate, [ship_pilot()]))
  }

  /// Full starter ship deck (`SHIP_WIDTH` × `SHIP_HEIGHT`).
  pub fn starting_ship() -> Self {
    prefab(
      "
                 ###WWWWWWWWWWW###
               ###...............###
             ###...######.######...###
            ##.....#B#,,,,,,L#B#.....##
           ##.====.#,l,,,,,,,l,#......##
          ##..====.#,#,,,k,,,#,#.......WW
         ##.U.====.###,,,TT,,###........W
         ##..........#,,,TT,,#......H.C.W
         ##.G.====.###,,,TT,,###...Q....W
          ##..====.#,m,,,,,,,X,#.......WW
           ##.d==..#,,,,,,,,,,,#......##
            ##.....######.######.....##
             ###...................###
               ###WWWWW#...#WWWWW###
                       #...#
                       ##a##
"
    )
    .assoc('#', (Tile::Bulkhead, []))
    .assoc('.', (Tile::DeckPlate, []))
    .assoc(',', (Tile::WoodTile, []))
    .assoc('W', (Tile::Window, []))
    .assoc('a', (Tile::AirlockDoor, [Object::airlock_door()]))
    .assoc('l', (Tile::DeckPlate, [Object::airlock_door()]))
    .assoc('=', (Tile::Conduit, []))
    .assoc('C', (Tile::DeckPlate, [Object::flight_console()]))
    .assoc('Q', (Tile::DeckPlate, [Object::loadout_console()]))
    .assoc('k', (Tile::WoodTile, [Object::space_cat()]))
    .assoc('B', (Tile::WoodTile, [Object::bed()]))
    .assoc('T', (Tile::WoodTile, [Object::table()]))
    .assoc('L', (Tile::WoodTile, [Object::locker()]))
    .assoc('X', (Tile::WoodTile, [Object::crate_obj()]))
    .assoc('m', (Tile::WoodTile, [npcs::mira::mira()]))
    .assoc('H', (Tile::DeckPlate, [npcs::chronos::chronos()]))
    .assoc('U', (Tile::DeckPlate, [npcs::unit7::unit7()]))
    .assoc('G', (Tile::DeckPlate, [npcs::kong::kong()]))
    .assoc('d', (Tile::DeckPlate, [npcs::guard::guard()]))
  }
}

#[cfg(test)]
mod tests {
  use {super::*,
       crate::level::{Level, Tile}};

  fn chest() -> Object { Object::loot_chest() }

  fn enemy() -> Object { Object::rat_soldier() }

  #[test]
  fn builds_multiple_objects_at_same_cell() {
    let p = prefab(
      "
            www
            wkw
            www
            "
    )
    .assoc('k', (Tile::DeckPlate, [chest(), enemy()]))
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
"
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
  fn small_building_has_npc_and_door_at_expected_offsets() {
    let mut positions = Vec::new();
    Prefab::small_building_with_npc().for_each_assoc_object(|x, y, _| {
      positions.push((x, y));
    });

    // npc (n) at (2,2), door (d) at (2,3)
    assert!(positions.contains(&(2, 2)));
    assert!(positions.contains(&(2, 3)));
  }

  #[test]
  fn small_spaceship_has_console_and_pilot_offsets() {
    let mut origins = Vec::new();
    Prefab::small_spaceship().for_each_assoc_object(|x, y, _| {
      origins.push((x, y));
    });

    // console (c) at (3,3), pilot (p) at (3,2), airlock door (a) at (3,5)
    assert_eq!(origins.len(), 3);
    assert!(origins.contains(&(3, 2)));
    assert!(origins.contains(&(3, 3)));
  }

  #[test]
  fn starting_ship_structure_tiles() {
    let p = Prefab::starting_ship();
    let (w, h) = p.dimensions();
    let mut stamped = Level::new(w, h, Tile::Vacuum);
    p.stamp_level(&mut stamped, 0, 0);
    // airlock at bottom centre
    assert_eq!(stamped.get(16, 15), Some(Tile::AirlockDoor));
    // console
    assert_eq!(stamped.get(29, 7),  Some(Tile::DeckPlate));
    // conduit column (cols 5-8 of rows 6-8)
    assert_eq!(stamped.get(5, 6),   Some(Tile::Conduit));
    assert_eq!(stamped.get(5, 8),   Some(Tile::Conduit));
    // bulkhead outer hull
    assert_eq!(stamped.get(0, 6),   Some(Tile::Bulkhead));
    assert_eq!(stamped.get(0, 8),   Some(Tile::Bulkhead));
  }
}
