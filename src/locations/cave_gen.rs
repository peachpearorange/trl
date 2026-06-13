//! Multi-level cave generation shared by every planet type.
//!
//! Why not cellular automata: pure CA produces a fragmented archipelago of
//! round pockets — no long winding passages, no guaranteed connectivity, and
//! every level looks the same. This generator works the other way around:
//!
//! 1. **Caverns first.** Scatter cavern centers across the level by dart
//!    throwing with a minimum spacing, then carve each as a noise-perturbed
//!    blob so no two chambers share a silhouette.
//! 2. **Tunnels as a graph.** Connect the caverns with a minimum spanning
//!    tree (guaranteed connectivity) plus a fraction of extra edges (loops,
//!    so the caves sprawl instead of forming a single corridor). Each edge is
//!    carved by a wobbly walk that steers toward its target while drifting
//!    randomly — long, windy, naturally varying-width tunnels.
//! 3. **Roughen.** A floor-only growth pass eats noise into the walls. It
//!    never removes floor, so connectivity survives.
//! 4. **Stack levels.** A planet gets 2–4 underground levels. Each level
//!    rolls its own *theme* (creature set, pools, decorations), and the
//!    descending passages of level `z` become forced cavern sites of level
//!    `z + 1`, so stairways always land in open chambers.
//! 5. **Populate by distance.** A BFS from the level's entry points drives
//!    placement: creatures only spawn a safe distance in, loot chests favor
//!    the farthest caverns, and the deepest hoards are supply caches.

use {crate::{entities::*, galaxy::Location, level::{Item, Level, Tile}},
     bevy::prelude::Color,
     noise::{Fbm, MultiFractal, NoiseFn, Perlin},
     rand::{Rng, SeedableRng, rngs::SmallRng},
     std::collections::VecDeque};

/// Border of the level that is never carved.
const MARGIN: i32 = 3;
/// Creatures spawn at least this far (BFS steps) from a level's entry points.
const SAFE_RADIUS: i32 = 14;

const CACHE_MID: &[(Item, u32)] =
  &[(Item::PipeRevolver, 1), (Item::HealthPotion, 2), (Item::GoldCoin, 14)];
const CACHE_DEEP: &[(Item, u32)] = &[
  (Item::ChainMail, 1),
  (Item::HealthPotion, 3),
  (Item::Crystal, 4),
  (Item::GoldCoin, 25)
];

/// Stable per-location seed so different planets get different caves (FNV-1a).
pub fn name_seed(name: &str) -> u64 {
  name.bytes().fold(0xcbf2_9ce4_8422_2325, |h, b| (h ^ b as u64).wrapping_mul(0x100_0000_01b3))
}

struct NoiseField {
  base: Fbm<Perlin>
}

impl NoiseField {
  fn new(seed: u32, frequency: f64, octaves: usize) -> Self {
    Self { base: Fbm::<Perlin>::new(seed).set_frequency(frequency).set_octaves(octaves) }
  }

  /// Noise remapped from roughly [-1, 1] to [0, 1].
  fn sample01(&self, x: f64, y: f64) -> f64 { (self.base.get([x, y]) + 1.0) * 0.5 }
}

fn weighted<'a, T>(rng: &mut SmallRng, table: &'a [(u32, T)]) -> &'a T {
  let total: u32 = table.iter().map(|&(w, _)| w).sum();
  let mut roll = rng.gen_range(0..total) as i64;
  table
    .iter()
    .find(|&&(w, _)| {
      roll -= w as i64;
      roll < 0
    })
    .map(|(_, t)| t)
    .unwrap()
}

/// Per-level flavor: each underground level rolls one, so the creature set,
/// liquids, and decorations change as the player descends.
#[derive(Clone, Copy, PartialEq, Eq)]
enum CaveTheme {
  /// Dirt patches, scree, boulders, scavengers.
  Rocky,
  /// Glowing pools, mushroom thickets, spore beasts.
  Fungal,
  /// Half-drowned galleries with sand banks and deep water.
  Flooded,
  /// Crystal veins in the walls, shards underfoot, mantis predators.
  Crystal,
  /// Crimson seeps and lava cores. Deepest levels only.
  Molten
}

