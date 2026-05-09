use trl::{galaxy::{Location, LocationId},
          level::{LocationType, Tile, ZONE_WIDTH, ZONE_HEIGHT},
          prefabs::Prefab};

pub const ID: LocationId = (0, 0, 0);

pub fn generate() -> Location {
  let mut loc = Location::new(ZONE_WIDTH, ZONE_HEIGHT, 1, LocationType::PlanetSurface { breathable: true }, Tile::Vacuum);
  Prefab::starter_planet_surface().stamp_level(loc.level_mut(0), 0, 0);
  loc.level_mut(0).set(24, 29, Tile::ShipDock);
  loc
}

pub const NPC_COORDS: &[(i32, i32)] = &[
  (22, 25), // mira
  (20, 23), // chronos
  (26, 22), // unit7
  (22, 21), // kong
  (24, 23)  // guard
];
