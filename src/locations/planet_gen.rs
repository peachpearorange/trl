use {crate::{entities::*,
             galaxy::{Location, LocationId},
             level::{Level, LocationType, Tile}},
     rand::{Rng, SeedableRng, rngs::SmallRng},
     std::collections::VecDeque};

pub const ID_ALIEN_JUNGLE: LocationId = (4, 0, 0);
pub const ID_CRYSTAL_CAVES: LocationId = (5, 0, 0);
pub const ID_ARCTIC_WASTE: LocationId = (6, 0, 0);
pub const ID_DESERT_WORLD: LocationId = (1, 1, 0);
pub const ID_LAVA_WORLD: LocationId = (7, 0, 0);
pub const ID_BRIGHT_WORLD: LocationId = (8, 0, 0);
pub const ID_PLANET_GRASS: LocationId = (9, 0, 0);
pub const ID_PLANET_GRABLOB: LocationId = (10, 0, 0);
pub const ID_PLANET_ZUGXUBLU: LocationId = (11, 0, 0);

pub fn all_ids() -> Vec<(LocationId, &'static str)> {
  vec![
    (ID_ALIEN_JUNGLE, "Xel-Nara IV"),
    (ID_CRYSTAL_CAVES, "Keth Caverns"),
    (ID_ARCTIC_WASTE, "Boreas Prime"),
    (ID_DESERT_WORLD, "Khamsin Reach"),
    (ID_LAVA_WORLD, "Pyros Maw"),
    (ID_BRIGHT_WORLD, "Lumos Reach"),
    (ID_PLANET_GRASS, "Planet Grass"),
    (ID_PLANET_GRABLOB, "Planet Grablob"),
    (ID_PLANET_ZUGXUBLU, "Planet Zugxublu"),
  ]
}

const GRID_ALIEN: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/planet_alien.bin"));
const GRID_CRYSTAL: &[u8] =
  include_bytes!(concat!(env!("OUT_DIR"), "/planet_crystal.bin"));
const GRID_ARCTIC: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/planet_arctic.bin"));
const GRID_DESERT: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/planet_desert.bin"));
const GRID_BRIGHT: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/planet_bright.bin"));
const GRID_LAVA: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/planet_lava.bin"));
const GRID_GRASS: &[u8] =
  include_bytes!("../../assets/generated/planets/planet_grass.bin");
const GRID_GRABLOB: &[u8] =
  include_bytes!("../../assets/generated/planets/planet_grablob.bin");
const GRID_ZUGXUBLU: &[u8] =
  include_bytes!("../../assets/generated/planets/planet_zugxublu.bin");
const PLANET_GRASS_MAGIC: &[u8; 4] = b"PGR1";

pub fn generate_by_id(id: LocationId) -> Option<Location> {
  match id {
    ID_ALIEN_JUNGLE => Some(generate(&PlanetParams::alien("Xel-Nara IV"), GRID_ALIEN)),
    ID_CRYSTAL_CAVES => {
      Some(generate(&PlanetParams::crystal("Keth Caverns"), GRID_CRYSTAL))
    }
    ID_ARCTIC_WASTE => Some(generate(&PlanetParams::arctic("Boreas Prime"), GRID_ARCTIC)),
    ID_DESERT_WORLD => {
      Some(generate(&PlanetParams::desert("Khamsin Reach"), GRID_DESERT))
    }
    ID_LAVA_WORLD => Some(generate_lava(&PlanetParams::lava("Pyros Maw"), GRID_LAVA)),
    ID_BRIGHT_WORLD => Some(generate(&PlanetParams::bright("Lumos Reach"), GRID_BRIGHT)),
    ID_PLANET_GRASS => {
      Some(generate_editor_planet(&PlanetParams::grassland("Planet Grass"), GRID_GRASS))
    }
    ID_PLANET_GRABLOB => Some(generate_editor_planet(
      &PlanetParams::grassland("Planet Grablob"),
      GRID_GRABLOB
    )),
    ID_PLANET_ZUGXUBLU => Some(generate_editor_planet(
      &PlanetParams::grassland("Planet Zugxublu"),
      GRID_ZUGXUBLU
    )),
    _ => None
  }
}

