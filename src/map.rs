use {crate::components::{Position, Tile, TileType},
     rand::{Rng, SeedableRng, rngs::StdRng}};

pub struct Map {
  pub tiles: Vec<Vec<Vec<Tile>>>,
  pub width: usize,
  pub height: usize,
  pub depth: usize
}

impl Map {
  pub fn generate_world(width: usize, height: usize, depth: usize) -> Self {
    let seed: u64 = rand::random();
    let mut tiles = vec![vec![vec![Tile::new(TileType::Wall); width]; height]; depth];

    tiles[0] = Self::generate_surface_level(width, height, seed);

    for z in 1..depth {
      tiles[z] = Self::generate_cavern_level(width, height, seed + z as u64);
    }

    Self::connect_levels(&mut tiles, width, height, depth);

    Map { tiles, width, height, depth }
  }

  fn generate_surface_level(width: usize, height: usize, seed: u64) -> Vec<Vec<Tile>> {
    let mut rng = StdRng::seed_from_u64(seed);
    let surface_height = Self::generate_surface_height(width, height, &mut rng);
    let mut tiles = Self::create_base_world(width, height, &surface_height);
    Self::carve_caves(&mut tiles, width, height, &mut rng);
    Self::place_trees(&mut tiles, width, &surface_height, &mut rng);
    Self::place_lakes(&mut tiles, width, &surface_height, &mut rng);
    tiles
  }

  fn generate_surface_height(width: usize, height: usize, rng: &mut StdRng) -> Vec<usize> {
    let mut height_map = vec![0; width];
    let seed: u64 = rng.random();

    for x in 0..width {
      let noise_val = Self::perlin_noise(x as f64 * 0.1, seed as f64);
      let height_offset = ((noise_val + 1.0) * 0.5 * 15.0) as usize;
      height_map[x] = height - 10 - height_offset.min(20);
    }

    height_map
  }

  fn create_base_world(
    width: usize,
    height: usize,
    surface_height: &[usize]
  ) -> Vec<Vec<Tile>> {
    let mut tiles = vec![vec![Tile::new(TileType::Wall); width]; height];

    for x in 0..width {
      let surface = surface_height[x];

      for y in surface..height {
        let tile_type = if y == surface {
          TileType::Grass
        } else if y < surface + 5 {
          TileType::Sand
        } else {
          TileType::Floor
        };

        tiles[y][x] = Tile::new(tile_type);
      }
    }

    tiles
  }

  fn carve_caves(tiles: &mut [Vec<Tile>], width: usize, height: usize, rng: &mut StdRng) {
    let mut cave_map = vec![vec![false; width]; height];

    for y in 0..height {
      for x in 0..width {
        if rng.random_bool(0.12) && tiles[y][x].tile_type == TileType::Floor {
          cave_map[y][x] = true;
        }
      }
    }

    for _ in 0..5 {
      cave_map = Self::smooth_cave(&cave_map, width, height);
    }

    for y in 5..height - 5 {
      for x in 3..width - 3 {
        if cave_map[y][x] {
          tiles[y][x] = Tile::new(TileType::Wall);
        }
      }
    }
  }

  fn smooth_cave(grid: &[Vec<bool>], width: usize, height: usize) -> Vec<Vec<bool>> {
    let mut new_grid = vec![vec![false; width]; height];

    for y in 1..height - 1 {
      for x in 1..width - 1 {
        let mut neighbors = 0;

        for dy in -1..=1 {
          for dx in -1..=1 {
            let nx = (x as i32 + dx) as usize;
            let ny = (y as i32 + dy) as usize;
            if grid[ny][nx] {
              neighbors += 1;
            }
          }
        }

        new_grid[y][x] = neighbors > 4;
      }
    }

    new_grid
  }

  fn place_trees(
    tiles: &mut [Vec<Tile>],
    width: usize,
    surface_height: &[usize],
    rng: &mut StdRng
  ) {
    for x in 2..width - 2 {
      let surface = surface_height[x];

      if rng.random_bool(0.08) && tiles[surface][x].tile_type == TileType::Grass {
        let has_space = (x.saturating_sub(1)..=(x + 1).min(width - 1))
          .all(|check_x| tiles[surface][check_x].tile_type == TileType::Grass);

        if has_space {
          tiles[surface][x] = Tile::new(TileType::Wall);
        }
      }
    }
  }

  fn place_lakes(
    tiles: &mut [Vec<Tile>],
    width: usize,
    surface_height: &[usize],
    rng: &mut StdRng
  ) {
    let lake_count = rng.random_range(2..=4);

    for _ in 0..lake_count {
      let lake_x = rng.random_range(5..width - 5);
      let lake_width = rng.random_range(3..=6);
      let lake_depth = rng.random_range(2..=4);

      for dx in 0..lake_width {
        let x = (lake_x + dx).min(width - 1);
        let surface = surface_height[x];

        for dy in 0..lake_depth {
          let y = (surface + dy).min(tiles.len() - 1);
          if tiles[y][x].tile_type == TileType::Grass
            || tiles[y][x].tile_type == TileType::Sand
          {
            tiles[y][x] = Tile::new(TileType::Water);
          }
        }
      }
    }
  }

