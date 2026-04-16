use {crate::tile_loader::Faction,
     serde::{Deserialize, Serialize}};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TileType {
  Floor,
  Wall,
  StairsDown,
  StairsUp,
  Water,
  Grass,
  Sand
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tile {
  pub tile_type: TileType,
  pub revealed: bool,
  pub visible: bool
}

impl Tile {
  pub fn new(tile_type: TileType) -> Self {
    Tile { tile_type, revealed: false, visible: false }
  }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Position {
  pub x: i32,
  pub y: i32
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Entity {
  pub pos: Position,
  pub char: char,
  pub name: String,
  pub blocks: bool,
  pub hp: i32,
  pub max_hp: i32,
  pub sprite_name: String,
  pub faction: Faction
}

impl Entity {
  pub fn new(
    pos: Position,
    char: char,
    name: &str,
    sprite_name: &str,
    faction: Faction,
    blocks: bool
  ) -> Self {
    Entity {
      pos,
      char,
      name: name.to_string(),
      sprite_name: sprite_name.to_string(),
      faction,
      blocks,
      hp: 10,
      max_hp: 10
    }
  }
}

#[derive(Clone, Debug)]
pub struct GameState {
  pub depth: u32,
  pub player: Entity,
  pub entities: Vec<Entity>
}

impl GameState {
  pub fn new() -> Self {
    GameState {
      depth: 1,
      player: Entity::new(
        Position { x: 1, y: 1 },
        '@',
        "Player",
        "player",
        Faction::Player,
        true
      ),
      entities: Vec::new()
    }
  }
}