pub const PLANET_SIZE: usize = 300;
pub const SEED: u64 = 0xDEAD_BEEF;

#[derive(Clone, Copy, Debug)]
pub enum PlanetBiome {
  Grassland,
  Desert,
  Crystal,
  Alien,
  Arctic,
  Lava,
  Bright
}

pub struct PlanetParams {
  pub name: &'static str,
  pub biome: PlanetBiome,
  pub breathable: bool,
  pub water_coverage: f32,
  pub vegetation_density: f32,
  pub rock_frequency: f32
}

impl PlanetParams {
  pub fn grassland(name: &'static str) -> Self {
    Self {
      name,
      biome: PlanetBiome::Grassland,
      breathable: true,
      water_coverage: 0.3,
      vegetation_density: 0.5,
      rock_frequency: 0.1
    }
  }
  pub fn alien(name: &'static str) -> Self {
    Self {
      name,
      biome: PlanetBiome::Alien,
      breathable: false,
      water_coverage: 0.25,
      vegetation_density: 0.4,
      rock_frequency: 0.15
    }
  }
  pub fn lava(name: &'static str) -> Self {
    Self {
      name,
      biome: PlanetBiome::Lava,
      breathable: false,
      water_coverage: 0.35,
      vegetation_density: 0.0,
      rock_frequency: 0.4
    }
  }
  pub fn crystal(name: &'static str) -> Self {
    Self {
      name,
      biome: PlanetBiome::Crystal,
      breathable: false,
      water_coverage: 0.1,
      vegetation_density: 0.3,
      rock_frequency: 0.3
    }
  }
  pub fn arctic(name: &'static str) -> Self {
    Self {
      name,
      biome: PlanetBiome::Arctic,
      breathable: false,
      water_coverage: 0.2,
      vegetation_density: 0.0,
      rock_frequency: 0.25
    }
  }
  pub fn desert(name: &'static str) -> Self {
    Self {
      name,
      biome: PlanetBiome::Desert,
      breathable: false,
      water_coverage: 0.1,
      vegetation_density: 0.0,
      rock_frequency: 0.3
    }
  }
  pub fn bright(name: &'static str) -> Self {
    Self {
      name,
      biome: PlanetBiome::Bright,
      breathable: true,
      water_coverage: 0.15,
      vegetation_density: 0.0,
      rock_frequency: 0.3
    }
  }
  pub fn with_water(mut self, v: f32) -> Self {
    self.water_coverage = v;
    self
  }
  pub fn with_vegetation(mut self, v: f32) -> Self {
    self.vegetation_density = v;
    self
  }
  pub fn with_rocks(mut self, v: f32) -> Self {
    self.rock_frequency = v;
    self
  }
}

fn is_solid_ground(tile: Tile) -> bool {
  tile.walkable()
  // matches!(
  //   tile,
  //   Tile::Grass
  //     | Tile::TallGrass
  //     | Tile::Ash
  //     | Tile::CaveFloor
  //     | Tile::IceFloor
  //     | Tile::AlienSoil
  //     | Tile::AlienGrass
  //     | Tile::BrightGround
  // )
}

fn scaled(param: f32, scale: f32) -> f32 { (param * scale).max(0.05) }

fn editor_object(index: u16) -> Option<fn() -> Object> {
  match index {
    0 => Some(Object::tree as fn() -> Object),
    _ => None
  }
}

fn decode_editor_cell(encoded: u16) -> (Tile, Option<fn() -> Object>) {
  let tile = Tile::try_from(encoded & 0xFF).unwrap_or(Tile::Grass);
  let object = (encoded >> 8).checked_sub(1).and_then(editor_object);
  (tile, object)
}

