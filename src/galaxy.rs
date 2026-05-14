use {crate::{entities::Object, level::{Level, LocationType, Tile}, prefabs::Prefab},
     bevy::prelude::*,
     std::collections::HashMap};

/// Unique identifier for a Location in the Galaxy.
pub type LocationId = (i32, i32, i32);

/// One variable-size zone in the galaxy.
#[derive(Clone)]
pub struct Location {
  pub name: &'static str,
  pub width: usize,
  pub height: usize,
  pub depth: usize,
  pub levels: Vec<Level>,
  pub location_type: LocationType,
  /// Objects to spawn when this location is loaded.
  /// Each entry: (local_x, local_y, z, object) — world offset applied at spawn time.
  pub spawn_objects: Vec<(i32, i32, usize, Object)>,
}

impl Location {
  pub fn new(
    name: &'static str,
    width: usize,
    height: usize,
    depth: usize,
    location_type: LocationType,
    fill: crate::level::Tile
  ) -> Self {
    let levels = (0..depth).map(|_| Level::new(width, height, fill)).collect();
    Location { name, width, height, depth, levels, location_type, spawn_objects: vec![] }
  }

  /// Build a `Location` sized to `prefab`'s layout and stamp level 0 with it.
  pub fn from_prefab(
    name: &'static str,
    prefab: Prefab,
    location_type: LocationType,
    fill: Tile
  ) -> Self {
    let (w, h) = prefab.dimensions();
    let mut loc = Location::new(name, w, h, 1, location_type, fill);
    prefab.stamp_level(loc.level_mut(0), 0, 0);
    loc
  }

  pub fn level(&self, z: usize) -> &Level { &self.levels[z] }

  pub fn level_mut(&mut self, z: usize) -> &mut Level { &mut self.levels[z] }
}

/// The sparse galaxy map. Locations are lazily generated when first visited.
#[derive(Clone, Resource)]
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
