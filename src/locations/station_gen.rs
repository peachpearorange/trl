use rand::{Rng, SeedableRng, rngs::SmallRng};

use crate::{entities::*,
            galaxy::{Location, LocationId},
            level::{Level, LocationType, Tile}};

pub const STATION_SIZE: usize = 64;
pub const STATION_MARGIN: usize = 16;

pub const ID_NOVA_OUTPOST: LocationId = (2, 1, 0);
pub const ID_IRON_RING: LocationId = (3, 1, 0);
pub const ID_VEGA_RELAY: LocationId = (0, 4, 0);

pub fn all() -> Vec<(LocationId, Location)> {
  vec![
    (
      ID_NOVA_OUTPOST,
      generate(&StationParams::new("Nova Outpost").with_seed(0xABCD_1234))
    ),
    (
      ID_IRON_RING,
      generate(
        &StationParams::new("Iron Ring Station").with_decks(3).with_seed(0x9876_FEDC)
      )
    ),
    (
      ID_VEGA_RELAY,
      generate(
        &StationParams::new("Vega Relay")
          .with_decks(4)
          .with_rooms(10)
          .with_seed(0x1111_2222)
      )
    ),
  ]
}

// ---------------------------------------------------------------------------
// Params
// ---------------------------------------------------------------------------

pub struct StationParams {
  pub name: &'static str,
  pub decks: usize,
  pub rooms_per_deck: usize,
  pub seed: Option<u64>
}

impl StationParams {
  pub fn new(name: &'static str) -> Self {
    Self { name, decks: 2, rooms_per_deck: 7, seed: None }
  }

  pub fn with_seed(mut self, s: u64) -> Self {
    self.seed = Some(s);
    self
  }
  pub fn with_decks(mut self, d: usize) -> Self {
    self.decks = d;
    self
  }
  pub fn with_rooms(mut self, r: usize) -> Self {
    self.rooms_per_deck = r;
    self
  }
}

// ---------------------------------------------------------------------------
// BSP tree
// ---------------------------------------------------------------------------

#[derive(Clone)]
struct Rect {
  x: usize,
  y: usize,
  w: usize,
  h: usize
}

impl Rect {
  fn center(&self) -> (usize, usize) { (self.x + self.w / 2, self.y + self.h / 2) }
  fn inner(&self) -> Rect {
    Rect {
      x: self.x + 1,
      y: self.y + 1,
      w: self.w.saturating_sub(2),
      h: self.h.saturating_sub(2)
    }
  }
}

struct BspNode {
  cell: Rect,
  left: Option<Box<BspNode>>,
  right: Option<Box<BspNode>>,
  room: Option<Rect>
}

impl BspNode {
  fn leaf(cell: Rect) -> Self { BspNode { cell, left: None, right: None, room: None } }

  fn split(mut self, rng: &mut SmallRng, min_size: usize, depth: usize) -> Self {
    let can_split = self.cell.w >= min_size * 2 || self.cell.h >= min_size * 2;
    // Stop early only for medium-sized cells so we get fat-but-not-giant hub rooms.
    let medium = self.cell.w <= 22 && self.cell.h <= 22;
    let early_stop = can_split && depth > 0 && medium && rng.gen_bool(0.25);
    if depth == 0 || !can_split || early_stop {
      // Leaf: carve a room. Fat rooms get a small inset; regular rooms get a random one.
      let max_inset_x = if early_stop { 1 } else { (self.cell.w / 4).max(1) };
      let max_inset_y = if early_stop { 1 } else { (self.cell.h / 4).max(1) };
      let inset_x = rng.gen_range(1..=max_inset_x);
      let inset_y = rng.gen_range(1..=max_inset_y);
      let rw = self.cell.w.saturating_sub(inset_x * 2).max(3);
      let rh = self.cell.h.saturating_sub(inset_y * 2).max(3);
      self.room =
        Some(Rect { x: self.cell.x + inset_x, y: self.cell.y + inset_y, w: rw, h: rh });
      return self;
    }

    // Prefer splitting along the longer axis
    let split_h = if self.cell.w >= min_size * 2 && self.cell.h >= min_size * 2 {
      rng.gen_bool(0.5)
    } else {
      self.cell.w >= min_size * 2
    };

    if split_h {
      let split = rng.gen_range(min_size..=self.cell.w - min_size);
      let left_cell = Rect { x: self.cell.x, y: self.cell.y, w: split, h: self.cell.h };
      let right_cell = Rect {
        x: self.cell.x + split,
        y: self.cell.y,
        w: self.cell.w - split,
        h: self.cell.h
      };
      self.left =
        Some(Box::new(BspNode::leaf(left_cell).split(rng, min_size, depth - 1)));
      self.right =
        Some(Box::new(BspNode::leaf(right_cell).split(rng, min_size, depth - 1)));
    } else {
      let split = rng.gen_range(min_size..=self.cell.h - min_size);
      let left_cell = Rect { x: self.cell.x, y: self.cell.y, w: self.cell.w, h: split };
      let right_cell = Rect {
        x: self.cell.x,
        y: self.cell.y + split,
        w: self.cell.w,
        h: self.cell.h - split
      };
      self.left =
        Some(Box::new(BspNode::leaf(left_cell).split(rng, min_size, depth - 1)));
      self.right =
        Some(Box::new(BspNode::leaf(right_cell).split(rng, min_size, depth - 1)));
    }
    self
  }

