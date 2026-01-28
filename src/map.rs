use crate::components::{Position, Tile, TileType};
use rand::Rng;

pub struct Map {
    pub tiles: Vec<Vec<Tile>>,
    pub width: usize,
    pub height: usize,
}

impl Map {
    pub fn generate_cavern(width: usize, height: usize) -> Self {
        let mut tiles = vec![vec![Tile::new(TileType::Wall); width]; height];
        let mut smoothed = vec![vec![false; width]; height];

        let mut rng = rand::thread_rng();

        for y in 0..height {
            for x in 0..width {
                if rng.gen_bool(0.45) {
                    smoothed[y][x] = true;
                }
            }
        }

        for _ in 0..4 {
            smoothed = smooth_cave(&smoothed, width, height);
        }

        for y in 0..height {
            for x in 0..width {
                if smoothed[y][x] {
                    tiles[y][x] = Tile::new(TileType::Floor);

                    let rand_val = rng.gen::<f64>();
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

        for _ in 0..5 {
            let x = rng.gen_range(1..width - 1);
            let y = rng.gen_range(1..height - 1);

            if tiles[y][x].tile_type == TileType::Floor {
                tiles[y][x] = Tile::new(TileType::StairsDown);
                break;
            }
        }

        for _ in 0..3 {
            let x = rng.gen_range(1..width - 1);
            let y = rng.gen_range(1..height - 1);

            if tiles[y][x].tile_type == TileType::Floor {
                tiles[y][x] = Tile::new(TileType::StairsUp);
                break;
            }
        }

        Map {
            tiles,
            width,
            height,
        }
    }

    pub fn find_start_position(&self) -> Position {
        let mut rng = rand::thread_rng();

        for _ in 0..1000 {
            let x = rng.gen_range(1..self.width - 1);
            let y = rng.gen_range(1..self.height - 1);

            if self.tiles[y][x].tile_type == TileType::Floor {
                return Position { x: x as i32, y: y as i32 };
            }
        }

        Position { x: 1, y: 1 }
    }

    pub fn find_exit_position(&self) -> Position {
        let mut rng = rand::thread_rng();

        for _ in 0..1000 {
            let x = rng.gen_range(1..self.width - 1);
            let y = rng.gen_range(1..self.height - 1);

            if self.tiles[y][x].tile_type == TileType::StairsUp {
                return Position { x: x as i32, y: y as i32 };
            }
        }

        self.find_start_position()
    }
}

fn smooth_cave(grid: &Vec<Vec<bool>>, width: usize, height: usize) -> Vec<Vec<bool>> {
    let mut new_grid = vec![vec![false; width]; height];

    for y in 0..height {
        for x in 0..width {
            let mut neighbors = 0;

            for dy in -1..=1 {
                for dx in -1..=1 {
                    let nx = x as i32 + dx;
                    let ny = y as i32 + dy;

                    if nx >= 0 && ny >= 0 && nx < width as i32 && ny < height as i32 {
                        if grid[ny as usize][nx as usize] {
                            neighbors += 1;
                        }
                    }
                }
            }

            new_grid[y][x] = neighbors > 4;
        }
    }

    new_grid
}