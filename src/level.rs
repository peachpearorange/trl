#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tile {
  Air,
  Floor,
  Wall,
  CobblestoneWall,
  BrickWall,
  Grass,
  Water,
  Sand,
  StairsUp,
  StairsDown,
  Door,
  TallGrass,
  Bush,
  Ash,
  Lava,
  ShallowWater,
  DeepWater,
  Road,
  WoodWall,
  WoodFloor,
  Fence,
  CaveWall,
  CaveFloor,
  CrystalFormation,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Item {
  GoldCoin,
  HealthPotion,
  Torch,
  Rock,
  Mushroom
}

impl Item {
  pub fn name(self) -> &'static str {
    match self {
      Item::GoldCoin => "Gold Coin",
      Item::HealthPotion => "Health Potion",
      Item::Torch => "Torch",
      Item::Rock => "Rock",
      Item::Mushroom => "Mushroom"
    }
  }

  pub fn glyph(self) -> &'static str {
    match self {
      Item::GoldCoin => "$",
      Item::HealthPotion => "!",
      Item::Torch => "/",
      Item::Rock => "`",
      Item::Mushroom => "%"
    }
  }

  pub fn color(self) -> [f32; 3] {
    match self {
      Item::GoldCoin => [1.0, 0.85, 0.0],
      Item::HealthPotion => [0.9, 0.2, 0.3],
      Item::Torch => [1.0, 0.6, 0.1],
      Item::Rock => [0.5, 0.5, 0.5],
      Item::Mushroom => [0.6, 0.3, 0.7]
    }
  }
}

impl Tile {
  pub fn glyph(self) -> &'static str {
    match self {
      Tile::Air => " ",
      Tile::Floor => ".",
      Tile::Wall | Tile::CobblestoneWall | Tile::BrickWall => "#",
      Tile::Grass => "\"",
      Tile::Water => "~",
      Tile::Sand => ",",
      Tile::StairsUp => "<",
      Tile::StairsDown => ">",
      Tile::Door => "+",
      Tile::TallGrass => "\"",
      Tile::Bush => "%",
      Tile::Ash => ".",
      Tile::Lava => "~",
      Tile::ShallowWater => "~",
      Tile::DeepWater => "≈",
      Tile::Road => "·",
      Tile::WoodWall => "#",
      Tile::WoodFloor => ".",
      Tile::Fence => "+",
      Tile::CaveWall => "#",
      Tile::CaveFloor => ".",
      Tile::CrystalFormation => "*",
    }
  }

  pub fn color(self) -> [f32; 3] {
    match self {
      Tile::Air => [0.0, 0.0, 0.0],
      Tile::Floor => [0.6, 0.5, 0.3],
      Tile::Wall => [0.4, 0.4, 0.4],
      Tile::CobblestoneWall => [0.5, 0.5, 0.5],
      Tile::BrickWall => [0.6, 0.3, 0.2],
      Tile::Grass => [0.2, 0.6, 0.2],
      Tile::Water => [0.2, 0.3, 0.8],
      Tile::Sand => [0.8, 0.7, 0.4],
      Tile::StairsUp => [0.9, 0.9, 0.2],
      Tile::StairsDown => [0.9, 0.9, 0.2],
      Tile::Door => [0.6, 0.3, 0.1],
      Tile::TallGrass => [0.25, 0.65, 0.25],
      Tile::Bush => [0.15, 0.45, 0.15],
      Tile::Ash => [0.55, 0.53, 0.5],
      Tile::Lava => [0.9, 0.3, 0.05],
      Tile::ShallowWater => [0.3, 0.5, 0.85],
      Tile::DeepWater => [0.1, 0.15, 0.6],
      Tile::Road => [0.45, 0.4, 0.35],
      Tile::WoodWall => [0.45, 0.3, 0.15],
      Tile::WoodFloor => [0.55, 0.4, 0.25],
      Tile::Fence => [0.5, 0.35, 0.2],
      Tile::CaveWall => [0.3, 0.28, 0.25],
      Tile::CaveFloor => [0.4, 0.38, 0.35],
      Tile::CrystalFormation => [0.5, 0.8, 0.95],
    }
  }

  pub fn texture_path(self) -> Option<&'static str> {
    match self {
      Tile::CobblestoneWall => Some("textures/cobblestone_wall.png"),
      Tile::BrickWall => Some("textures/brick_wall.png"),
      _ => None,
    }
  }

  pub fn walkable(self) -> bool {
    matches!(
      self,
      Tile::Air
        | Tile::Floor
        | Tile::Grass
        | Tile::Sand
        | Tile::StairsUp
        | Tile::StairsDown
        | Tile::TallGrass
        | Tile::Ash
        | Tile::Road
        | Tile::WoodFloor
        | Tile::CaveFloor
        | Tile::ShallowWater
    )
  }
}