impl CaveTheme {
  fn pick(rng: &mut SmallRng, depth: usize, prev: Option<Self>) -> Self {
    let table: &[(u32, Self)] = match depth {
      1 => &[(4, Self::Rocky), (3, Self::Fungal), (3, Self::Flooded)],
      2 => &[(2, Self::Rocky), (3, Self::Fungal), (2, Self::Flooded), (3, Self::Crystal)],
      _ => &[(1, Self::Rocky), (2, Self::Fungal), (4, Self::Crystal), (3, Self::Molten)]
    };
    let first = *weighted(rng, table);
    if prev == Some(first) { *weighted(rng, table) } else { first }
  }

  fn creatures(self) -> &'static [(u32, fn() -> Object)] {
    match self {
      Self::Rocky => &[
        (3, || Object::ALIEN_RUNNER),
        (3, || Object::CRAB_ALIEN),
        (2, || Object::RAT_SOLDIER)
      ],
      Self::Fungal => &[(5, || Object::MUSHROOM_CREATURE), (2, || Object::CRAB_ALIEN), (2, || Object::POLYCHROMATIC_SHEEP)],
      Self::Flooded => &[(4, || Object::CRAB_ALIEN), (2, || Object::ALIEN_RUNNER)],
      Self::Crystal => &[(4, || Object::MANTIS_ALIEN), (2, || Object::ALIEN_RUNNER)],
      Self::Molten => &[(1, || Object::LAVA_CRAB)]
    }
  }

  /// Walkable liquid scattered where the pool noise runs high.
  fn pool(self) -> (Tile, f64) {
    match self {
      Self::Rocky => (Tile::ShallowWater, 0.74),
      Self::Fungal => (Tile::BioluminescentPool, 0.70),
      Self::Flooded => (Tile::ShallowWater, 0.56),
      Self::Crystal => (Tile::BioluminescentPool, 0.74),
      Self::Molten => (Tile::CrimsonPool, 0.64)
    }
  }

  /// Impassable liquid at pool cores, kept away from walls so the shore
  /// around it always stays walkable.
  fn deep_pool(self) -> Option<Tile> {
    match self {
      Self::Flooded => Some(Tile::DeepWater),
      Self::Molten => Some(Tile::Lava),
      _ => None
    }
  }
}

/// Boolean carve mask for one level; true = floor.
struct Carved {
  size: i32,
  floor: Vec<bool>
}

impl Carved {
  fn new(size: i32) -> Self {
    Self { size, floor: vec![false; (size * size) as usize] }
  }

  fn get(&self, x: i32, y: i32) -> bool {
    x >= 0 && y >= 0 && x < self.size && y < self.size
      && self.floor[(y * self.size + x) as usize]
  }

  fn set(&mut self, x: i32, y: i32) {
    if x >= MARGIN && y >= MARGIN && x < self.size - MARGIN && y < self.size - MARGIN {
      self.floor[(y * self.size + x) as usize] = true
    }
  }

  fn carve_disc(&mut self, cx: f32, cy: f32, r: f32) {
    let rr = r.ceil() as i32;
    for dy in -rr..=rr {
      for dx in -rr..=rr {
        if ((dx * dx + dy * dy) as f32) <= r * r {
          self.set(cx as i32 + dx, cy as i32 + dy)
        }
      }
    }
  }

  fn floor_neighbors8(&self, x: i32, y: i32) -> usize {
    (-1..=1)
      .flat_map(|dy| (-1..=1).map(move |dx| (dx, dy)))
      .filter(|&(dx, dy)| (dx, dy) != (0, 0) && self.get(x + dx, y + dy))
      .count()
  }
}

/// Dart-throw cavern centers with minimum spacing. `forced` sites (stairway
/// landings from the level above) always become caverns.
fn scatter_caverns(
  rng: &mut SmallRng,
  size: i32,
  forced: &[(i32, i32)]
) -> Vec<(i32, i32, f32)> {
  let mut caverns: Vec<(i32, i32, f32)> =
    forced.iter().map(|&(x, y)| (x, y, rng.gen_range(4.0..7.0))).collect();
  let target = forced.len() + (((size * size) / 4200) as usize).max(6);
  let min_dist = size as f32 * 0.075;
  let lo = MARGIN + 8;
  let hi = size - MARGIN - 8;
  let mut attempts = 0;
  while caverns.len() < target && attempts < 1000 {
    attempts += 1;
    let (x, y) = (rng.gen_range(lo..hi), rng.gen_range(lo..hi));
    let clear = caverns
      .iter()
      .all(|&(cx, cy, _)| (((cx - x).pow(2) + (cy - y).pow(2)) as f32) > min_dist * min_dist);
    if clear {
      caverns.push((x, y, rng.gen_range(4.5..13.0)))
    }
  }
  caverns
}

