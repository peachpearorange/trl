use bevy::prelude::*;
use crate::galaxy::LocationId;
use crate::level::LocationType;
use crate::prefabs::Prefab;

pub const SHIP_WIDTH: usize = 20;
pub const SHIP_HEIGHT: usize = 15;
pub const SHIP_DEPTH: usize = 1;

/// The position of the airlock within the ship's local tile grid.
pub const AIRLOCK_X: i32 = 10;
pub const AIRLOCK_Y: i32 = 14; // south wall, bottom row

/// The position of the flight console within the ship.
pub const CONSOLE_X: i32 = 10;
pub const CONSOLE_Y: i32 = 2;

/// Ship state resource.
#[derive(Clone, Debug, Resource)]
pub struct Ship {
    pub location_id: LocationId,
    pub docked_at: Option<LocationId>,
    pub fuel: u32,
    pub max_fuel: u32,
}

impl Ship {
    pub fn new(location_id: LocationId) -> Self {
        Ship {
            location_id,
            docked_at: None,
            fuel: 500,
            max_fuel: 500,
        }
    }
}

/// Build the ship interior as a Location.
pub fn build_ship_interior() -> crate::galaxy::Location {
    use crate::galaxy::Location;
    use crate::level::Tile;

    let mut loc = Location::new(
        SHIP_WIDTH,
        SHIP_HEIGHT,
        SHIP_DEPTH,
        LocationType::ShipInterior,
        Tile::Vacuum,
    );

    let deck = loc.level_mut(0);
    Prefab::starting_ship().stamp_level(deck, 0, 0);
    loc
}