enum EditorPlanetGrid<'a> {
  Palette { palette: &'a [u8], indices: &'a [u8] },
  Raw(&'a [u8])
}

impl<'a> EditorPlanetGrid<'a> {
  fn new(bytes: &'a [u8]) -> Self {
    if bytes.starts_with(PLANET_GRASS_MAGIC) {
      let palette_len = u16::from_le_bytes([bytes[4], bytes[5]]) as usize;
      let indices_start = 6 + palette_len * 2;
      Self::Palette {
        palette: &bytes[6..indices_start],
        indices: &bytes[indices_start..]
      }
    } else {
      Self::Raw(bytes)
    }
  }

  fn cell(&self, index: usize) -> u16 {
    match self {
      Self::Palette { palette, indices } => {
        let palette_index = indices[index] as usize;
        let byte_index = palette_index * 2;
        u16::from_le_bytes([palette[byte_index], palette[byte_index + 1]])
      }
      Self::Raw(bytes) => {
        let byte_index = index * 2;
        u16::from_le_bytes([bytes[byte_index], bytes[byte_index + 1]])
      }
    }
  }
}

fn generate_editor_planet(params: &PlanetParams, grid_cells: &[u8]) -> Location {
  let mut loc = Location::new(
    params.name,
    PLANET_SIZE,
    PLANET_SIZE,
    2,
    LocationType::PlanetSurface { breathable: params.breathable },
    Tile::Grass
  );
  let grid = EditorPlanetGrid::new(grid_cells);
  let mut spawn_objects = Vec::new();

  {
    let level = loc.level_mut(0);
    for y in 0..PLANET_SIZE {
      for x in 0..PLANET_SIZE {
        let (tile, entity_fn) = decode_editor_cell(grid.cell(y * PLANET_SIZE + x));
        level.set(x as i32, y as i32, tile);
        if let Some(spawn) = entity_fn {
          spawn_objects.push((x as i32, y as i32, 0, spawn()));
        }
      }
    }
    place_ship_dock(level, Tile::Grass);
  }
  loc.spawn_objects.extend(spawn_objects);

  generate_cave_sublevel(&mut loc);

  loc
}

