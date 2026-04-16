#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tile {
  Air,
  Floor,
  Wall,
  Grass,
  Water,
  Sand,
  StairsUp,
  StairsDown,
  Door
}

impl Tile {
  pub fn glyph(self) -> &'static str {
    match self {
      Tile::Air => " ",
      Tile::Floor => ".",
      Tile::Wall => "#",
      Tile::Grass => "\"",
      Tile::Water => "~",
      Tile::Sand => ",",
      Tile::StairsUp => "<",
      Tile::StairsDown => ">",
      Tile::Door => "+"
    }
  }

  pub fn color(self) -> [f32; 3] {
    match self {
      Tile::Air => [0.0, 0.0, 0.0],
      Tile::Floor => [0.6, 0.5, 0.3],
      Tile::Wall => [0.4, 0.4, 0.4],
      Tile::Grass => [0.2, 0.6, 0.2],
      Tile::Water => [0.2, 0.3, 0.8],
      Tile::Sand => [0.8, 0.7, 0.4],
      Tile::StairsUp => [0.9, 0.9, 0.2],
      Tile::StairsDown => [0.9, 0.9, 0.2],
      Tile::Door => [0.6, 0.3, 0.1]
    }
  }

  pub fn walkable(self) -> bool {
    matches!(
      self,
      Tile::Floor
        | Tile::Grass
        | Tile::Sand
        | Tile::StairsUp
        | Tile::StairsDown
        | Tile::Door
    )
  }
}

impl Tile {
  pub fn opaque(self) -> bool { matches!(self, Tile::Wall | Tile::Door) }

  pub fn name(self) -> &'static str {
    match self {
      Tile::Air => "Air",
      Tile::Floor => "Floor",
      Tile::Wall => "Wall",
      Tile::Grass => "Grass",
      Tile::Water => "Water",
      Tile::Sand => "Sand",
      Tile::StairsUp => "Stairs Up",
      Tile::StairsDown => "Stairs Down",
      Tile::Door => "Door"
    }
  }
}

pub struct Level {
  pub tiles: Vec<Vec<Tile>>,
  pub width: usize,
  pub height: usize
}

impl Level {
  pub fn new(width: usize, height: usize, fill: Tile) -> Self {
    Level { tiles: vec![vec![fill; width]; height], width, height }
  }

  pub fn get(&self, x: i32, y: i32) -> Option<Tile> {
    if x < 0 || y < 0 {
      return None;
    }
    let (ux, uy) = (x as usize, y as usize);
    (ux < self.width && uy < self.height).then(|| self.tiles[uy][ux])
  }

  pub fn set(&mut self, x: i32, y: i32, tile: Tile) {
    if x >= 0 && y >= 0 && (x as usize) < self.width && (y as usize) < self.height {
      self.tiles[y as usize][x as usize] = tile;
    }
  }

  pub fn walkable(&self, x: i32, y: i32) -> bool {
    self.get(x, y).is_some_and(|t| t.walkable())
  }
}

// ---------------------------------------------------------------------------
// Builder utilities — usable by hand-crafted levels and procgen alike
// ---------------------------------------------------------------------------

/// Fill a rectangular region with a tile.
pub fn fill_rect(level: &mut Level, x: i32, y: i32, w: usize, h: usize, tile: Tile) {
  for dy in 0..h as i32 {
    for dx in 0..w as i32 {
      level.set(x + dx, y + dy, tile);
    }
  }
}

/// Place a room: wall border with floor interior.
pub fn place_room(level: &mut Level, x: i32, y: i32, w: usize, h: usize) {
  fill_rect(level, x, y, w, h, Tile::Wall);
  if w > 2 && h > 2 {
    fill_rect(level, x + 1, y + 1, w - 2, h - 2, Tile::Floor);
  }
}

