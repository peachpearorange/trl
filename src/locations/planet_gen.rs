use {crate::{entities::Object,
             galaxy::{Location, LocationId},
             level::{Level, LocationType, Tile}},
     bevy_ghx_proc_gen::proc_gen::{generator::{RngMode,
                                               builder::GeneratorBuilder,
                                               model::ModelCollection,
                                               rules::RulesBuilder,
                                               socket::{SocketCollection,
                                                        SocketsCartesian2D}},
                                   ghx_grid::cartesian::{coordinates::Cartesian2D,
                                                         grid::CartesianGrid}},
     rand::{Rng, SeedableRng, rngs::SmallRng},
     std::collections::VecDeque};

pub const ID_ALIEN_JUNGLE: LocationId = (4, 0, 0);
pub const ID_CRYSTAL_CAVES: LocationId = (5, 0, 0);
pub const ID_ARCTIC_WASTE: LocationId = (6, 0, 0);
pub const ID_DESERT_WORLD: LocationId = (1, 1, 0);
pub const ID_LAVA_WORLD: LocationId = (7, 0, 0);
pub const ID_BRIGHT_WORLD: LocationId = (8, 0, 0);

pub fn all() -> Vec<(LocationId, Location)> {
  vec![
    (
      ID_ALIEN_JUNGLE,
      generate(
        &PlanetParams::alien("Xel-Nara IV")
          .with_water(0.35)
          .with_vegetation(0.6)
      )
    ),
    (
      ID_CRYSTAL_CAVES,
      generate(
        &PlanetParams::crystal("Keth Caverns")
          .with_rocks(0.4)
          .with_vegetation(0.4)
      )
    ),
    (
      ID_ARCTIC_WASTE,
      generate(
        &PlanetParams::arctic("Boreas Prime")
          .with_water(0.25)
          .with_rocks(0.3)
      )
    ),
    (
      ID_DESERT_WORLD,
      generate(
        &PlanetParams::desert("Khamsin Reach")
          .with_water(0.08)
          .with_rocks(0.35)
      )
    ),
    (
      ID_LAVA_WORLD,
      generate_lava(
        &PlanetParams::lava("Pyros Maw").with_rocks(0.5)
      )
    ),
    (
      ID_BRIGHT_WORLD,
      generate(
        &PlanetParams::bright("Lumos Reach")
          .with_water(0.15)
          .with_rocks(0.3)
      )
    ),
  ]
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

pub fn generate(params: &PlanetParams) -> Location {
  let mut sockets = SocketCollection::new();
  let ground = sockets.create();
  let feature = sockets.create();
  let shallow = sockets.create();
  let deep = sockets.create();
  let rock = sockets.create();

  sockets.add_connections([
    (ground, vec![ground, feature, shallow, rock]),
    (feature, vec![feature, ground]),
    (shallow, vec![shallow, deep, ground]),
    (deep, vec![deep, shallow]),
    (rock, vec![rock, ground])
  ]);

  let mut models = ModelCollection::<Cartesian2D>::new();
  // (tile, optional entity to spawn when this model is placed)
  let mut tile_map: Vec<(Tile, Option<fn() -> Object>)> = Vec::new();

  macro_rules! tile {
    ($sock:expr, $weight:expr, $t:expr) => {{
      models.create(SocketsCartesian2D::Mono($sock)).with_weight($weight);
      tile_map.push(($t, None));
    }};
    ($sock:expr, $weight:expr, $t:expr, $e:expr) => {{
      models.create(SocketsCartesian2D::Mono($sock)).with_weight($weight);
      tile_map.push(($t, Some($e as fn() -> Object)));
    }};
  }

  let &PlanetParams {
    water_coverage: wc,
    vegetation_density: vd,
    rock_frequency: rf,
    ..
  } = params;

  // let (wc, vd, rf) = (params.water_coverage, params.vegetation_density, params.rock_frequency);

  match params.biome {
    PlanetBiome::Grassland => {
      tile!(ground, 10.0, Tile::Grass);
      tile!(feature, scaled(vd, 8.0), Tile::TallGrass); // tall grass clusters
      tile!(feature, scaled(vd, 4.0), Tile::Bush); // bush clusters
      tile!(shallow, scaled(wc, 8.0), Tile::ShallowWater);
      tile!(deep, scaled(wc, 4.0), Tile::DeepWater);
      tile!(rock, scaled(rf, 8.0), Tile::Wall);
    }
    PlanetBiome::Desert => {
      tile!(ground, 10.0, Tile::Ash);
      tile!(feature, scaled(rf, 5.0), Tile::CaveFloor); // hardpan patches
      tile!(rock, scaled(rf, 8.0), Tile::CaveWall);
      tile!(shallow, scaled(wc, 4.0), Tile::AlienFluid);
      tile!(deep, scaled(wc, 2.0), Tile::AcidPool);
    }
    PlanetBiome::Crystal => {
      tile!(ground, 8.0, Tile::CaveFloor);
      tile!(rock, scaled(rf, 8.0), Tile::CaveWall);
      tile!(feature, scaled(vd, 6.0), Tile::CrystalFormation); // crystal clusters
      tile!(feature, scaled(vd, 3.0), Tile::Ash);
      tile!(shallow, scaled(wc, 3.0), Tile::BioluminescentPool);
      tile!(deep, scaled(wc, 2.0), Tile::AcidPool);
      // Mantis: ambush predators lurking among crystal formations
      tile!(feature, 0.35, Tile::CrystalFormation, Object::mantis_alien);
    }
    PlanetBiome::Alien => {
      tile!(ground, 10.0, Tile::AlienSoil);
      tile!(feature, scaled(vd, 8.0), Tile::AlienGrass); // grass clusters in patches
      tile!(shallow, scaled(wc, 5.0), Tile::AlienFluid);
      tile!(deep, scaled(wc, 3.0), Tile::BioluminescentPool);
      tile!(rock, scaled(rf, 5.0), Tile::CaveWall);
      // Hunters: rare, WFC-placed on feature cells; underlying tile stays walkable
      tile!(feature, 0.4, Tile::AlienSoil, Object::alien_runner);
      // Crawlers: slower and tankier, spawn on ground
      tile!(ground, 0.3, Tile::AlienSoil, Object::crab_alien);
    }
    PlanetBiome::Arctic => {
      tile!(ground, 10.0, Tile::IceFloor);
      tile!(rock, scaled(rf, 8.0), Tile::IceWall);
      tile!(shallow, scaled(wc, 6.0), Tile::ShallowWater);
      tile!(deep, scaled(wc, 3.0), Tile::DeepWater);
    }
    PlanetBiome::Lava => unreachable!("lava uses generate_lava"),
    PlanetBiome::Bright => {
      tile!(ground, 10.0, Tile::BrightGround);
      tile!(rock, scaled(rf, 8.0), Tile::BrightCobbleWall);
      tile!(shallow, scaled(wc, 5.0), Tile::ShallowWater);
      tile!(deep, scaled(wc, 2.0), Tile::DeepWater);
      tile!(ground, 0.4, Tile::BrightGround, Object::gunman);
      tile!(ground, 0.1, Tile::BrightGround, Object::grenade_thrower);
    }
  }

  let rules = RulesBuilder::new_cartesian_2d(models, sockets)
    .build()
    .expect("planet_gen: rules build failed");
  let grid =
    CartesianGrid::new_cartesian_2d(PLANET_SIZE as u32, PLANET_SIZE as u32, false, false);
  let mut generator = GeneratorBuilder::new()
    .with_rules(rules)
    .with_grid(grid)
    .with_rng(RngMode::Seeded(SEED))
    .with_max_retry_count(100)
    .build()
    .expect("planet_gen: generator build failed");

  let (_info, grid_data) =
    generator.generate_grid().expect("planet_gen: generation failed");

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
    for y in 0..PLANET_SIZE as u32 {
      for x in 0..PLANET_SIZE as u32 {
        let (tile, _) = tile_map[grid_data.get_2d(x, y).model_index];
        level.set(x as i32, y as i32, tile);
      }
    }
    place_ship_dock(level, fill);
  }

  for y in 0..PLANET_SIZE as u32 {
    for x in 0..PLANET_SIZE as u32 {
      let (_, entity_fn) = tile_map[grid_data.get_2d(x, y).model_index];
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
      cells[y][x] = rng.random_bool(CAVE_FILL_CHANCE);
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
    loc.spawn_objects.push((cx, cy, 1, Object::loot_chest()));
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

fn generate_lava(params: &PlanetParams) -> Location {
  let mut sockets = SocketCollection::new();
  let s_wall = sockets.create();
  let s_edge = sockets.create();
  let s_run = sockets.create();

  sockets.add_connections([
    (s_wall, vec![s_wall, s_edge]),
    (s_edge, vec![s_wall, s_edge, s_run]),
    (s_run, vec![s_run, s_edge])
  ]);

  let mut models = ModelCollection::<Cartesian2D>::new();
  let mut tile_map: Vec<Tile> = Vec::new();

  // Rock
  models.create(SocketsCartesian2D::Mono(s_wall)).with_weight(3.0);
  tile_map.push(Tile::CaveWall);

  // Edge (corridor terminator / flexible transition)
  models.create(SocketsCartesian2D::Mono(s_edge)).with_weight(1.5);
  tile_map.push(Tile::Ash);

  // Straight corridor (extends along x, stackable along y for width)
  models
    .create(SocketsCartesian2D::Simple {
      x_pos: s_run,
      x_neg: s_run,
      y_pos: s_edge,
      y_neg: s_edge
    })
    .with_weight(5.0)
    // .with_all_rotations()
  ;
  tile_map.push(Tile::Ash);

  // Corner (s_run on two adjacent sides)
  models
    .create(SocketsCartesian2D::Simple {
      x_pos: s_run,
      x_neg: s_edge,
      y_pos: s_run,
      y_neg: s_edge
    })
    .with_weight(0.1)
    .with_all_rotations();
  tile_map.push(Tile::Ash);

  // T-junction (s_run on three sides)
  models
    .create(SocketsCartesian2D::Simple {
      x_pos: s_run,
      x_neg: s_run,
      y_pos: s_run,
      y_neg: s_edge
    })
    .with_weight(0.5)
    .with_all_rotations();
  tile_map.push(Tile::Ash);

  // Cross (s_run on all sides — full intersection)
  models.create(SocketsCartesian2D::Mono(s_run)).with_weight(0.5);
  tile_map.push(Tile::Ash);

  let rules = RulesBuilder::new_cartesian_2d(models, sockets)
    .build()
    .expect("lava_gen: rules build failed");
  let grid =
    CartesianGrid::new_cartesian_2d(PLANET_SIZE as u32, PLANET_SIZE as u32, false, false);
  let mut generator = GeneratorBuilder::new()
    .with_rules(rules)
    .with_grid(grid)
    .with_rng(RngMode::Seeded(SEED))
    .with_max_retry_count(100)
    .build()
    .expect("lava_gen: generator build failed");

  let (_info, grid_data) =
    generator.generate_grid().expect("lava_gen: generation failed");

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
    for y in 0..PLANET_SIZE as u32 {
      for x in 0..PLANET_SIZE as u32 {
        level.set(x as i32, y as i32, tile_map[grid_data.get_2d(x, y).model_index]);
      }
    }

    let mut rng = SmallRng::seed_from_u64(SEED);
    let size = PLANET_SIZE as i32;
    for y in 0..size {
      for x in 0..size {
        if level.get(x, y) == Some(Tile::Ash) {
          let r: f64 = rng.random();
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
        continue;
      }
      let _: f64 = rng.random();
      if rng.random_bool(0.01) {
        loc.spawn_objects.push((x, y, 0, Object::lava_crab()));
      }
    }
  }

  generate_cave_sublevel(&mut loc);

  loc
}