fn generate(params: &PlanetParams, grid_indices: &[u8]) -> Location {
  let mut tile_map: Vec<(Tile, Option<fn() -> Object>)> = Vec::new();

  match params.biome {
    PlanetBiome::Grassland => {
      tile_map.push((Tile::Grass, None));
      tile_map.push((Tile::TallGrass, None));
      tile_map.push((Tile::Bush, None));
      tile_map.push((Tile::ShallowWater, None));
      tile_map.push((Tile::DeepWater, None));
      tile_map.push((Tile::Wall, None));
    }
    PlanetBiome::Desert => {
      tile_map.push((Tile::Ash, None));
      tile_map.push((Tile::CaveFloor, None));
      tile_map.push((Tile::CaveWall, None));
      tile_map.push((Tile::AlienFluid, None));
      tile_map.push((Tile::AcidPool, None));
    }
    PlanetBiome::Crystal => {
      tile_map.push((Tile::CaveFloor, None));
      tile_map.push((Tile::CaveWall, None));
      tile_map.push((Tile::CrystalFormation, None));
      tile_map.push((Tile::Ash, None));
      tile_map.push((Tile::BioluminescentPool, None));
      tile_map.push((Tile::AcidPool, None));
      tile_map
        .push((Tile::CrystalFormation, Some((|| Object::MANTIS_ALIEN.clone()) as fn() -> Object)));
    }
    PlanetBiome::Alien => {
      tile_map.push((Tile::AlienSoil, None));
      tile_map.push((Tile::AlienGrass, None));
      tile_map.push((Tile::AlienFluid, None));
      tile_map.push((Tile::BioluminescentPool, None));
      tile_map.push((Tile::CaveWall, None));
      tile_map.push((Tile::AlienSoil, Some((|| Object::ALIEN_RUNNER.clone()) as fn() -> Object)));
      tile_map.push((Tile::AlienSoil, Some((|| Object::CRAB_ALIEN.clone()) as fn() -> Object)));
    }
    PlanetBiome::Arctic => {
      tile_map.push((Tile::IceFloor, None));
      tile_map.push((Tile::IceWall, None));
      tile_map.push((Tile::ShallowWater, None));
      tile_map.push((Tile::DeepWater, None));
    }
    PlanetBiome::Lava => unreachable!("lava uses generate_lava"),
    PlanetBiome::Bright => {
      tile_map.push((Tile::BrightGround, None));
      tile_map.push((Tile::BrightCobbleWall, None));
      tile_map.push((Tile::ShallowWater, None));
      tile_map.push((Tile::DeepWater, None));
      tile_map.push((Tile::BrightGround, Some((|| Object::GUNMAN.clone()) as fn() -> Object)));
      tile_map
        .push((Tile::BrightGround, Some((|| Object::GRENADE_THROWER.clone()) as fn() -> Object)));
    }
  }

  let fill = tile_map[0].0;
  let mut loc = Location::new(
    params.name,
    PLANET_SIZE,
    PLANET_SIZE,
    2,
    LocationType::PlanetSurface { breathable: params.breathable },
    fill
  );

  {
    let level = loc.level_mut(0);
    for y in 0..PLANET_SIZE {
      for x in 0..PLANET_SIZE {
        let idx = grid_indices[y * PLANET_SIZE + x] as usize;
        let (tile, _) = tile_map[idx];
        level.set(x as i32, y as i32, tile);
      }
    }
    place_ship_dock(level, fill);
  }

  for y in 0..PLANET_SIZE {
    for x in 0..PLANET_SIZE {
      let idx = grid_indices[y * PLANET_SIZE + x] as usize;
      let (_, entity_fn) = tile_map[idx];
      if let Some(spawn) = entity_fn {
        loc.spawn_objects.push((x as i32, y as i32, 0, spawn()));
      }
    }
  }

  generate_cave_sublevel(&mut loc);

  loc
}

const CAVE_ENTRANCES: usize = 3;
const CAVE_FILL_CHANCE: f64 = 0.45;
const CAVE_SMOOTH_PASSES: usize = 5;