/// Minimum spanning tree over the caverns plus a few extra edges, so the
/// tunnel network is connected but has loops to wander through.
fn plan_edges(rng: &mut SmallRng, caverns: &[(i32, i32, f32)]) -> Vec<(usize, usize)> {
  let d2 = |a: usize, b: usize| {
    (caverns[a].0 - caverns[b].0).pow(2) + (caverns[a].1 - caverns[b].1).pow(2)
  };
  let mut in_tree = vec![0usize];
  let mut edges = Vec::new();
  while in_tree.len() < caverns.len() {
    let (a, b) = in_tree
      .iter()
      .flat_map(|&a| {
        (0..caverns.len()).filter(|b| !in_tree.contains(b)).map(move |b| (a, b))
      })
      .min_by_key(|&(a, b)| d2(a, b))
      .unwrap();
    edges.push((a, b));
    in_tree.push(b);
  }
  for i in 0..caverns.len() {
    let near = (0..caverns.len())
      .filter(|&j| {
        j != i && !edges.contains(&(i, j)) && !edges.contains(&(j, i))
      })
      .min_by_key(|&j| d2(i, j));
    if let Some(j) = near
      && rng.gen_bool(0.30)
    {
      edges.push((i, j))
    }
  }
  edges
}

/// Wobbly walk from `from` to `to`: each step steers toward the target but
/// drifts with random jitter, and the carve width breathes as it goes. A
/// straight fallback finishes the job if the wobble runs out of steps, so the
/// tunnel always connects.
fn carve_tunnel(carved: &mut Carved, rng: &mut SmallRng, from: (i32, i32), to: (i32, i32)) {
  let (tx, ty) = (to.0 as f32 + 0.5, to.1 as f32 + 0.5);
  let (mut x, mut y) = (from.0 as f32 + 0.5, from.1 as f32 + 0.5);
  let mut heading = (ty - y).atan2(tx - x);
  let mut width: f32 = rng.gen_range(1.0..1.8);
  let mut steps = 0;
  while (tx - x).hypot(ty - y) > 2.0 && steps < 4 * carved.size {
    let bearing = (ty - y).atan2(tx - x);
    let delta = (bearing - heading + std::f32::consts::PI)
      .rem_euclid(std::f32::consts::TAU)
      - std::f32::consts::PI;
    heading += delta.clamp(-0.30, 0.30) + rng.gen_range(-0.40..0.40);
    x += heading.cos();
    y += heading.sin();
    width = (width + rng.gen_range(-0.25..0.25)).clamp(1.0, 2.4);
    carved.carve_disc(x, y, width);
    steps += 1;
  }
  let remaining = (tx - x).hypot(ty - y).ceil() as i32;
  for t in 0..=remaining {
    let f = t as f32 / remaining.max(1) as f32;
    carved.carve_disc(x + (tx - x) * f, y + (ty - y) * f, 1.2)
  }
}

/// Carve one cavern as a blob whose radius is perturbed by noise, so chambers
/// come out lumpy and asymmetric instead of circular.
fn carve_blob(carved: &mut Carved, noise: &NoiseField, cx: i32, cy: i32, r: f32) {
  let rr = (r * 1.4).ceil() as i32;
  for dy in -rr..=rr {
    for dx in -rr..=rr {
      let d = ((dx * dx + dy * dy) as f32).sqrt();
      let n = noise.sample01((cx + dx) as f64 * 0.11, (cy + dy) as f64 * 0.11) as f32;
      if d <= r * (0.65 + 0.7 * n) {
        carved.set(cx + dx, cy + dy)
      }
    }
  }
}

