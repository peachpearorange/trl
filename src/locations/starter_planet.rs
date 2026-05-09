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

