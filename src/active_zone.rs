use crate::galaxy::Location;
use crate::level::Level;
use crate::ship::{AIRLOCK_X, AIRLOCK_Y};

/// The currently rendered tile grid. Holds the merged tiles of the ship
/// and (when docked) the destination location.
#[derive(Clone, Debug)]
pub struct ActiveZone {
    pub levels: Vec<Level>,
    pub width: usize,
    pub height: usize,
    pub depth: usize,
    /// Offset within the active zone where the ship's (0,0) tile maps.
    pub ship_origin: (i32, i32),
    /// If docked, offset where the destination's (0,0) tile maps.
    pub dest_origin: Option<(i32, i32)>,
    /// The destination's dimensions when docked.
    pub dest_dims: Option<(usize, usize, usize)>,
}

impl ActiveZone {
    /// Create an ActiveZone containing only the ship (deep space mode).
    pub fn ship_only(ship_loc: &Location) -> Self {
        let w = ship_loc.width;
        let h = ship_loc.height;
        ActiveZone {
            levels: ship_loc.levels.clone(),
            width: w,
            height: h,
            depth: ship_loc.depth,
            ship_origin: (0, 0),
            dest_origin: None,
            dest_dims: None,
        }
    }

    /// Create an ActiveZone with ship docked to a destination at a landing spot.
    /// Position the destination so its landing spot is adjacent to the ship's airlock.
    pub fn docked(
        ship_loc: &Location,
        dest: &Location,
        landing_spot_idx: usize,
    ) -> Option<Self> {
        let spot = dest.landing_spots.get(landing_spot_idx)?;

        // Ship airlock is at (AIRLOCK_X, AIRLOCK_Y) in ship-local coords.
        // Position the destination so its landing spot is adjacent (south) of the airlock.
        let dest_origin = (
            AIRLOCK_X - spot.x,
            AIRLOCK_Y + 1 - spot.y,
        );

        // Compute bounding box
        let ship_x0 = 0i32;
        let ship_y0 = 0i32;
        let ship_x1 = ship_loc.width as i32;
        let ship_y1 = ship_loc.height as i32;

        let dest_x0 = dest_origin.0;
        let dest_y0 = dest_origin.1;
        let dest_x1 = dest_origin.0 + dest.width as i32;
        let dest_y1 = dest_origin.1 + dest.height as i32;

        let min_x = ship_x0.min(dest_x0);
        let min_y = ship_y0.min(dest_y0);
        let max_x = ship_x1.max(dest_x1);
        let max_y = ship_y1.max(dest_y1);

        let total_w = (max_x - min_x) as usize;
        let total_h = (max_y - min_y) as usize;
        let total_d = ship_loc.depth.max(dest.depth);

        // Shift ship_origin so it's positive in the merged grid
        let ship_origin = (
            ship_x0 - min_x,
            ship_y0 - min_y,
        );
        let dest_origin_shifted = (
            dest_x0 - min_x,
            dest_y0 - min_y,
        );

        let mut levels: Vec<Level> = (0..total_d)
            .map(|_| Level::new(total_w, total_h, crate::level::Tile::Vacuum))
            .collect();

        // Copy ship tiles
        for y in 0..ship_loc.height {
            for x in 0..ship_loc.width {
                for z in 0..ship_loc.depth {
                    let tile = ship_loc.levels[z].tiles[y][x];
                    let lx = ship_origin.0 + x as i32;
                    let ly = ship_origin.1 + y as i32;
                    if lx >= 0 && ly >= 0
                        && (lx as usize) < total_w
                        && (ly as usize) < total_h
                    {
                        levels[z].set(lx, ly, tile);
                    }
                    if let Some(item) = ship_loc.levels[z].items[y][x] {
                        let lx = ship_origin.0 + x as i32;
                        let ly = ship_origin.1 + y as i32;
                        if lx >= 0 && ly >= 0
                            && (lx as usize) < total_w
                            && (ly as usize) < total_h
                        {
                            levels[z].set_item(lx, ly, Some(item));
                        }
                    }
                }
            }
        }

        // Copy destination tiles
        for y in 0..dest.height {
            for x in 0..dest.width {
                for z in 0..dest.depth {
                    let tile = dest.levels[z].tiles[y][x];
                    let lx = dest_origin_shifted.0 + x as i32;
                    let ly = dest_origin_shifted.1 + y as i32;
                    if lx >= 0 && ly >= 0
                        && (lx as usize) < total_w
                        && (ly as usize) < total_h
                    {
                        levels[z].set(lx, ly, tile);
                    }
                    if let Some(item) = dest.levels[z].items[y][x] {
                        let lx = dest_origin_shifted.0 + x as i32;
                        let ly = dest_origin_shifted.1 + y as i32;
                        if lx >= 0 && ly >= 0
                            && (lx as usize) < total_w
                            && (ly as usize) < total_h
                        {
                            levels[z].set_item(lx, ly, Some(item));
                        }
                    }
                }
            }
        }

        Some(ActiveZone {
            levels,
            width: total_w,
            height: total_h,
            depth: total_d,
            ship_origin,
            dest_origin: Some(dest_origin_shifted),
            dest_dims: Some((dest.width, dest.height, dest.depth)),
        })
    }

    pub fn level(&self, z: usize) -> &Level {
        &self.levels[z]
    }

    pub fn level_mut(&mut self, z: usize) -> &mut Level {
        &mut self.levels[z]
    }
}
