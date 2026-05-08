use crate::galaxy::Location;
use crate::level::{LocationType, Tile};
use crate::prefabs::Prefab;

/// Generate a small starter planet at the origin of the galaxy.
pub fn generate_starter_planet() -> Location {
    const W: usize = crate::level::ZONE_WIDTH;
    const H: usize = crate::level::ZONE_HEIGHT;
    const D: usize = 1;

    let mut loc = Location::new(W, H, D, LocationType::PlanetSurface { breathable: true }, Tile::Vacuum);
    Prefab::starter_planet_surface().stamp_level(loc.level_mut(0), 0, 0);

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
