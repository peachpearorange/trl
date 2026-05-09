use trl::{galaxy::{Location, LocationId},
          level::{LocationType, Tile, ZONE_WIDTH, ZONE_HEIGHT},
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
  let mut loc = Location::new(
    ZONE_WIDTH, ZONE_HEIGHT, 1,
    LocationType::PlanetSurface { breathable: false },
    Tile::Ash
  );
  lava_prefab().stamp_level(loc.level_mut(0), 0, 0);
  loc
}
