use {crate::level::Level, bevy::prelude::*};

/// Marker: entity is protected from vacuum damage.
#[derive(Component)]
pub struct EVASuit;

/// Apply 1 vacuum damage per sim tick to entities on tiles without atmosphere.
pub fn vacuum_damage<'a>(
  level: &Level,
  positions: impl Iterator<Item = ((i32, i32, usize), &'a mut i32)>
) {
  for ((x, y, _z), hp) in positions {
    if y >= 0 && (y as usize) < level.height && x >= 0 && (x as usize) < level.width {
      let tile = level.tiles[y as usize][x as usize];
      if !tile.has_atmosphere() {
        *hp -= 1;
      }
    }
  }
}
