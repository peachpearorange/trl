use crate::galaxy::{LandingSpot, Location, LocationId};
use crate::level::{LocationType, Tile};
use crate::prefabs::Prefab;

pub const ID_STARTER_PLANET: LocationId = (0, 0, 0);
pub const ID_ASTEROID_FIELD: LocationId = (1, 0, 0);

/// Generate a small starter planet at the origin of the galaxy.
pub fn generate_starter_planet() -> Location {
    const W: usize = crate::level::ZONE_WIDTH;
    const H: usize = crate::level::ZONE_HEIGHT;
    const D: usize = 1;

    let mut loc = Location::new(W, H, D, LocationType::PlanetSurface { breathable: true }, Tile::Vacuum);
    Prefab::starter_planet_surface().stamp_level(loc.level_mut(0), 0, 0);

    loc.landing_spots.push(LandingSpot {
        x: 24,
        y: 29,
        z: 0,
    });

    loc
}

/// Sparse rocky belt — vacuum with clustered rocks and a marked landing pad.
pub fn generate_asteroid_field() -> Location {
    const W: usize = crate::level::ZONE_WIDTH;
    const H: usize = crate::level::ZONE_HEIGHT;
    let mut loc = Location::new(W, H, 1, LocationType::AsteroidField, Tile::Vacuum);
    let level = loc.level_mut(0);
    let cx = W as f32 * 0.5;
    let cy = H as f32 * 0.5;
    for y in 0..H {
        for x in 0..W {
            let dx = x as f32 - cx;
            let dy = y as f32 - cy;
            let d = (dx * dx + dy * dy).sqrt();
            let t = if d < 6.5 {
                Tile::AsteroidFloor
            } else if d < 14.0 {
                if (x + y * 3) % 5 == 0 {
                    Tile::AsteroidRock
                } else {
                    Tile::AsteroidFloor
                }
            } else if d < 20.0 {
                Tile::AsteroidRock
            } else {
                Tile::Vacuum
            };
            level.set(x as i32, y as i32, t);
        }
    }
    let lx = (cx as i32).clamp(6, W as i32 - 7);
    let ly = (cy as i32).clamp(6, H as i32 - 7);
    for yy in (ly - 1).max(0)..=(ly + 1).min(H as i32 - 1) {
        for xx in (lx - 1).max(0)..=(lx + 1).min(W as i32 - 1) {
            level.set(xx, yy, Tile::Regolith);
        }
    }
    level.set(lx, ly, Tile::Regolith);
    loc.landing_spots.push(LandingSpot {
        x: lx,
        y: ly,
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
