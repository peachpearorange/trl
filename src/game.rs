use {crate::{components::{Position, TileType},
             map::Map,
             tile_loader::Faction},
     rand::Rng};

pub struct Game {
  pub state: crate::components::GameState,
  pub map: Map,
  pub current_z: usize
}

impl Game {
  pub fn new() -> Self {
    let map = Map::generate_world(80, 50, 10);
    let current_z = 0;

    let start_pos = map.find_start_position(current_z);

    let mut game = Game { state: crate::components::GameState::new(), map, current_z };

    game.state.player.pos = start_pos;
    game.spawn_enemies();

    game
  }

  fn spawn_enemies(&mut self) {
    let mut rng = rand::thread_rng();

    let enemies = (0..5)
      .filter_map(|_| {
        let pos = self.map.find_start_position(self.current_z);
        let enemy_type = rng.random_range(0..3);
        let (name, sprite, faction, char) = match enemy_type {
          0 => ("Goblin", "goblin", Faction::Hostile, 'g'),
          1 => ("Slime", "slime", Faction::Hostile, 's'),
          _ => ("Dragon", "dragon", Faction::Hostile, 'D')
        };

        Some(crate::components::Entity::new(
          Position { x: pos.x, y: pos.y },
          char,
          name,
          sprite,
          faction,
          true
        ))
      })
      .collect();

    self.state.entities = enemies;
  }

  pub fn player_move(&mut self, dx: i32, dy: i32) {
    let new_x = self.state.player.pos.x + dx;
    let new_y = self.state.player.pos.y + dy;

    if let Some(tile) = self.map.get_tile(new_x, new_y, self.current_z) {
      if tile.tile_type != TileType::Wall && tile.tile_type != TileType::Water {
        self.state.player.pos.x = new_x;
        self.state.player.pos.y = new_y;
      }
    }

    self.update_fov();
  }

  pub fn descend(&mut self) {
    if let Some(tile) =
      self.map.get_tile(self.state.player.pos.x, self.state.player.pos.y, self.current_z)
    {
      if tile.tile_type == TileType::StairsDown && self.current_z < self.map.depth - 1 {
        self.current_z += 1;
        self.state.depth += 1;

        let start_pos = self.map.find_start_position(self.current_z);
        self.state.player.pos = start_pos;

        self.spawn_enemies();
        self.update_fov();
      }
    }
  }

  pub fn ascend(&mut self) {
    if let Some(tile) =
      self.map.get_tile(self.state.player.pos.x, self.state.player.pos.y, self.current_z)
    {
      if tile.tile_type == TileType::StairsUp && self.current_z > 0 {
        self.current_z -= 1;
        self.state.depth -= 1;

        let start_pos = self.map.find_exit_position(self.current_z);
        self.state.player.pos = start_pos;

        self.spawn_enemies();
        self.update_fov();
      }
    }
  }

  fn update_fov(&mut self) {
    let player_pos = self.state.player.pos;

    for y in 0..self.map.height {
      for x in 0..self.map.width {
        self.map.tiles[self.current_z][y][x].visible = false;
      }
    }

    for dy in -6..=6 {
      for dx in -6..=6 {
        let x = player_pos.x + dx;
        let y = player_pos.y + dy;

        if x >= 0 && y >= 0 {
          let x = x as usize;
          let y = y as usize;
          if x < self.map.width && y < self.map.height {
            self.map.tiles[self.current_z][y][x].visible = true;
            self.map.tiles[self.current_z][y][x].revealed = true;
          }
        }
      }
    }
  }

  pub fn current_level_tiles(&self) -> &Vec<Vec<crate::components::Tile>> {
    &self.map.tiles[self.current_z]
  }
}

impl Default for Game {
  fn default() -> Self { Self::new() }
}
