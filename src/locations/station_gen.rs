use rand::{Rng, SeedableRng, rngs::SmallRng};

use crate::{
    entities::Object,
    galaxy::{Location, LocationId},
    level::{Level, LocationType, Tile},
};

pub const STATION_SIZE: usize = 64;

pub const ID_NOVA_OUTPOST: LocationId = (2, 1, 0);
pub const ID_IRON_RING:    LocationId = (3, 1, 0);
pub const ID_VEGA_RELAY:   LocationId = (0, 4, 0);

pub fn all() -> Vec<(LocationId, Location)> {
    vec![
        (ID_NOVA_OUTPOST, generate(&StationParams::new("Nova Outpost").with_seed(0xABCD_1234))),
        (ID_IRON_RING,    generate(&StationParams::new("Iron Ring Station").with_decks(3).with_seed(0x9876_FEDC))),
        (ID_VEGA_RELAY,   generate(&StationParams::new("Vega Relay").with_decks(4).with_rooms(10).with_seed(0x1111_2222))),
    ]
}

// ---------------------------------------------------------------------------
// Params
// ---------------------------------------------------------------------------

pub struct StationParams {
    pub name: &'static str,
    pub decks: usize,
    pub rooms_per_deck: usize,
    pub seed: Option<u64>,
}

impl StationParams {
    pub fn new(name: &'static str) -> Self {
        Self { name, decks: 2, rooms_per_deck: 7, seed: None }
    }

    pub fn with_seed(mut self, s: u64) -> Self { self.seed = Some(s); self }
    pub fn with_decks(mut self, d: usize) -> Self { self.decks = d; self }
    pub fn with_rooms(mut self, r: usize) -> Self { self.rooms_per_deck = r; self }
}

// ---------------------------------------------------------------------------
// BSP tree
// ---------------------------------------------------------------------------

#[derive(Clone)]
struct Rect { x: usize, y: usize, w: usize, h: usize }

impl Rect {
    fn center(&self) -> (usize, usize) { (self.x + self.w / 2, self.y + self.h / 2) }
    fn inner(&self) -> Rect {
        Rect {
            x: self.x + 1,
            y: self.y + 1,
            w: self.w.saturating_sub(2),
            h: self.h.saturating_sub(2),
        }
    }
}

struct BspNode {
    cell: Rect,
    left: Option<Box<BspNode>>,
    right: Option<Box<BspNode>>,
    room: Option<Rect>,
}

impl BspNode {
    fn leaf(cell: Rect) -> Self { BspNode { cell, left: None, right: None, room: None } }

    fn split(mut self, rng: &mut SmallRng, min_size: usize, depth: usize) -> Self {
        if depth == 0 || (self.cell.w < min_size * 2 && self.cell.h < min_size * 2) {
            // Leaf: carve a room with random inset
            let inset_x = rng.random_range(1..=(self.cell.w / 4).max(1));
            let inset_y = rng.random_range(1..=(self.cell.h / 4).max(1));
            let rw = self.cell.w.saturating_sub(inset_x * 2).max(3);
            let rh = self.cell.h.saturating_sub(inset_y * 2).max(3);
            self.room = Some(Rect {
                x: self.cell.x + inset_x,
                y: self.cell.y + inset_y,
                w: rw,
                h: rh,
            });
            return self;
        }

        // Prefer splitting along the longer axis
        let split_h = if self.cell.w >= min_size * 2 && self.cell.h >= min_size * 2 {
            rng.random_bool(0.5)
        } else {
            self.cell.w >= min_size * 2
        };

        if split_h {
            let split = rng.random_range(min_size..=self.cell.w - min_size);
            let left_cell = Rect { x: self.cell.x, y: self.cell.y, w: split, h: self.cell.h };
            let right_cell = Rect { x: self.cell.x + split, y: self.cell.y, w: self.cell.w - split, h: self.cell.h };
            self.left = Some(Box::new(BspNode::leaf(left_cell).split(rng, min_size, depth - 1)));
            self.right = Some(Box::new(BspNode::leaf(right_cell).split(rng, min_size, depth - 1)));
        } else {
            let split = rng.random_range(min_size..=self.cell.h - min_size);
            let left_cell = Rect { x: self.cell.x, y: self.cell.y, w: self.cell.w, h: split };
            let right_cell = Rect { x: self.cell.x, y: self.cell.y + split, w: self.cell.w, h: self.cell.h - split };
            self.left = Some(Box::new(BspNode::leaf(left_cell).split(rng, min_size, depth - 1)));
            self.right = Some(Box::new(BspNode::leaf(right_cell).split(rng, min_size, depth - 1)));
        }
        self
    }