/// Floor-only growth: wall cells surrounded by enough floor sometimes become
/// floor. Roughens every edge without ever cutting a passage.
fn roughen(carved: &mut Carved, rng: &mut SmallRng) {
  for _ in 0..2 {
    let snapshot = carved.floor.clone();
    for y in MARGIN..carved.size - MARGIN {
      for x in MARGIN..carved.size - MARGIN {
        if !snapshot[(y * carved.size + x) as usize]
          && carved.floor_neighbors8(x, y) >= 4
          && rng.gen_bool(0.45)
        {
          carved.set(x, y)
        }
      }
    }
  }
}

fn carve_level(
  rng: &mut SmallRng,
  size: i32,
  seed: u32,
  forced: &[(i32, i32)]
) -> (Carved, Vec<(i32, i32, f32)>) {
  let blob_noise = NoiseField::new(seed.wrapping_add(77), 1.0, 3);
  let caverns = scatter_caverns(rng, size, forced);
  let mut carved = Carved::new(size);
  for &(cx, cy, r) in &caverns {
    carve_blob(&mut carved, &blob_noise, cx, cy, r)
  }
  for (a, b) in plan_edges(rng, &caverns) {
    carve_tunnel(&mut carved, rng, (caverns[a].0, caverns[a].1), (caverns[b].0, caverns[b].1))
  }
  roughen(&mut carved, rng);
  (carved, caverns)
}

/// 4-connected BFS distance from `starts` over `passable` cells; -1 = unreachable.
fn bfs_dist(size: i32, passable: impl Fn(i32, i32) -> bool, starts: &[(i32, i32)]) -> Vec<i32> {
  let mut dist = vec![-1i32; (size * size) as usize];
  let mut queue = VecDeque::new();
  for &(x, y) in starts {
    if passable(x, y) && dist[(y * size + x) as usize] < 0 {
      dist[(y * size + x) as usize] = 0;
      queue.push_back((x, y))
    }
  }
  while let Some((x, y)) = queue.pop_front() {
    for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
      let (nx, ny) = (x + dx, y + dy);
      if nx >= 0
        && ny >= 0
        && nx < size
        && ny < size
        && dist[(ny * size + nx) as usize] < 0
        && passable(nx, ny)
      {
        dist[(ny * size + nx) as usize] = dist[(y * size + x) as usize] + 1;
        queue.push_back((nx, ny))
      }
    }
  }
  dist
}

/// Greedy max-min selection: pick `count` points spread as far from each
/// other as possible.
fn pick_spread(
  rng: &mut SmallRng,
  mut candidates: Vec<(i32, i32)>,
  count: usize
) -> Vec<(i32, i32)> {
  let mut chosen: Vec<(i32, i32)> = Vec::new();
  while chosen.len() < count && !candidates.is_empty() {
    let pick = if chosen.is_empty() {
      rng.gen_range(0..candidates.len())
    } else {
      candidates
        .iter()
        .enumerate()
        .max_by_key(|&(_, &(x, y))| {
          chosen.iter().map(|&(cx, cy)| (cx - x).pow(2) + (cy - y).pow(2)).min().unwrap()
        })
        .unwrap()
        .0
    };
    chosen.push(candidates.swap_remove(pick))
  }
  chosen
}

/// Nearest walkable, entry-reachable cell to (x, y) within a small box, for
/// snapping cavern centers that decoration may have covered with liquid.
fn snap_reachable(level: &Level, dist: &[i32], x: i32, y: i32) -> Option<(i32, i32)> {
  let size = level.width as i32;
  (-3..=3)
    .flat_map(|dy| (-3..=3).map(move |dx| (x + dx, y + dy)))
    .filter(|&(nx, ny)| {
      nx >= 0
        && ny >= 0
        && nx < size
        && ny < size
        && level.walkable(nx, ny)
        && dist[(ny * size + nx) as usize] >= 0
    })
    .min_by_key(|&(nx, ny)| (nx - x).pow(2) + (ny - y).pow(2))
}

