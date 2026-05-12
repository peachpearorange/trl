use {crate::{galaxy::LocationId, level::LocationType, prefabs::Prefab},
     bevy::prelude::*};

pub const SHIP_WIDTH: usize = 32;
pub const SHIP_HEIGHT: usize = 16;
pub const SHIP_DEPTH: usize = 1;

/// Display name for logs and UI.
pub const SHIP_NAME: &str = "Mongoose";

/// The position of the airlock within the ship's local tile grid.
pub const AIRLOCK_X: i32 = 16;
pub const AIRLOCK_Y: i32 = 15;

/// The position of the flight console within the ship.
pub const CONSOLE_X: i32 = 29;
pub const CONSOLE_Y: i32 = 7;

/// The position of the loadout console within the ship.
pub const LOADOUT_X: i32 = 25;
pub const LOADOUT_Y: i32 = 8;

/// Ship state resource.
#[derive(Clone, Debug, Resource)]
pub struct Ship {
  pub location_id: LocationId,
  pub docked_at: Option<LocationId>,
  pub fuel: u32,
  pub max_fuel: u32
}

impl Ship {
  pub fn new(location_id: LocationId) -> Self {
    Ship { location_id, docked_at: None, fuel: 500, max_fuel: 500 }
  }
}

/// Build the ship interior as a Location.
pub fn build_ship_interior() -> crate::galaxy::Location {
  use crate::{galaxy::Location, level::Tile};
  Location::from_prefab("Ship Interior", Prefab::starting_ship(), LocationType::ShipInterior, Tile::Blank)
}
