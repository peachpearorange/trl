use bevy::prelude::*;
use crate::galaxy::LocationId;
use crate::level::{LocationType, Tile};

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

    let mut loc = Location::new(
        SHIP_WIDTH,
        SHIP_HEIGHT,
        SHIP_DEPTH,
        LocationType::ShipInterior,
        Tile::Vacuum,
    );

    let deck = loc.level_mut(0);

    // Fill interior with deck plates, surrounded by bulkhead walls
    for y in 0..SHIP_HEIGHT as i32 {
        for x in 0..SHIP_WIDTH as i32 {
            let is_edge = x == 0 || x == SHIP_WIDTH as i32 - 1
                || y == 0 || y == SHIP_HEIGHT as i32 - 1;
            deck.set(x, y, if is_edge { Tile::Bulkhead } else { Tile::DeckPlate });
        }
    }

    // Airlock door (south wall)
    deck.set(AIRLOCK_X, AIRLOCK_Y, Tile::AirlockDoor);

    // Windows along north and side walls
    for x in 3..17 {
        deck.set(x, 0, Tile::Window);
    }
    for y in 3..12 {
        deck.set(0, y, Tile::Window);
        deck.set(SHIP_WIDTH as i32 - 1, y, Tile::Window);
    }

    // Interior walls for rooms
    // Crew quarters (left side)
    for y in 4..7 {
        deck.set(6, y, Tile::Bulkhead);
    }
    deck.set(6, 4, Tile::AirlockDoor); // crew quarters door

    // Engineering (right side)
    for y in 8..11 {
        deck.set(13, y, Tile::Bulkhead);
    }
    deck.set(13, 9, Tile::AirlockDoor); // engineering door

    // Conduits in engineering
    deck.set(16, 10, Tile::Conduit);
    deck.set(17, 10, Tile::Conduit);
    deck.set(16, 9, Tile::Conduit);

    loc
}