impl Tile {
  pub fn opaque(self) -> bool {
    matches!(
      self,
      Tile::Wall
        | Tile::CobblestoneWall
        | Tile::BrickWall
        | Tile::WoodWall
        | Tile::CaveWall
        | Tile::Door
    )
  }

  /// True when an entity standing here should fall to the level below.
  pub fn causes_falling(self) -> bool { matches!(self, Tile::Air) }

  pub fn name(self) -> &'static str {
    match self {
      Tile::Air => "Air",
      Tile::Floor => "Floor",
      Tile::Wall => "Wall",
      Tile::CobblestoneWall => "Cobblestone Wall",
      Tile::BrickWall => "Brick Wall",
      Tile::Grass => "Grass",
      Tile::Water => "Water",
      Tile::Sand => "Sand",
      Tile::StairsUp => "Stairs Up",
      Tile::StairsDown => "Stairs Down",
      Tile::Door => "Door",
      Tile::TallGrass => "Tall Grass",
      Tile::Bush => "Bush",
      Tile::Ash => "Ash",
      Tile::Lava => "Lava",
      Tile::ShallowWater => "Shallow Water",
      Tile::DeepWater => "Deep Water",
      Tile::Road => "Road",
      Tile::WoodWall => "Wooden Wall",
      Tile::WoodFloor => "Wooden Floor",
      Tile::Fence => "Fence",
      Tile::CaveWall => "Cave Wall",
      Tile::CaveFloor => "Cave Floor",
      Tile::CrystalFormation => "Crystal Formation",
    }
  }
}

pub struct Level {
  pub tiles: Vec<Vec<Tile>>,
  pub items: Vec<Vec<Option<Item>>>,
  pub width: usize,
  pub height: usize
}

impl Level {
  pub fn new(width: usize, height: usize, fill: Tile) -> Self {
    Level {
      tiles: vec![vec![fill; width]; height],
      items: vec![vec![None; width]; height],
      width,
      height
    }
  }

  pub fn get(&self, x: i32, y: i32) -> Option<Tile> {
    if x < 0 || y < 0 {
      return None;
    }
    let (ux, uy) = (x as usize, y as usize);
    (ux < self.width && uy < self.height).then(|| self.tiles[uy][ux])
  }

  pub fn get_item(&self, x: i32, y: i32) -> Option<Item> {
    if x < 0 || y < 0 {
      return None;
    }
    let (ux, uy) = (x as usize, y as usize);
    (ux < self.width && uy < self.height).then(|| self.items[uy][ux]).flatten()
  }

