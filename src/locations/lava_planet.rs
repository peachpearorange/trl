use crate::{galaxy::{Location, LocationId},
          level::{LocationType, Tile},
          prefabs::{prefab, Prefab}};

pub const ID: LocationId = (2, 0, 0);

pub fn lava_prefab() -> Prefab {
  prefab(include_str!("../../assets/prefabs/lava_planet.txt"))
    .assoc('a', (Tile::Ash, []))
    .assoc('l', (Tile::Lava, []))
    .assoc('r', (Tile::CaveWall, []))
    .assoc('c', (Tile::CaveFloor, []))
    .assoc('P', (Tile::ShipDock, []))
}

pub fn generate() -> Location {
  Location::from_prefab(
    lava_prefab(),
    LocationType::PlanetSurface { breathable: false },
    Tile::Ash
  )
}