/// Surface cells above the first cave level that can host an entrance: dry,
/// walkable ground sitting over carved floor.
fn surface_entrances(rng: &mut SmallRng, loc: &Location, carved: &Carved) -> Vec<(i32, i32)> {
  let surface = loc.level(0);
  let size = carved.size;
  let candidates: Vec<(i32, i32)> = (MARGIN + 3..size - MARGIN - 3)
    .flat_map(|y| (MARGIN + 3..size - MARGIN - 3).map(move |x| (x, y)))
    .filter(|&(x, y)| {
      carved.get(x, y)
        && surface
          .get(x, y)
          .is_some_and(|t| t.walkable() && !t.is_liquid() && t != Tile::ShipDock)
    })
    .collect();
  let count = rng.gen_range(2..=4);
  pick_spread(rng, candidates, count)
}

/// Write tiles and decoration for one carved level. Returns objects to spawn
/// (the caller adds the z coordinate).
fn build_level(
  level: &mut Level,
  carved: &Carved,
  theme: CaveTheme,
  seed: u32,
  rng: &mut SmallRng
) -> Vec<(i32, i32, Object)> {
  let size = carved.size;
  let pools = NoiseField::new(seed, 0.05, 3);
  let detail = NoiseField::new(seed.wrapping_add(1), 0.18, 2);
  let (pool_tile, pool_thr) = theme.pool();

  // Distance from the nearest wall, for keeping deep liquid off the shores.
  let shore: Vec<(i32, i32)> = (0..size)
    .flat_map(|y| (0..size).map(move |x| (x, y)))
    .filter(|&(x, y)| {
      carved.get(x, y)
        && [(1, 0), (-1, 0), (0, 1), (0, -1)]
          .iter()
          .any(|&(dx, dy)| !carved.get(x + dx, y + dy))
    })
    .collect();
  let wall_dist = bfs_dist(size, |x, y| carved.get(x, y), &shore);

  let mut spawns = Vec::new();
  for y in 0..size {
    for x in 0..size {
      let tile = if !carved.get(x, y) {
        Tile::CaveWall
      } else {
        let p = pools.sample01(x as f64, y as f64);
        let d = detail.sample01(x as f64, y as f64);
        let deep = theme
          .deep_pool()
          .filter(|_| p > pool_thr + 0.10 && wall_dist[(y * size + x) as usize] >= 2);
        deep.unwrap_or(if p > pool_thr {
          pool_tile
        } else {
          match theme {
            CaveTheme::Rocky if d > 0.82 => Tile::SmallRocks,
            CaveTheme::Rocky if d < 0.08 => Tile::Ground,
            CaveTheme::Flooded if d > 0.85 => Tile::Sand,
            CaveTheme::Molten if d > 0.84 => Tile::SmallRocks,
            CaveTheme::Molten if d < 0.10 => Tile::Ash,
            _ => Tile::CaveFloor
          }
        })
      };
      level.set(x, y, tile);

      if tile.walkable() && !tile.is_liquid() {
        let mushrooms: &[(u32, fn() -> Object)] = &[
          (3, || {
            Object::mushroom(
              Color::srgb(0.16, 0.55, 0.48),
              Color::srgb(0.45, 0.95, 0.78),
              "Glowcap"
            )
          }),
          (2, || {
            Object::mushroom(
              Color::srgb(0.50, 0.28, 0.60),
              Color::srgb(0.82, 0.58, 0.95),
              "Veilshroom"
            )
          }),
          (2, || {
            Object::mushroom(
              Color::srgb(0.55, 0.42, 0.20),
              Color::srgb(0.88, 0.74, 0.42),
              "Bruisegill"
            )
          })
        ];
        let d = detail.sample01(x as f64 + 900.0, y as f64 + 400.0);
        match theme {
          CaveTheme::Fungal if d > 0.62 && rng.gen_bool(0.06) => {
            spawns.push((x, y, weighted(rng, mushrooms)()))
          }
          CaveTheme::Fungal if rng.gen_bool(0.003) => {
            spawns.push((x, y, Object::ground_item(Item::Mushroom)))
          }
          CaveTheme::Crystal if d > 0.70 && rng.gen_bool(0.005) => {
            spawns.push((x, y, Object::ground_item(Item::Crystal)))
          }
          CaveTheme::Rocky | CaveTheme::Molten
            if wall_dist[(y * size + x) as usize] >= 2 && rng.gen_bool(0.006) =>
          {
            spawns.push((x, y, Object::BOULDER))
          }
          _ => {}
        }
      }
    }
  }

  // Crystal veins: walls bordering floor turn to (sight-passing) crystal.
  if theme == CaveTheme::Crystal {
    for &(x, y) in &shore {
      for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
        if !carved.get(x + dx, y + dy)
          && level.get(x + dx, y + dy) == Some(Tile::CaveWall)
          && rng.gen_bool(0.22)
        {
          level.set(
            x + dx,
            y + dy,
            if rng.gen_bool(0.5) { Tile::CrystalFormation } else { Tile::CrystalGrowth }
          )
        }
      }
    }
  }

  spawns
}