  pub fn set_item(&mut self, x: i32, y: i32, item: Option<Item>) {
    if x >= 0 && y >= 0 && (x as usize) < self.width && (y as usize) < self.height {
      self.items[y as usize][x as usize] = item;
    }
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

///// Place a room: wall border with floor interior.
pub fn place_room(level: &mut Level, x: i32, y: i32, w: usize, h: usize, wall: Tile) {
  fill_rect(level, x, y, w, h, wall);
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
  door_offset: usize,
  wall: Tile
) {
  place_room(level, x, y, w, h, wall);
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
    levels[z_from].set(x, y, Tile::StairsUp);
    levels[z_to].set(x, y, Tile::StairsDown);
  } else {
    levels[z_from].set(x, y, Tile::StairsDown);
    levels[z_to].set(x, y, Tile::StairsUp);
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
// Zone world — 10×10×4 grid of 48×48 Levels
// ---------------------------------------------------------------------------

pub const ZONE_WIDTH:  usize = 48;
pub const ZONE_HEIGHT: usize = 48;
pub const WORLD_COLS:  usize = 10;
pub const WORLD_ROWS:  usize = 10;
pub const WORLD_DEPTH: usize = 4;

/// A 10×10×4 grid of zones.  zones[zx][zy][z] is one 48×48 Level.
/// Surface is z=3; underground levels are z=2, z=1, z=0.
pub struct ZoneWorld {
  pub zones: Vec<Vec<Vec<Level>>>,
}

impl ZoneWorld {
  /// Construct an empty ZoneWorld; every level is filled with `fill`.
  pub fn new(fill: Tile) -> Self {
    let zones = (0..WORLD_COLS)
      .map(|_| {
        (0..WORLD_ROWS)
          .map(|_| {
            (0..WORLD_DEPTH)
              .map(|_| Level::new(ZONE_WIDTH, ZONE_HEIGHT, fill))
              .collect()
          })
          .collect()
      })
      .collect();
    ZoneWorld { zones }
  }

  pub fn zone(&self, zx: usize, zy: usize, z: usize) -> &Level {
    &self.zones[zx][zy][z]
  }

  pub fn zone_mut(&mut self, zx: usize, zy: usize, z: usize) -> &mut Level {
    &mut self.zones[zx][zy][z]
  }

  pub fn in_bounds(&self, zx: i32, zy: i32) -> bool {
    zx >= 0 && zy >= 0
      && (zx as usize) < WORLD_COLS
      && (zy as usize) < WORLD_ROWS
  }
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
    place_room_with_door(s, 30, 8, 12, 10, Side::South, 6, Tile::BrickWall);
    // interior detail
    fill_rect(s, 36, 9, 1, 5, Tile::BrickWall);

    // cave entrance: open air you fall through (stairs at 8,38 are placed after)
    fill_rect(s, 5, 35, 8, 6, Tile::Air);

    // pond
    fill_rect(s, 50, 35, 7, 5, Tile::Water);

    // some trees (wall tiles on grass)
    for &(tx, ty) in &[(15, 15), (18, 12), (22, 16), (45, 20), (55, 14), (60, 22)] {
      s.set(tx, ty, Tile::Wall);
    }

    // surface items
    for &(tx, ty, item) in &[
      (12, 30, Item::GoldCoin),
      (25, 12, Item::Rock),
      (48, 22, Item::Torch),
      (65, 40, Item::Mushroom)
    ] {
      s.set_item(tx, ty, Some(item));
    }
  }

  // === z=3: building upper floor ===
  {
    let u = world.level_mut(3);
    place_room_with_door(u, 30, 8, 12, 10, Side::South, 6, Tile::BrickWall);
    fill_rect(u, 35, 9, 1, 4, Tile::BrickWall);
    u.set(35, 12, Tile::Door);
  }

  // stairs between building floors (z=2 <-> z=3)
  place_stairs(&mut world.levels, 2, 3, 40, 10);

  // === z=1: shallow cave ===
  {
    let c = world.level_mut(1);
    fill_rect(c, 0, 0, W, H, Tile::CobblestoneWall);

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

    // shallow cave items
    for &(tx, ty, item) in &[
      (25, 28, Item::GoldCoin),
      (25, 29, Item::GoldCoin),
      (50, 18, Item::HealthPotion),
      (35, 32, Item::Torch),
      (12, 36, Item::Mushroom)
    ] {
      c.set_item(tx, ty, Some(item));
    }
  }

  // stairs: surface cave entrance (z=2) <-> shallow cave (z=1)
  place_stairs(&mut world.levels, 1, 2, 8, 38);
  clear_around(world.level_mut(1), 8, 38, 2);
  clear_around(world.level_mut(2), 8, 38, 2);

  // === z=0: deep cave ===
  {
    let d = world.level_mut(0);
    fill_rect(d, 0, 0, W, H, Tile::CobblestoneWall);

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

    // deep cave items
    for &(tx, ty, item) in &[
      (30, 28, Item::GoldCoin),
      (31, 28, Item::GoldCoin),
      (30, 29, Item::GoldCoin),
      (20, 13, Item::HealthPotion),
      (58, 23, Item::Torch),
      (40, 35, Item::Rock),
      (14, 39, Item::Mushroom),
      (16, 41, Item::Mushroom),
      (62, 26, Item::GoldCoin)
    ] {
      d.set_item(tx, ty, Some(item));
    }
  }

  // stairs: shallow cave (z=1) <-> deep cave (z=0)
  place_stairs(&mut world.levels, 0, 1, 30, 30);
  clear_around(world.level_mut(0), 30, 30, 2);
  clear_around(world.level_mut(1), 30, 30, 2);

  world
}

// ---------------------------------------------------------------------------
// Visibility: perimeter flood-fill
//
// Expand outward chebyshev-ring by chebyshev-ring from the viewer.
// A tile is visible if any of its parent tiles (one step closer to the
// viewer along each axis) is itself visible and not opaque.
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
/// Uses perimeter flood-fill: expand outward ring by ring; a tile is visible
/// if any of its parents (one step closer along each axis) are visible and
/// not opaque.
pub fn compute_fov(fov: &mut FovGrid, level: &Level, cx: i32, cy: i32, radius: i32) {
  fov.clear_visible();

  // viewer tile is always visible
  if cx >= 0 && cy >= 0 && (cx as usize) < fov.width && (cy as usize) < fov.height {
    fov.mark_visible(cx as usize, cy as usize);
  }

  // local visibility grid, offset-relative to viewer
  let size = (2 * radius + 1) as usize;
  let mut vis = vec![vec![false; size]; size];
  let r = radius as usize;
  vis[r][r] = true;

  fn sign(n: i32) -> i32 {
    if n > 0 { 1 } else if n < 0 { -1 } else { 0 }
  }

  for d in 1..=radius {
    for dx in -d..=d {
      for dy in -d..=d {
        if dx.abs().max(dy.abs()) != d {
          continue;
        }
        let (sx, sy) = (sign(dx), sign(dy));
        // All parents are on ring d-1, so iteration order doesn't matter.
        // Corners use only the diagonal parent to ensure a single diagonal
        // wall tile properly occludes. Edge tiles use two inward parents
        // along their dominant axis so they aren't over-blocked.
        let parents: &[(i32, i32)] = if dx == 0 {
          &[(0, -sy)]
        } else if dy == 0 {
          &[(-sx, 0)]
        } else if dx.abs() == dy.abs() {
          // corner: only the diagonal d-1 parent
          &[(-sx, -sy)]
        } else if dx.abs() > dy.abs() {
          // vertical edge: two parents one step inward along x
          &[(-sx, 0), (-sx, -sy)]
        } else {
          // horizontal edge: two parents one step inward along y
          &[(0, -sy), (-sx, -sy)]
        };

        let visible = parents.iter().any(|&(px, py)| {
          let (pj, pi) = ((dx + px) + radius, (dy + py) + radius);
          let (uj, ui) = (pj as usize, pi as usize);
          uj < size
            && ui < size
            && vis[ui][uj]
            && !level
              .get(cx + dx + px, cy + dy + py)
              .is_some_and(|t| t.opaque())
        });

        if visible {
          let (j, i) = ((dx + radius) as usize, (dy + radius) as usize);
          vis[i][j] = true;
          let (wx, wy) = (cx + dx, cy + dy);
          if wx >= 0 && wy >= 0 && (wx as usize) < fov.width && (wy as usize) < fov.height {
            fov.mark_visible(wx as usize, wy as usize);
          }
        }
      }
    }
  }
}