fn generate_cave_sublevel(loc: &mut Location) {
  let size = loc.width;
  let mut rng = SmallRng::seed_from_u64(SEED);

  let cave = loc.level_mut(1);
  for y in 0..size {
    for x in 0..size {
      cave.set(x as i32, y as i32, Tile::CaveWall);
    }
  }

  // Cellular automata: seed random floor cells, then smooth
  let mut cells = vec![vec![false; size]; size];
  for y in 2..size - 2 {
    for x in 2..size - 2 {
      cells[y][x] = rng.gen_bool(CAVE_FILL_CHANCE);
    }
  }
  for _ in 0..CAVE_SMOOTH_PASSES {
    let prev = cells.clone();
    for y in 1..size - 1 {
      for x in 1..size - 1 {
        let neighbors = (-1..=1i32)
          .flat_map(|dy| (-1..=1i32).map(move |dx| (dx, dy)))
          .filter(|&(dx, dy)| (dx, dy) != (0, 0))
          .filter(|&(dx, dy)| prev[(y as i32 + dy) as usize][(x as i32 + dx) as usize])
          .count();
        cells[y][x] = neighbors >= 5 || (cells[y][x] && neighbors >= 4);
      }
    }
  }

  for y in 0..size {
    for x in 0..size {
      if cells[y][x] {
        cave.set(x as i32, y as i32, Tile::CaveFloor);
      }
    }
  }

  // Find the largest connected cave region
  let largest = largest_walkable_component(cave);
  if largest.len() < 20 {
    return;
  }

  // Pick entrance positions: spread across the cave, on solid ground on the surface
  let surface = loc.levels[0].clone();
  let mut entrance_candidates: Vec<(i32, i32)> = largest
    .iter()
    .copied()
    .filter(|&(x, y)| {
      x > 5
        && y > 5
        && x < (size as i32 - 5)
        && y < (size as i32 - 5)
        && is_solid_ground(surface.get(x, y).unwrap_or(Tile::CaveWall))
    })
    .collect();

  // Sort deterministically
  entrance_candidates.sort_by_key(|&(x, y)| (x, y));

  let count = CAVE_ENTRANCES.min(entrance_candidates.len());
  if count == 0 {
    return;
  }

  // Space entrances apart by picking from evenly-spaced indices
  let step = entrance_candidates.len() / count;
  let entrances: Vec<(i32, i32)> =
    (0..count).map(|i| entrance_candidates[i * step]).collect();

  for &(ex, ey) in &entrances {
    // Clear a small area around the entrance in the cave
    for dy in -1..=1 {
      for dx in -1..=1 {
        let cave = loc.level_mut(1);
        if !cave.walkable(ex + dx, ey + dy) {
          cave.set(ex + dx, ey + dy, Tile::CaveFloor);
        }
      }
    }

    loc.spawn_objects.push((ex, ey, 0, Object::cave_entrance(ex, ey, ex, ey)));
    loc.spawn_objects.push((ex, ey, 1, Object::cave_exit(ex, ey, ex, ey)));
  }

  // Scatter loot chests in the cave
  let chest_candidates: Vec<(i32, i32)> = largest
    .iter()
    .copied()
    .filter(|&(x, y)| {
      !entrances.contains(&(x, y))
        && x > 3
        && y > 3
        && x < (size as i32 - 3)
        && y < (size as i32 - 3)
    })
    .collect();
  let chest_count = (chest_candidates.len() / 80).clamp(2, 6);
  let chest_step = chest_candidates.len().max(1) / chest_count.max(1);
  for i in 0..chest_count.min(chest_candidates.len()) {
    let (cx, cy) = chest_candidates[i * chest_step];
    loc.spawn_objects.push((cx, cy, 1, Object::LOOT_CHEST.clone()));
  }
}

fn largest_walkable_component(level: &Level) -> Vec<(i32, i32)> {
  let (w, h) = (level.width as i32, level.height as i32);
  let mut visited = vec![vec![false; level.width]; level.height];
  let mut best = Vec::new();

  for sy in 0..h {
    for sx in 0..w {
      if visited[sy as usize][sx as usize] || !level.walkable(sx, sy) {
        continue;
      }
      let mut component = Vec::new();
      let mut queue = VecDeque::new();
      visited[sy as usize][sx as usize] = true;
      queue.push_back((sx, sy));
      while let Some((x, y)) = queue.pop_front() {
        component.push((x, y));
        for (dx, dy) in [(1i32, 0), (-1, 0), (0, 1), (0, -1)] {
          let (nx, ny) = (x + dx, y + dy);
          if nx >= 0
            && ny >= 0
            && nx < w
            && ny < h
            && !visited[ny as usize][nx as usize]
            && level.walkable(nx, ny)
          {
            visited[ny as usize][nx as usize] = true;
            queue.push_back((nx, ny));
          }
        }
      }
      if component.len() > best.len() {
        best = component;
      }
    }
  }
  best
}