/// Creatures and loot for one level, placed by BFS distance from its entry
/// points: nothing hostile near the stairs, the best loot in the far caverns.
fn populate(
  level: &Level,
  theme: CaveTheme,
  dist: &[i32],
  caverns: &[(i32, i32, f32)],
  depth: usize,
  rng: &mut SmallRng
) -> Vec<(i32, i32, Object)> {
  let size = level.width as i32;
  let mut spawns = Vec::new();

  let mut lairs: Vec<(i32, i32)> = (0..size)
    .flat_map(|y| (0..size).map(move |x| (x, y)))
    .filter(|&(x, y)| dist[(y * size + x) as usize] >= SAFE_RADIUS && level.walkable(x, y))
    .collect();
  let reachable = lairs.len();
  let mut remaining = (reachable / 450).clamp(6, 24) + depth * 2;
  while remaining > 0 && !lairs.is_empty() {
    let (x, y) = lairs.swap_remove(rng.gen_range(0..lairs.len()));
    spawns.push((x, y, weighted(rng, theme.creatures())()));
    remaining -= 1;
  }

  let mut hoards: Vec<((i32, i32), i32)> = caverns
    .iter()
    .filter_map(|&(cx, cy, _)| snap_reachable(level, dist, cx, cy))
    .filter(|&(x, y)| dist[(y * size + x) as usize] >= 10)
    .map(|(x, y)| ((x, y), dist[(y * size + x) as usize]))
    .collect();
  hoards.sort_by_key(|&(_, d)| -d);
  hoards.dedup();
  let chest_count = (caverns.len() / 3).clamp(2, 6);
  for (i, &((x, y), _)) in hoards.iter().take(chest_count).enumerate() {
    let object = if i == 0 && depth >= 2 {
      Object::supply_cache(if depth >= 3 { CACHE_DEEP } else { CACHE_MID })
    } else {
      Object::LOOT_CHEST
    };
    spawns.push((x, y, object))
  }

  spawns
}

