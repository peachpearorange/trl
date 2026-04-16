use {serde::{Deserialize, Serialize},
     std::collections::HashMap};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Color {
  White,
  Black,
  Red,
  Green,
  Yellow,
  Blue,
  Cyan,
  Grey
}

#[derive(Clone, Debug)]
pub struct PixelArt {
  pub pixels: Vec<Vec<char>>,
  pub width: usize,
  pub height: usize
}

impl PixelArt {
  pub fn from_text(text: &str) -> Self {
    let pixels: Vec<Vec<char>> = text
      .lines()
      .filter(|line| !line.is_empty())
      .map(|line| line.chars().collect())
      .collect();

    let height = pixels.len();
    let width = pixels.first().map_or(0, |row| row.len());

    PixelArt { pixels, width, height }
  }

  pub fn is_empty(&self, x: usize, y: usize) -> bool {
    if y >= self.height || x >= self.width {
      return true;
    }
    let ch = self.pixels[y][x];
    ch == ' ' || ch == '.'
  }

  pub fn get_char(&self, x: usize, y: usize) -> Option<char> {
    if y >= self.height || x >= self.width {
      return None;
    }
    Some(self.pixels[y][x])
  }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Faction {
  Player,
  Friendly,
  Hostile,
  Neutral
}

#[derive(Clone, Debug)]
pub struct Sprite {
  pub art: PixelArt,
  pub char: char
}

impl Sprite {
  pub fn new(art: PixelArt, char: char) -> Self { Sprite { art, char } }

  pub fn get_color(&self, faction: Faction, ch: char) -> Color {
    match (faction, ch) {
      (Faction::Player, '@') => Color::White,
      (Faction::Player, _) => Color::Cyan,
      (Faction::Friendly, _) => Color::Green,
      (Faction::Hostile, _) => Color::Red,
      (Faction::Neutral, _) => Color::Yellow
    }
  }

  pub fn to_rgba(&self, color: Color) -> [u8; 4] {
    match color {
      Color::White => [255, 255, 255, 255],
      Color::Black => [0, 0, 0, 255],
      Color::Red => [255, 0, 0, 255],
      Color::Green => [0, 255, 0, 255],
      Color::Yellow => [255, 255, 0, 255],
      Color::Blue => [0, 0, 255, 255],
      Color::Cyan => [0, 255, 255, 255],
      Color::Grey => [128, 128, 128, 255]
    }
  }
}

pub struct TileLoader {
  cache: HashMap<String, PixelArt>
}

impl TileLoader {
  pub fn new() -> Self { TileLoader { cache: HashMap::new() } }

  pub fn load(&mut self, name: &str) -> Option<PixelArt> {
    if let Some(cached) = self.cache.get(name) {
      return Some(cached.clone());
    }

    let path = format!("tiles/{}.sprite", name);

    match std::fs::read_to_string(&path) {
      Ok(content) => {
        let art = PixelArt::from_text(&content);
        self.cache.insert(name.to_string(), art.clone());
        Some(art)
      }
      Err(_) => None
    }
  }

  pub fn load_or_single(&mut self, name: &str, fallback_char: char) -> Sprite {
    match self.load(name) {
      Some(art) => Sprite::new(art, fallback_char),
      None => {
        let art = PixelArt::from_text(&fallback_char.to_string());
        Sprite::new(art, fallback_char)
      }
    }
  }
}

impl Default for TileLoader {
  fn default() -> Self { Self::new() }
}