  fn rooms(&self) -> Vec<&Rect> {
    if let Some(ref r) = self.room {
      vec![r]
    } else {
      let mut rooms = Vec::new();
      if let Some(ref l) = self.left {
        rooms.extend(l.rooms());
      }
      if let Some(ref r) = self.right {
        rooms.extend(r.rooms());
      }
      rooms
    }
  }

  /// Returns a point within the subtree's room space (center of a room).
  fn center(&self) -> (usize, usize) {
    if let Some(ref r) = self.room {
      r.center()
    } else if let Some(ref l) = self.left {
      l.center()
    } else {
      self.cell.center()
    }
  }

  /// Collect all corridors needed to connect sibling pairs.
  fn collect_corridors(&self, corridors: &mut Vec<((usize, usize), (usize, usize))>) {
    if let (Some(l), Some(r)) = (&self.left, &self.right) {
      corridors.push((l.center(), r.center()));
      l.collect_corridors(corridors);
      r.collect_corridors(corridors);
    }
  }
}

// ---------------------------------------------------------------------------
// Level generation
// ---------------------------------------------------------------------------

pub fn generate(params: &StationParams) -> Location {
  let seed = params.seed.unwrap_or(0x5EED_5EED);
  let mut rng = SmallRng::seed_from_u64(seed);

  let size = STATION_SIZE;
  let map_size = size + 2 * STATION_MARGIN;
  let fill = Tile::Vacuum;
  let mut loc = Location::new(
    params.name,
    map_size,
    map_size,
    params.decks,
    LocationType::SpaceStation,
    fill
  );

  // --- Generate each deck independently ---
  let mut stair_positions: Vec<(usize, usize)> = Vec::new(); // stair (x,y) per deck boundary
  let mut door_positions: Vec<(i32, i32, usize)> = Vec::new();

  for z in 0..params.decks {
    let level = loc.level_mut(z);
    // Outer border stays Vacuum; we stamp walls+floors per-room so outer
    // room walls are directly adjacent to Vacuum — enabling visible windows.

    // BSP rooms — offset by STATION_MARGIN so the station sits centred in the larger map
    let root_cell =
      Rect { x: STATION_MARGIN + 1, y: STATION_MARGIN + 1, w: size - 2, h: size - 2 };
    let bsp = BspNode::leaf(root_cell).split(&mut rng, 8, 4);

    // Stamp rooms: wall perimeter first, then floor interior
    for room in bsp.rooms() {
      for ry in room.y..room.y + room.h {
        for rx in room.x..room.x + room.w {
          level.set(rx as i32, ry as i32, Tile::StationWall);
        }
      }
      let inner = room.inner();
      for ry in inner.y..inner.y + inner.h {
        for rx in inner.x..inner.x + inner.w {
          level.set(rx as i32, ry as i32, Tile::StationFloor);
        }
      }
    }

    // Carve corridors
    let mut corridors = Vec::new();
    bsp.collect_corridors(&mut corridors);
    for ((ax, ay), (bx, by)) in &corridors {
      carve_corridor(level, *ax, *ay, *bx, *by);
    }

    // Internal walls in large rooms — pillar grid or spine wall with a passage gap.
    for room in bsp.rooms() {
      add_internal_walls(level, room, &mut rng);
    }

    // Place doors where corridors enter rooms; collect positions for later Object spawn.
    for room in bsp.rooms() {
      door_positions.extend(
        place_room_doors(level, room, &mut rng).into_iter().map(|(x, y)| (x, y, z))
      );
    }

    // Place conduit strips in some corridors for visual interest
    for ((ax, ay), (bx, by)) in &corridors {
      if rng.gen_bool(0.3) {
        stamp_conduit(level, *ax, *ay, *bx, *by);
      }
    }

    // Windows: StationWall adjacent to both Vacuum AND floor — visible from inside.
    place_windows(level, map_size, &mut rng);

    // Ship dock on level 0: find a walkable room center
    if z == 0 {
      let rooms = bsp.rooms();
      if let Some(room) = rooms.first() {
        let (dx, dy) = room.center();
        level.set(dx as i32, dy as i32, Tile::ShipDock);
      }
    }

    // Stairs: place them in a room interior far from dock
    // We collect a candidate position now; we'll wire them up after all levels are built.
    let rooms = bsp.rooms();
    let stair_room = rooms.get(rooms.len() / 2).or_else(|| rooms.last());
    let (sx, sy) = stair_room.map(|r| r.center()).unwrap_or((map_size / 2, map_size / 2));
    stair_positions.push((sx, sy));
  }

  // Spawn airlock Objects at door positions collected above.
  for (dx, dy, z) in door_positions {
    loc.spawn_objects.push((dx, dy, z, Object::airlock_door()));
  }

  // Build the full floor routing table: (deck_index, local_x, local_y) for every deck.
  let floors: Vec<(usize, i32, i32)> = stair_positions
    .iter()
    .enumerate()
    .map(|(z, &(sx, sy))| (z, sx as i32, sy as i32))
    .collect();

  // Spawn one elevator per deck, each knowing all other floors.
  for (z, &(sx, sy)) in stair_positions.iter().enumerate() {
    loc.spawn_objects.push((
      sx as i32,
      sy as i32,
      z,
      Object::elevator(z, floors.clone())
    ));
  }

  loc
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn carve_corridor(level: &mut Level, ax: usize, ay: usize, bx: usize, by: usize) {
  // L-shaped corridor: horizontal leg first, then vertical.
  // Where the corridor crosses Vacuum we also add StationWall flanks so the
  // corridor has solid walls (enabling door and window placement later).
  let (mut x, mut y) = (ax as i32, ay as i32);
  let (tx, ty) = (bx as i32, by as i32);

  let dx = (tx - x).signum();
  while x != tx {
    stamp_corridor_tile(level, x, y, true);
    x += dx;
  }
  let dy = (ty - y).signum();
  while y != ty {
    stamp_corridor_tile(level, x, y, false);
    y += dy;
  }
  stamp_corridor_tile(level, x, y, false);
}

/// Carve one corridor tile and ensure flanking walls exist in Vacuum.
fn stamp_corridor_tile(level: &mut Level, x: i32, y: i32, horizontal: bool) {
  if matches!(level.get(x, y), Some(Tile::StationWall) | Some(Tile::Vacuum)) {
    level.set(x, y, Tile::DeckPlate);
  }
  // Add StationWall flanks perpendicular to travel direction so crossing
  // Vacuum gaps still have a wall — enabling window detection later.
  let flanks: [(i32, i32); 2] =
    if horizontal { [(x, y - 1), (x, y + 1)] } else { [(x - 1, y), (x + 1, y)] };
  for (fx, fy) in flanks {
    if level.get(fx, fy) == Some(Tile::Vacuum) {
      level.set(fx, fy, Tile::StationWall);
    }
  }
}

fn place_room_doors(
  level: &mut Level,
  room: &Rect,
  rng: &mut SmallRng
) -> Vec<(i32, i32)> {
  // Scan the room perimeter; where corridor (DeckPlate) is adjacent to the room interior,
  // open the wall and return the position for the caller to spawn an airlock Object.
  let (rx, ry, rw, rh) = (room.x as i32, room.y as i32, room.w as i32, room.h as i32);
  let mut doors = Vec::new();

  // Top and bottom edges
  for x in rx..rx + rw {
    if level.get(x, ry) == Some(Tile::StationWall)
      && level.get(x, ry - 1) == Some(Tile::DeckPlate)
      && rng.gen_bool(0.6)
    {
      level.set(x, ry, Tile::StationFloor);
      doors.push((x, ry));
    }
    if level.get(x, ry + rh - 1) == Some(Tile::StationWall)
      && level.get(x, ry + rh) == Some(Tile::DeckPlate)
      && rng.gen_bool(0.6)
    {
      level.set(x, ry + rh - 1, Tile::StationFloor);
      doors.push((x, ry + rh - 1));
    }
  }
  // Left and right edges
  for y in ry..ry + rh {
    if level.get(rx, y) == Some(Tile::StationWall)
      && level.get(rx - 1, y) == Some(Tile::DeckPlate)
      && rng.gen_bool(0.6)
    {
      level.set(rx, y, Tile::StationFloor);
      doors.push((rx, y));
    }
    if level.get(rx + rw - 1, y) == Some(Tile::StationWall)
      && level.get(rx + rw, y) == Some(Tile::DeckPlate)
      && rng.gen_bool(0.6)
    {
      level.set(rx + rw - 1, y, Tile::StationFloor);
      doors.push((rx + rw - 1, y));
    }
  }
  doors
}

/// Place windows on StationWall tiles that face both Vacuum (exterior) and a
/// floor tile (interior) — guaranteeing visibility from inside the station.
fn place_windows(level: &mut Level, size: usize, rng: &mut SmallRng) {
  let max = size as i32;
  for y in 0..max {
    for x in 0..max {
      if level.get(x, y) != Some(Tile::StationWall) {
        continue;
      }
      let neighbors: [(i32, i32); 4] = [(x - 1, y), (x + 1, y), (x, y - 1), (x, y + 1)];
      let has_vacuum =
        neighbors.iter().any(|&(nx, ny)| level.get(nx, ny) == Some(Tile::Vacuum));
      let has_floor = neighbors.iter().any(|&(nx, ny)| {
        matches!(level.get(nx, ny), Some(Tile::StationFloor) | Some(Tile::DeckPlate))
      });
      if has_vacuum && has_floor && rng.gen_bool(0.45) {
        level.set(x, y, Tile::Window);
      }
    }
  }
}

/// Add internal walls to rooms large enough to benefit from them.
/// Only overwrites StationFloor so corridors (DeckPlate) are preserved.
fn add_internal_walls(level: &mut Level, room: &Rect, rng: &mut SmallRng) {
  let inner = room.inner();
  if inner.w >= 8 && inner.h >= 8 {
    add_spine_wall(level, &inner, rng);
  }
}

/// Spine wall: a partial wall through the room centre with a doorway-width gap.
/// The wall stops short of room edges so it never seals off a corridor entrance.
fn add_spine_wall(level: &mut Level, inner: &Rect, rng: &mut SmallRng) {
  let horizontal = rng.gen_bool(0.5);
  let gap = 3usize; // passage width
  if horizontal && inner.w > gap + 4 {
    let wy = (inner.y + inner.h / 2) as i32;
    // Gap at a random position in the middle half of the room.
    let gap_x = inner.x + rng.gen_range(2..inner.w.saturating_sub(gap + 2));
    let wall_start = (inner.x + 1) as i32;
    let wall_end = (inner.x + inner.w - 1) as i32;
    for x in wall_start..=wall_end {
      let in_gap = x >= gap_x as i32 && x < (gap_x + gap) as i32;
      if !in_gap && level.get(x, wy) == Some(Tile::StationFloor) {
        level.set(x, wy, Tile::StationWall);
      }
    }
  } else if !horizontal && inner.h > gap + 4 {
    let wx = (inner.x + inner.w / 2) as i32;
    let gap_y = inner.y + rng.gen_range(2..inner.h.saturating_sub(gap + 2));
    let wall_start = (inner.y + 1) as i32;
    let wall_end = (inner.y + inner.h - 1) as i32;
    for y in wall_start..=wall_end {
      let in_gap = y >= gap_y as i32 && y < (gap_y + gap) as i32;
      if !in_gap && level.get(wx, y) == Some(Tile::StationFloor) {
        level.set(wx, y, Tile::StationWall);
      }
    }
  }
}

fn stamp_conduit(level: &mut Level, ax: usize, ay: usize, bx: usize, _by: usize) {
  // Lay conduit along the horizontal leg of the corridor.
  let (mut x, y) = (ax as i32, ay as i32);
  let tx = bx as i32;
  let dx = (tx - x).signum();
  while x != tx {
    if level.get(x, y) == Some(Tile::DeckPlate) {
      level.set(x, y, Tile::Conduit);
    }
    x += dx;
  }
}