/// Place a room with a door on a given side at a relative offset along that side.
pub fn place_room_with_door(
  level: &mut Level,
  x: i32,
  y: i32,
  w: usize,
  h: usize,
  door_side: Side,
  door_offset: usize
) {
  place_room(level, x, y, w, h);
  let (dx, dy) = match door_side {
    Side::North => (x + door_offset as i32, y),
    Side::South => (x + door_offset as i32, y + h as i32 - 1),
    Side::West => (x, y + door_offset as i32),
    Side::East => (x + w as i32 - 1, y + door_offset as i32)
  };
  level.set(dx, dy, Tile::Door);
}

#[derive(Clone, Copy)]
pub enum Side {
  North,
  South,
  East,
  West
}

/// Carve an L-shaped corridor between two points (horizontal first, then vertical).
pub fn place_corridor(level: &mut Level, x1: i32, y1: i32, x2: i32, y2: i32) {
  let (mut cx, cy1, cy2) = (x1, y1, y2);
  let dx = if x2 > x1 { 1 } else { -1 };
  while cx != x2 {
    level.set(cx, cy1, Tile::Floor);
    cx += dx;
  }
  let mut cy = cy1;
  let dy = if cy2 > cy1 { 1 } else { -1 };
  while cy != cy2 {
    level.set(x2, cy, Tile::Floor);
    cy += dy;
  }
  level.set(x2, cy2, Tile::Floor);
}

/// Place a pair of stairs connecting two levels at the same (x, y).
/// Caller is responsible for ensuring both levels exist.
pub fn place_stairs(levels: &mut [Level], z_from: usize, z_to: usize, x: i32, y: i32) {
  if z_to > z_from {
    levels[z_from].set(x, y, Tile::StairsDown);
    levels[z_to].set(x, y, Tile::StairsUp);
  } else {
    levels[z_from].set(x, y, Tile::StairsUp);
    levels[z_to].set(x, y, Tile::StairsDown);
  }
}

/// Carve an organic blob (rough circle) of floor tiles.
pub fn carve_blob(level: &mut Level, cx: i32, cy: i32, radius: i32, tile: Tile) {
  let r2 = radius * radius;
  for dy in -radius..=radius {
    for dx in -radius..=radius {
      let d2 = dx * dx + dy * dy;
      let fudge = ((dx.wrapping_mul(7) ^ dy.wrapping_mul(13)) & 3) as i32;
      if d2 <= r2 + fudge {
        level.set(cx + dx, cy + dy, tile);
      }
    }
  }
}

/// Ensure a square of walkable floor around a point (useful around stairs).
pub fn clear_around(level: &mut Level, x: i32, y: i32, radius: i32) {
  for dy in -radius..=radius {
    for dx in -radius..=radius {
      if level.get(x + dx, y + dy).is_some_and(|t| !t.walkable()) {
        level.set(x + dx, y + dy, Tile::Floor);
      }
    }
  }
}

/// Place a wide corridor (3 tiles across) between two points.
pub fn place_wide_corridor(level: &mut Level, x1: i32, y1: i32, x2: i32, y2: i32) {
  for offset in -1..=1 {
    // horizontal leg
    let (mut cx, cy) = (x1, y1);
    let dx = if x2 > x1 { 1 } else { -1 };
    while cx != x2 {
      level.set(cx, cy + offset, Tile::Floor);
      cx += dx;
    }
    // vertical leg
    let mut cy2 = y1;
    let dy = if y2 > y1 { 1 } else { -1 };
    while cy2 != y2 {
      level.set(x2 + offset, cy2, Tile::Floor);
      cy2 += dy;
    }
    level.set(x2 + offset, y2, Tile::Floor);
  }
}

// ---------------------------------------------------------------------------
// World: a stack of levels
// ---------------------------------------------------------------------------

pub struct World {
  pub levels: Vec<Level>,
  pub width: usize,
  pub height: usize
}

impl World {
  pub fn new(width: usize, height: usize, depth: usize, fill: Tile) -> Self {
    World {
      levels: (0..depth).map(|_| Level::new(width, height, fill)).collect(),
      width,
      height
    }
  }

