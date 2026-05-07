use crate::{entities::{Glyph, Named, Object, Stats},
            level::{Level, Tile}};
use bevy::prelude::Color;

pub type ObjectFactory = fn() -> Object;

struct CharAssoc {
  ch: char,
  tile: Tile,
  factories: Vec<ObjectFactory>
}

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

pub struct PrefabArea {
  assocs: Vec<CharAssoc>,
  default_tile: Tile
}

impl PrefabArea {
  pub fn new() -> Self { PrefabArea { assocs: Vec::new(), default_tile: Tile::Vacuum } }

  pub fn filled_with(mut self, tile: Tile) -> Self {
    self.default_tile = tile;
    self
  }

  pub fn assoc(mut self, ch: char, tile: Tile, factories: &[ObjectFactory]) -> Self {
    self.assocs.push(CharAssoc { ch, tile, factories: factories.to_vec() });
    self
  }

  pub fn tile(mut self, ch: char, tile: Tile) -> Self {
    self.assocs.push(CharAssoc { ch, tile, factories: Vec::new() });
    self
  }

  pub fn build(&self, layout: &str) -> (Level, Vec<PrefabObject>) {
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

    let mut level = Level::new(width, height, self.default_tile);
    let mut spawns: Vec<PrefabObject> = Vec::new();

    for (y, line) in lines.iter().enumerate() {
      for (x, ch) in line.chars().enumerate() {
        if let Some(assoc) = self.assocs.iter().find(|a| a.ch == ch) {
          level.set(x as i32, y as i32, assoc.tile);
          for factory in &assoc.factories {
            spawns.push(PrefabObject { object: factory(), x: x as i32, y: y as i32 });
          }
        }
      }
    }

    (level, spawns)
  }
}

impl Default for PrefabArea {
  fn default() -> Self { Self::new() }
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

pub fn small_building_with_npc() -> (Level, Vec<PrefabObject>) {
  crate::prefab_area! {
      w = Tile::StationWall,
      f = Tile::StationFloor,
      d = Tile::Door,
      n = Tile::StationFloor with resident,
  }
  .filled_with(Tile::Vacuum)
  .build(
    "
    wwwww
    wfffw
    wfnfw
    wwdfw
    wwwww
    "
  )
}

#[macro_export]
macro_rules! prefab_area {
    (
        $(
            $ch:ident = $tile_root:ident $(:: $tile_tail:ident)* $(with $first_object_root:ident $(:: $first_object_tail:ident)* $(and $object_root:ident $(:: $object_tail:ident)*)*)?
        ),+ $(,)?
    ) => {{
        $crate::prefabs::PrefabArea::new()
            $(
                .assoc(
                    stringify!($ch).chars().next().expect("prefab char must not be empty"),
                    $tile_root $(:: $tile_tail)*,
                    &[$($first_object_root $(:: $first_object_tail)* $(, $object_root $(:: $object_tail)*)*)?],
                )
            )+
    }};
}

#[cfg(test)]
mod tests {
  use super::*;

  fn chest() -> Object { Object::loot_chest() }

  fn enemy() -> Object { Object::rat_soldier() }

  #[test]
  fn builds_tiles_and_multiple_objects_from_layout() {
    let area = prefab_area! {
        k = Tile::Floor with chest and enemy,
        w = Tile::Wall,
    };

    let (level, spawns) = area.build(
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
    let (level, spawns) =
      PrefabArea::new().filled_with(Tile::Vacuum).tile('.', Tile::DeckPlate).build(".x");

    assert_eq!(level.get(0, 0), Some(Tile::DeckPlate));
    assert_eq!(level.get(1, 0), Some(Tile::Vacuum));
    assert!(spawns.is_empty());
  }

  #[test]
  fn macro_accepts_associated_object_constructors() {
    let area = prefab_area! {
        c = Tile::DeckPlate with Object::loot_chest,
    };

    let (_, spawns) = area.build("c");

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
}