  fn perlin_noise(x: f64, seed: f64) -> f64 {
    let x_scaled = x + seed * 0.1;
    (x_scaled.sin() + (x_scaled * 2.5).sin() * 0.5 + (x_scaled * 5.0).sin() * 0.25) / 1.75
  }

  fn generate_cavern_level(width: usize, height: usize, seed: u64) -> Vec<Vec<Tile>> {
    let mut rng = StdRng::seed_from_u64(seed);
    let mut tiles = vec![vec![Tile::new(TileType::Wall); width]; height];
    let mut smoothed = vec![vec![false; width]; height];

    for y in 0..height {
      for x in 0..width {
        if rng.random_bool(0.45) {
          smoothed[y][x] = true;
        }
      }
    }

    for _ in 0..4 {
      smoothed = Self::smooth_cave(&smoothed, width, height);
    }

    for y in 0..height {
      for x in 0..width {
        if smoothed[y][x] {
          tiles[y][x] = Tile::new(TileType::Floor);

          let rand_val: f64 = rng.random();
          if rand_val < 0.03 {
            tiles[y][x] = Tile::new(TileType::Water);
          } else if rand_val < 0.08 {
            tiles[y][x] = Tile::new(TileType::Grass);
          } else if rand_val < 0.11 {
            tiles[y][x] = Tile::new(TileType::Sand);
          }
        }
      }
    }

    tiles
  }

  fn connect_levels(
    tiles: &mut [Vec<Vec<Tile>>],
    width: usize,
    height: usize,
    depth: usize
  ) {
    let mut rng = rand::thread_rng();

    for z in 0..depth - 1 {
      let mut stairs_placed = false;
      for _ in 0..100 {
        let x = rng.random_range(1..width - 1);
        let y = rng.random_range(1..height - 1);

        if tiles[z][y][x].tile_type == TileType::Floor {
          tiles[z][y][x] = Tile::new(TileType::StairsDown);
          tiles[z + 1][y][x] = Tile::new(TileType::StairsUp);
          stairs_placed = true;
          break;
        }
      }

      if !stairs_placed {
        for y in 1..height - 1 {
          for x in 1..width - 1 {
            if tiles[z][y][x].tile_type == TileType::Floor {
              tiles[z][y][x] = Tile::new(TileType::StairsDown);
              tiles[z + 1][y][x] = Tile::new(TileType::StairsUp);
              break;
            }
          }
        }
      }
    }

    for _ in 0..3 {
      let z = rng.random_range(1..depth);
      for _ in 0..100 {
        let x = rng.random_range(1..width - 1);
        let y = rng.random_range(1..height - 1);

        if tiles[z][y][x].tile_type == TileType::Floor {
          tiles[z][y][x] = Tile::new(TileType::StairsUp);
          break;
        }
      }
    }
  }

  pub fn find_start_position(&self, z: usize) -> Position {
    let mut rng = rand::thread_rng();

    for _ in 0..1000 {
      let x = rng.random_range(1..self.width - 1);
      let y = rng.random_range(1..self.height - 1);

      if self.tiles[z][y][x].tile_type == TileType::Floor {
        return Position { x: x as i32, y: y as i32 };
      }
    }

    Position { x: 1, y: 1 }
  }

  pub fn find_exit_position(&self, z: usize) -> Position {
    let mut rng = rand::thread_rng();

    for _ in 0..1000 {
      let x = rng.random_range(1..self.width - 1);
      let y = rng.random_range(1..self.height - 1);

      if self.tiles[z][y][x].tile_type == TileType::StairsUp {
        return Position { x: x as i32, y: y as i32 };
      }
    }

    self.find_start_position(z)
  }

  pub fn get_tile(&self, x: i32, y: i32, z: usize) -> Option<&Tile> {
    if x < 0 || y < 0 || z >= self.depth {
      return None;
    }
    let x = x as usize;
    let y = y as usize;
    if x >= self.width || y >= self.height {
      return None;
    }
    Some(&self.tiles[z][y][x])
  }

  pub fn get_tile_mut(&mut self, x: i32, y: i32, z: usize) -> Option<&mut Tile> {
    if x < 0 || y < 0 || z >= self.depth {
      return None;
    }
    let x = x as usize;
    let y = y as usize;
    if x >= self.width || y >= self.height {
      return None;
    }
    Some(&mut self.tiles[z][y][x])
  }
}
