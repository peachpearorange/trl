use {crate::{components::{Position, TileType},
             map::Map,
             tile_loader::EntityFaction},
     rand::Rng};

pub struct Game {
  pub state: crate::components::GameState,
  pub maps: Vec<Map>,
  pub current_map_idx: usize
}

impl Game {
  pub fn new() -> Self {
    let mut game = Game {
      state: crate::components::GameState::new(),
      maps: Vec::new(),
      current_map_idx: 0
    };

    let first_map = Map::generate_cavern(80, 24);
    game.maps.push(first_map);

    let start_pos = game.maps[0].find_start_position();
    game.state.player.pos = start_pos;

    game.spawn_enemies();

    game
  }

  fn spawn_enemies(&mut self) {
    let map = &self.maps[self.current_map_idx];
    let mut rng = rand::thread_rng();

    let enemies = (0..5)
      .filter_map(|_| {
        let pos = map.find_start_position();
        let enemy_type = rng.gen_range(0..3);
        let (name, sprite, faction, char) = match enemy_type {
          0 => ("Goblin", "goblin", EntityFaction::Hostile, 'g'),
          1 => ("Slime", "slime", EntityFaction::Hostile, 's'),
          _ => ("Dragon", "dragon", EntityFaction::Hostile, 'D')
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

    let map = &self.maps[self.current_map_idx];

    if new_x < 0 || new_y < 0 || new_x >= map.width as i32 || new_y >= map.height as i32 {
      return;
    }

    let tile = map.tiles[new_y as usize][new_x as usize];

    if tile.tile_type != TileType::Wall && tile.tile_type != TileType::Water {
      self.state.player.pos.x = new_x;
      self.state.player.pos.y = new_y;
    }

    self.update_fov();
  }

  pub fn descend(&mut self) {
    let map = &self.maps[self.current_map_idx];
    let tile = map.tiles[self.state.player.pos.y as usize][self.state.player.pos.x as usize];

    if tile.tile_type == TileType::StairsDown {
      if self.current_map_idx == self.maps.len() - 1 {
        let new_map = Map::generate_cavern(80, 24);
        self.maps.push(new_map);
      }

      self.current_map_idx += 1;
      self.state.depth += 1;

      let start_pos = self.maps[self.current_map_idx].find_start_position();
      self.state.player.pos = start_pos;

      self.update_fov();
    }
  }

  pub fn ascend(&mut self) {
    let map = &self.maps[self.current_map_idx];
    let tile = map.tiles[self.state.player.pos.y as usize][self.state.player.pos.x as usize];

    if tile.tile_type == TileType::StairsUp && self.current_map_idx > 0 {
      self.current_map_idx -= 1;
      self.state.depth -= 1;

      let start_pos = self.maps[self.current_map_idx].find_exit_position();
      self.state.player.pos = start_pos;

      self.update_fov();
    }
  }

  fn update_fov(&mut self) {
    let map = &mut self.maps[self.current_map_idx];
    let player_pos = self.state.player.pos;

    for y in 0..map.height {
      for x in 0..map.width {
        map.tiles[y][x].visible = false;
      }
    }

    for dy in -6..=6 {
      for dx in -6..=6 {
        let x = player_pos.x + dx;
        let y = player_pos.y + dy;

        if x >= 0 && y >= 0 && x < map.width as i32 && y < map.height as i32 {
          map.tiles[y as usize][x as usize].visible = true;
          map.tiles[y as usize][x as usize].revealed = true;
        }
      }
    }
  }

  pub fn current_map(&self) -> &Map { &self.maps[self.current_map_idx] }
}

impl Default for Game {
  fn default() -> Self { Self::new() }
}
