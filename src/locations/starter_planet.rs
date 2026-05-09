use crate::{galaxy::{Location, LocationId},
          level::{LocationType, Tile},
          prefabs::Prefab};

pub const ID: LocationId = (0, 0, 0);

pub fn generate() -> Location {
  Location::from_prefab(
    Prefab::starter_planet_surface(),
    LocationType::PlanetSurface { breathable: true },
    Tile::AlienGrass
  )
}

pub const NPC_COORDS: &[(i32, i32)] = &[
  (22, 25), // mira
  (20, 23), // chronos
  (26, 22), // unit7
  (22, 21), // kong
  (24, 23)  // guard
];
