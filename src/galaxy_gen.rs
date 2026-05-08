use crate::galaxy::Location;
use crate::level::{LocationType, Tile, fill_rect, place_room_with_door, Side};

/// Generate a small starter planet at the origin of the galaxy.
pub fn generate_starter_planet() -> Location {
    const W: usize = 48;
    const H: usize = 48;
    const D: usize = 1;

    let mut loc = Location::new(W, H, D, LocationType::PlanetSurface { breathable: true }, Tile::AlienGrass);

    // Small outpost building
    place_room_with_door(loc.level_mut(0), 18, 20, 12, 8, Side::South, 6, Tile::StationWall);
    // Change interior to StationFloor
    for y in 21..27 {
        for x in 19..29 {
            loc.level_mut(0).set(x, y, Tile::StationFloor);
        }
    }

    // Landing spot just south of the building
    fill_rect(loc.level_mut(0), 18, 28, 12, 4, Tile::Road);

    // Scatter some crystals and alien fluid for flavor
    let feature_spots: &[(i32, i32, Tile)] = &[
        (5, 5, Tile::CrystalGrowth),
        (8, 12, Tile::CrystalGrowth),
        (40, 8, Tile::CrystalGrowth),
        (38, 30, Tile::CrystalGrowth),
        (10, 35, Tile::AlienFluid),
        (11, 35, Tile::AlienFluid),
        (10, 36, Tile::AlienFluid),
    ];

    for &(x, y, tile) in feature_spots {
        loc.level_mut(0).set(x, y, tile);
    }

    // Add landing spot after mutations are done
    loc.landing_spots.push(crate::galaxy::LandingSpot {
        x: 24,
        y: 29,
        z: 0,
    });

    loc
}

/// Tile coordinates for NPCs on the starter planet surface (destination-local coords).
pub const STARTER_NPC_COORDS: &[(i32, i32)] = &[
    (22, 25),  // mira
    (20, 23),  // chronos
    (26, 22),  // unit7
    (22, 21),  // kong
    (24, 23),  // guard
];
