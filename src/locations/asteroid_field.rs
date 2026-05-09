use crate::{galaxy::{Location, LocationId},
          level::{LocationType, Tile, ZONE_WIDTH, ZONE_HEIGHT}};

pub const ID: LocationId = (1, 0, 0);

pub fn generate() -> Location {
  const W: usize = ZONE_WIDTH;
  const H: usize = ZONE_HEIGHT;
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
        if (x + y * 3) % 5 == 0 { Tile::AsteroidRock } else { Tile::AsteroidFloor }
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
  level.set(lx, ly, Tile::ShipDock);
  loc
}