    fn rooms(&self) -> Vec<&Rect> {
        if let Some(ref r) = self.room {
            vec![r]
        } else {
            let mut rooms = Vec::new();
            if let Some(ref l) = self.left  { rooms.extend(l.rooms()); }
            if let Some(ref r) = self.right { rooms.extend(r.rooms()); }
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
    let fill = Tile::Vacuum;
    let mut loc = Location::new(
        params.name,
        size,
        size,
        params.decks,
        LocationType::SpaceStation,
        fill,
    );

    // --- Generate each deck independently ---
    let mut stair_positions: Vec<(usize, usize)> = Vec::new(); // stair (x,y) per deck boundary

    for z in 0..params.decks {
        let level = loc.level_mut(z);
        // Fill with walls first; vacuum is the outer shell.
        // The outer border stays Vacuum; inner area starts as StationWall.
        let border = 2usize;
        for ry in border..size - border {
            for rx in border..size - border {
                level.set(rx as i32, ry as i32, Tile::StationWall);
            }
        }

        // BSP rooms
        let root_cell = Rect { x: border, y: border, w: size - border * 2, h: size - border * 2 };
        let bsp = BspNode::leaf(root_cell).split(&mut rng, 8, 4);

        // Carve rooms
        for room in bsp.rooms() {
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

        // Place doors where corridors enter rooms
        for room in bsp.rooms() {
            place_room_doors(level, room, &mut rng);
        }

        // Place conduit strips in some corridors for visual interest
        for ((ax, ay), (bx, by)) in &corridors {
            if rng.random_bool(0.3) {
                stamp_conduit(level, *ax, *ay, *bx, *by);
            }
        }

        // Windows: any StationWall directly facing Vacuum gets a random chance to become a Window.
        place_windows_facing_vacuum(level, size, &mut rng);

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
        let (sx, sy) = stair_room.map(|r| r.center()).unwrap_or((size / 2, size / 2));
        stair_positions.push((sx, sy));
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
            sx as i32, sy as i32, z,
            Object::elevator(z, floors.clone()),
        ));
    }

    loc
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn carve_corridor(level: &mut Level, ax: usize, ay: usize, bx: usize, by: usize) {
    // L-shaped corridor: move horizontally first, then vertically.
    let (mut x, mut y) = (ax as i32, ay as i32);
    let (tx, ty) = (bx as i32, by as i32);

    let dx = (tx - x).signum();
    while x != tx {
        if level.get(x, y) == Some(Tile::StationWall) || level.get(x, y) == Some(Tile::Vacuum) {
            level.set(x, y, Tile::DeckPlate);
        }
        x += dx;
    }
    let dy = (ty - y).signum();
    while y != ty {
        if level.get(x, y) == Some(Tile::StationWall) || level.get(x, y) == Some(Tile::Vacuum) {
            level.set(x, y, Tile::DeckPlate);
        }
        y += dy;
    }
    // Ensure endpoint is carved
    if level.get(x, y) == Some(Tile::StationWall) || level.get(x, y) == Some(Tile::Vacuum) {
        level.set(x, y, Tile::DeckPlate);
    }
}

fn place_room_doors(level: &mut Level, room: &Rect, rng: &mut SmallRng) {
    // Scan the room perimeter; where corridor (DeckPlate) is adjacent to the room interior,
    // place an AirlockDoor in the room wall tile.
    let (rx, ry, rw, rh) = (room.x as i32, room.y as i32, room.w as i32, room.h as i32);

    // Top and bottom edges
    for x in rx..rx + rw {
        if level.get(x, ry) == Some(Tile::StationWall)
            && level.get(x, ry - 1) == Some(Tile::DeckPlate)
            && rng.random_bool(0.6)
        {
            level.set(x, ry, Tile::AirlockDoor);
        }
        if level.get(x, ry + rh - 1) == Some(Tile::StationWall)
            && level.get(x, ry + rh) == Some(Tile::DeckPlate)
            && rng.random_bool(0.6)
        {
            level.set(x, ry + rh - 1, Tile::AirlockDoor);
        }
    }
    // Left and right edges
    for y in ry..ry + rh {
        if level.get(rx, y) == Some(Tile::StationWall)
            && level.get(rx - 1, y) == Some(Tile::DeckPlate)
            && rng.random_bool(0.6)
        {
            level.set(rx, y, Tile::AirlockDoor);
        }
        if level.get(rx + rw - 1, y) == Some(Tile::StationWall)
            && level.get(rx + rw, y) == Some(Tile::DeckPlate)
            && rng.random_bool(0.6)
        {
            level.set(rx + rw - 1, y, Tile::AirlockDoor);
        }
    }
}

/// Post-pass window placement: scan every StationWall tile; if any cardinal
/// neighbour is Vacuum (open space), randomly turn it into a Window.
/// This is correct regardless of BSP room insets or corridor positions.
fn place_windows_facing_vacuum(level: &mut Level, size: usize, rng: &mut SmallRng) {
    let max = size as i32;
    for y in 0..max {
        for x in 0..max {
            if level.get(x, y) != Some(Tile::StationWall) {
                continue;
            }
            let faces_vacuum = [(x - 1, y), (x + 1, y), (x, y - 1), (x, y + 1)]
                .iter()
                .any(|&(nx, ny)| level.get(nx, ny) == Some(Tile::Vacuum));
            if faces_vacuum && rng.random_bool(0.45) {
                level.set(x, y, Tile::Window);
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