  pub fn depth(&self) -> usize { self.levels.len() }

  pub fn level(&self, z: usize) -> &Level { &self.levels[z] }

  pub fn level_mut(&mut self, z: usize) -> &mut Level { &mut self.levels[z] }
}

// ---------------------------------------------------------------------------
// Test world
// ---------------------------------------------------------------------------

/// Build a hand-crafted test world:
///   z=0  deep cave
///   z=1  shallow cave / basement
///   z=2  surface (main level, building, cave entrance)
///   z=3  building upper floor
pub fn build_test_world() -> World {
  const W: usize = 80;
  const H: usize = 60;
  let mut world = World::new(W, H, 4, Tile::Air);

  // === z=2: surface ===
  {
    let s = world.level_mut(2);

    // grass ground across the whole level
    fill_rect(s, 0, 0, W, H, Tile::Grass);

    // dirt paths
    fill_rect(s, 10, 28, 50, 3, Tile::Sand);
    fill_rect(s, 35, 10, 3, 21, Tile::Sand);

    // building ground floor (12x10) at top-right area
    place_room_with_door(s, 30, 8, 12, 10, Side::South, 6);
    // interior detail
    fill_rect(s, 36, 9, 1, 5, Tile::Wall);

    // cave entrance (opening in the ground)
    fill_rect(s, 5, 35, 8, 6, Tile::Floor);

    // pond
    fill_rect(s, 50, 35, 7, 5, Tile::Water);

    // some trees (wall tiles on grass)
    for &(tx, ty) in &[(15, 15), (18, 12), (22, 16), (45, 20), (55, 14), (60, 22)] {
      s.set(tx, ty, Tile::Wall);
    }
  }

  // === z=3: building upper floor ===
  {
    let u = world.level_mut(3);
    place_room_with_door(u, 30, 8, 12, 10, Side::South, 6);
    fill_rect(u, 35, 9, 1, 4, Tile::Wall);
    u.set(35, 12, Tile::Door);
  }

  // stairs between building floors (z=2 <-> z=3)
  place_stairs(&mut world.levels, 2, 3, 40, 10);

  // === z=1: shallow cave ===
  {
    let c = world.level_mut(1);
    fill_rect(c, 0, 0, W, H, Tile::Wall);

    // large main cavern
    carve_blob(c, 30, 30, 14, Tile::Floor);
    // entrance cavern near stairs from surface
    carve_blob(c, 10, 38, 8, Tile::Floor);
    // side chamber
    carve_blob(c, 55, 20, 8, Tile::Floor);
    // connecting corridors (wide)
    place_wide_corridor(c, 18, 38, 30, 30);
    place_wide_corridor(c, 44, 30, 55, 20);

    // underground pool
    carve_blob(c, 25, 22, 4, Tile::Water);

    // sandy area
    carve_blob(c, 40, 35, 3, Tile::Sand);
  }

  // stairs: surface cave entrance (z=2) <-> shallow cave (z=1)
  place_stairs(&mut world.levels, 1, 2, 8, 38);
  clear_around(world.level_mut(1), 8, 38, 2);
  clear_around(world.level_mut(2), 8, 38, 2);

  // === z=0: deep cave ===
  {
    let d = world.level_mut(0);
    fill_rect(d, 0, 0, W, H, Tile::Wall);

    // huge main chamber
    carve_blob(d, 35, 30, 16, Tile::Floor);
    // northern chamber
    carve_blob(d, 20, 15, 10, Tile::Floor);
    // eastern alcove
    carve_blob(d, 60, 25, 7, Tile::Floor);
    // connecting passages
    place_wide_corridor(d, 28, 20, 35, 30);
    place_wide_corridor(d, 51, 30, 60, 25);

    // lava pool
    carve_blob(d, 42, 38, 5, Tile::Water);

    // sandy alcove
    carve_blob(d, 15, 40, 4, Tile::Sand);
  }

  // stairs: shallow cave (z=1) <-> deep cave (z=0)
  place_stairs(&mut world.levels, 0, 1, 30, 30);
  clear_around(world.level_mut(0), 30, 30, 2);
  clear_around(world.level_mut(1), 30, 30, 2);

  world
}

