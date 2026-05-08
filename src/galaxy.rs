use {crate::level::{Level, LocationType},
     bevy::prelude::*,
     std::collections::HashMap};

/// Unique identifier for a Location in the Galaxy.
pub type LocationId = (i32, i32, i32);

/// Where the ship connects when docking at this location.
#[derive(Clone, Debug)]
pub struct LandingSpot {
  pub x: i32,
  pub y: i32,
  pub z: usize
}

/// One variable-size zone in the galaxy.
#[derive(Clone, Debug)]
pub struct Location {
  pub width: usize,
  pub height: usize,
  pub depth: usize,
  pub levels: Vec<Level>,
  pub location_type: LocationType,
  pub landing_spots: Vec<LandingSpot>
}

impl Location {
  pub fn new(
    width: usize,
    height: usize,
    depth: usize,
    location_type: LocationType,
    fill: crate::level::Tile
  ) -> Self {
    let levels = (0..depth).map(|_| Level::new(width, height, fill)).collect();
    Location { width, height, depth, levels, location_type, landing_spots: Vec::new() }
  }

  pub fn level(&self, z: usize) -> &Level { &self.levels[z] }

  pub fn level_mut(&mut self, z: usize) -> &mut Level { &mut self.levels[z] }
}

/// The sparse galaxy map. Locations are lazily generated when first visited.
#[derive(Clone, Debug, Resource)]
pub struct Galaxy {
  pub locations: HashMap<LocationId, Location>
}

impl Galaxy {
  pub fn new() -> Self { Galaxy { locations: HashMap::new() } }

  pub fn get(&self, id: LocationId) -> Option<&Location> { self.locations.get(&id) }

  pub fn get_mut(&mut self, id: LocationId) -> Option<&mut Location> {
    self.locations.get_mut(&id)
  }

  pub fn insert(&mut self, id: LocationId, location: Location) {
    self.locations.insert(id, location);
  }

  /// Euclidean distance between two location coordinates.
  pub fn distance(a: LocationId, b: LocationId) -> f64 {
    let dx = (a.0 - b.0) as f64;
    let dy = (a.1 - b.1) as f64;
    let dz = (a.2 - b.2) as f64;
    (dx * dx + dy * dy + dz * dz).sqrt()
  }
}