/// Entry point: carve a stack of themed cave levels under `loc`, growing
/// `loc.levels` as needed, and wire surface entrances plus inter-level
/// passages into `loc.spawn_objects`.
pub fn generate_caves(loc: &mut Location, seed: u64) {
  let size = loc.width as i32;
  let mut rng = SmallRng::seed_from_u64(seed ^ 0x5EED_CAFE);
  let level_count = *weighted(&mut rng, &[(3, 2usize), (4, 3), (2, 4)]);

  let (first_carved, first_caverns) = carve_level(&mut rng, size, seed as u32, &[]);
  let entrances = surface_entrances(&mut rng, loc, &first_carved);
  if !entrances.is_empty() {
    while loc.levels.len() < 1 + level_count {
      loc.levels.push(Level::new(loc.width, loc.height, Tile::CaveWall))
    }
    loc.depth = loc.levels.len();
    for &(x, y) in &entrances {
      loc.spawn_objects.push((x, y, 0, Object::cave_entrance(x, y, x, y)));
      loc.spawn_objects.push((x, y, 1, Object::cave_exit(x, y, x, y)));
    }

    let mut pending = Some((first_carved, first_caverns));
    let mut entry_points = entrances;
    let mut prev_theme = None;
    for depth in 1..=level_count {
      let level_seed = (seed as u32).wrapping_add(depth as u32 * 7919);
      let (carved, caverns) = pending
        .take()
        .unwrap_or_else(|| carve_level(&mut rng, size, level_seed, &entry_points));
      let theme = CaveTheme::pick(&mut rng, depth, prev_theme);
      prev_theme = Some(theme);

      let decor = build_level(loc.level_mut(depth), &carved, theme, level_seed, &mut rng);
      loc.spawn_objects.extend(decor.into_iter().map(|(x, y, o)| (x, y, depth, o)));

      // Guarantee a clear landing under every stairway into this level.
      for &(ex, ey) in &entry_points {
        let level = loc.level_mut(depth);
        for dy in -1..=1 {
          for dx in -1..=1 {
            if !level.walkable(ex + dx, ey + dy) {
              level.set(ex + dx, ey + dy, Tile::CaveFloor)
            }
          }
        }
      }

      let dist = {
        let level = loc.level(depth);
        bfs_dist(size, |x, y| level.walkable(x, y), &entry_points)
      };
      let life = populate(loc.level(depth), theme, &dist, &caverns, depth, &mut rng);
      loc.spawn_objects.extend(life.into_iter().map(|(x, y, o)| (x, y, depth, o)));

      if depth < level_count {
        let far_centers: Vec<(i32, i32)> = caverns
          .iter()
          .filter_map(|&(cx, cy, _)| snap_reachable(loc.level(depth), &dist, cx, cy))
          .filter(|&(x, y)| dist[(y * size + x) as usize] >= 20)
          .collect();
        let shafts = 1 + usize::from(rng.gen_bool(0.6));
        entry_points = pick_spread(&mut rng, far_centers, shafts);
        for &(x, y) in &entry_points {
          loc.spawn_objects.push((x, y, depth, Object::passage_down(depth, x, y, x, y)));
          loc.spawn_objects.push((x, y, depth + 1, Object::passage_up(depth, x, y, x, y)));
        }
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use {super::*, crate::level::LocationType};

  #[test]
  fn carves_connected_themed_levels() {
    let mut loc = Location::new(
      "Test Caves",
      120,
      120,
      2,
      LocationType::PlanetSurface { breathable: true },
      Tile::Grass
    );
    generate_caves(&mut loc, 0x1234_5678);

    let underground = loc.levels.len() - 1;
    assert!((2..=4).contains(&underground), "expected 2-4 cave levels, got {underground}");

    // Index every stairway and loot spot by level.
    let mut elevators: Vec<Vec<(i32, i32)>> = vec![Vec::new(); loc.levels.len()];
    let mut loot: Vec<Vec<(i32, i32)>> = vec![Vec::new(); loc.levels.len()];
    let mut creatures = 0;
    for &(x, y, z, ref obj) in &loc.spawn_objects {
      if Has::<Elevator>::get(obj).is_some() {
        elevators[z].push((x, y))
      } else if Has::<LootChest>::get(obj).is_some() {
        loot[z].push((x, y))
      } else if Has::<Enemy>::get(obj).is_some() {
        creatures += 1
      }
    }
    assert!(!elevators[0].is_empty(), "expected surface entrances");
    assert!(creatures > 0, "expected cave creatures");

    // Every level must reach all its stairways and loot from its stairways.
    for z in 1..loc.levels.len() {
      let level = loc.level(z);
      let size = level.width as i32;
      let dist = bfs_dist(size, |x, y| level.walkable(x, y), &elevators[z]);
      let reachable = |&(x, y): &(i32, i32)| dist[(y * size + x) as usize] >= 0;
      assert!(
        elevators[z].iter().all(reachable),
        "level {z}: stairways disconnected at {:?}",
        elevators[z]
      );
      assert!(
        loot[z].iter().all(reachable),
        "level {z}: unreachable loot at {:?}",
        loot[z]
      );
      assert!(!loot[z].is_empty(), "level {z}: expected loot");

      let floor = (0..size)
        .flat_map(|y| (0..size).map(move |x| (x, y)))
        .filter(|&(x, y)| level.walkable(x, y))
        .count();
      let mut render = String::new();
      for y in (0..size).step_by(2) {
        for x in (0..size).step_by(2) {
          render.push(match level.get(x, y).unwrap() {
            Tile::CaveWall => '▓',
            t if t.is_liquid() && !t.walkable() => '≈',
            t if t.is_liquid() => '~',
            t if !t.walkable() => '*',
            _ => '.'
          })
        }
        render.push('\n')
      }
      eprintln!(
        "level {z}: floor {floor}  stairways {}  loot {}\n{render}",
        elevators[z].len(),
        loot[z].len()
      );
    }
  }
}