// ---------------------------------------------------------------------------
// SS13-style visibility (Bresenham raycasting)
//
// For each tile within `radius` of the viewer, cast a ray from viewer to
// target using Bresenham's line algorithm. Intermediate opaque tiles block
// vision, but an opaque tile itself is visible (you can see the wall face).
// This produces the sharp rectangular shadows characteristic of SS13.
// ---------------------------------------------------------------------------

pub struct FovGrid {
  pub visible: Vec<Vec<bool>>,
  pub revealed: Vec<Vec<bool>>,
  pub width: usize,
  pub height: usize
}

impl FovGrid {
  pub fn new(width: usize, height: usize) -> Self {
    FovGrid {
      visible: vec![vec![false; width]; height],
      revealed: vec![vec![false; width]; height],
      width,
      height
    }
  }

  pub fn clear_visible(&mut self) {
    for row in &mut self.visible {
      for cell in row.iter_mut() {
        *cell = false;
      }
    }
  }

  pub fn mark_visible(&mut self, x: usize, y: usize) {
    if x < self.width && y < self.height {
      self.visible[y][x] = true;
      self.revealed[y][x] = true;
    }
  }

  pub fn is_visible(&self, x: usize, y: usize) -> bool {
    x < self.width && y < self.height && self.visible[y][x]
  }

  pub fn is_revealed(&self, x: usize, y: usize) -> bool {
    x < self.width && y < self.height && self.revealed[y][x]
  }
}

/// Compute FOV from (cx, cy) with the given radius on the given level.
/// Uses SS13-style Bresenham raycasting: cast a ray to every tile on the
/// perimeter of the vision square, marking tiles visible along the way
/// until hitting an opaque tile (which is itself marked visible).
pub fn compute_fov(fov: &mut FovGrid, level: &Level, cx: i32, cy: i32, radius: i32) {
  fov.clear_visible();

  // viewer tile is always visible
  if cx >= 0 && cy >= 0 {
    fov.mark_visible(cx as usize, cy as usize);
  }

  // cast rays to every tile on the perimeter of the vision square
  for dy in -radius..=radius {
    cast_ray(fov, level, cx, cy, cx - radius, cy + dy);
    cast_ray(fov, level, cx, cy, cx + radius, cy + dy);
  }
  for dx in (-radius + 1)..radius {
    cast_ray(fov, level, cx, cy, cx + dx, cy - radius);
    cast_ray(fov, level, cx, cy, cx + dx, cy + radius);
  }
}

fn cast_ray(fov: &mut FovGrid, level: &Level, x0: i32, y0: i32, x1: i32, y1: i32) {
  let mut x = x0;
  let mut y = y0;
  let dx = (x1 - x0).abs();
  let dy = -(y1 - y0).abs();
  let sx = if x0 < x1 { 1 } else { -1 };
  let sy = if y0 < y1 { 1 } else { -1 };
  let mut err = dx + dy;

  loop {
    // mark current tile visible if in bounds
    if x >= 0 && y >= 0 && (x as usize) < fov.width && (y as usize) < fov.height {
      fov.mark_visible(x as usize, y as usize);

      // if this tile is opaque and it's not the origin, stop the ray
      if (x != x0 || y != y0) && level.get(x, y).is_some_and(|t| t.opaque()) {
        return;
      }
    }

    if x == x1 && y == y1 {
      return;
    }

    let e2 = 2 * err;
    if e2 >= dy {
      err += dy;
      x += sx;
    }
    if e2 <= dx {
      err += dx;
      y += sy;
    }
  }
}