fn place_ship_dock(level: &mut Level, fill: Tile) {
  let w = level.width as i32;
  let h = level.height as i32;
  let (cx, cy) = (w / 2, h / 2);

  // Flood-fill every walkable component and find the largest one.
  // The dock must land there so the player isn't isolated in a small pocket.
  let mut visited = vec![vec![false; level.width]; level.height];
  let mut best: Vec<(i32, i32)> = Vec::new();

  for sy in 0..h {
    for sx in 0..w {
      if visited[sy as usize][sx as usize] || !level.walkable(sx, sy) {
        continue;
      }
      let mut component = Vec::new();
      let mut queue = VecDeque::new();
      visited[sy as usize][sx as usize] = true;
      queue.push_back((sx, sy));
      while let Some((x, y)) = queue.pop_front() {
        component.push((x, y));
        for (dx, dy) in [(1i32, 0), (-1, 0), (0, 1), (0, -1)] {
          let (nx, ny) = (x + dx, y + dy);
          if nx >= 0
            && ny >= 0
            && nx < w
            && ny < h
            && !visited[ny as usize][nx as usize]
            && level.walkable(nx, ny)
          {
            visited[ny as usize][nx as usize] = true;
            queue.push_back((nx, ny));
          }
        }
      }
      if component.len() > best.len() {
        best = component;
      }
    }
  }

  // Within the largest component, pick the solid-ground tile closest to map center.
  let Some(&(sx, sy)) = best
    .iter()
    .filter(|&&(x, y)| level.get(x, y).is_some_and(is_solid_ground))
    .min_by_key(|&&(x, y)| (x - cx).abs() + (y - cy).abs())
  else {
    return;
  };

  let max = PLANET_SIZE as i32 - 1;
  for py in (sy - 1).max(0)..=(sy + 1).min(max) {
    for px in (sx - 1).max(0)..=(sx + 1).min(max) {
      if !level.walkable(px, py) {
        level.set(px, py, fill);
      }
    }
  }
  level.set(sx, sy, Tile::ShipDock);
}

fn generate_lava(params: &PlanetParams, grid_indices: &[u8]) -> Location {
  // model_index → tile: 0=CaveWall, 1=Ash(edge), 2=Ash(straight), 3=Ash(corner), 4=Ash(T), 5=Ash(cross), 6=CaveWall(patches)
  let tile_map: &[Tile] = &[
    Tile::CaveWall,
    Tile::Ash,
    Tile::Ash,
    Tile::Ash,
    Tile::Ash,
    Tile::Ash,
    Tile::CaveWall
  ];

  let mut loc = Location::new(
    params.name,
    PLANET_SIZE,
    PLANET_SIZE,
    2,
    LocationType::PlanetSurface { breathable: params.breathable },
    Tile::Ash
  );

  {
    let level = loc.level_mut(0);
    for y in 0..PLANET_SIZE {
      for x in 0..PLANET_SIZE {
        level.set(
          x as i32,
          y as i32,
          tile_map[grid_indices[y * PLANET_SIZE + x] as usize]
        );
      }
    }

    let mut rng = SmallRng::seed_from_u64(SEED);
    let size = PLANET_SIZE as i32;
    for y in 0..size {
      for x in 0..size {
        if level.get(x, y) == Some(Tile::Ash) {
          let r: f64 = rng.r#gen();
          if r < 0.03 {
            level.set(x, y, Tile::Lava);
          } else if r < 0.04 {
            level.set(x, y, Tile::CrimsonPool);
          }
        }
      }
    }

    let dock = (PLANET_SIZE as i32 / 2, PLANET_SIZE as i32 / 2);
    for dy in -1..=1i32 {
      for dx in -1..=1i32 {
        level.set(dock.0 + dx, dock.1 + dy, Tile::Ash);
      }
    }
    level.set(dock.0, dock.1, Tile::ShipDock);
  }

  let mut rng = SmallRng::seed_from_u64(SEED);
  let size = PLANET_SIZE as i32;
  for y in 0..size {
    for x in 0..size {
      if loc.level(0).get(x, y) != Some(Tile::Ash) {
        let _: f64 = rng.r#gen();
      } else {
        let _: f64 = rng.r#gen();
        if rng.gen_bool(0.01) {
          loc.spawn_objects.push((x, y, 0, Object::LAVA_CRAB.clone()));
        }
      }
    }
  }

  generate_cave_sublevel(&mut loc);

  loc
}
