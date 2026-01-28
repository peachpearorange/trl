use {image::{ImageBuffer, Rgba, RgbaImage},
     serde::{Deserialize, Serialize},
     std::{collections::HashMap, fs}};

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
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum EntityFaction {
  Player,
  Friendly,
  Hostile,
  Neutral
}

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
pub struct Sprite {
  pub art: PixelArt,
  pub char: char
}

impl Sprite {
  pub fn new(art: PixelArt, char: char) -> Self { Sprite { art, char } }

  pub fn get_color(&self, faction: EntityFaction, _ch: char) -> Color {
    match faction {
      EntityFaction::Player => Color::White,
      EntityFaction::Friendly => Color::Green,
      EntityFaction::Hostile => Color::Red,
      EntityFaction::Neutral => Color::Yellow
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
       Some(cached.clone())
    }
    else if let Ok(content)=fs::read_to_string(&format!("tiles/{name}.sprite")){
        let art = PixelArt::from_text(&content);
        self.cache.insert(name.to_string(), art.clone());
        Some(art)
    }
    else {None}
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

pub struct TextureGenerator {
  tile_loader: TileLoader
}

impl TextureGenerator {
  pub fn new() -> Self { TextureGenerator { tile_loader: TileLoader::new() } }

  pub fn generate_all_textures(&mut self) {
    self.generate_texture("player", EntityFaction::Player, '@');
    self.generate_texture("goblin", EntityFaction::Hostile, 'g');
    self.generate_texture("dragon", EntityFaction::Hostile, 'D');
    self.generate_texture("slime", EntityFaction::Hostile, 's');
  }

  fn get_rgba_color(&self, color: Color) -> [u8; 4] {
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

  fn get_color(&self, sprite: &Sprite, faction: EntityFaction) -> Color {
    sprite.get_color(faction, sprite.char)
  }

  fn generate_texture(&mut self, name: &str, faction: EntityFaction, default_char: char) {
    let sprite = self.tile_loader.load_or_single(name, default_char);
    let art = &sprite.art;

    let width = art.width as u32 * 16;
    let height = art.height as u32 * 16;

    let mut img: RgbaImage = ImageBuffer::new(width, height);

    for py in 0..art.height {
      for px in 0..art.width {
        let color = self.get_color(&sprite, faction);
        let rgba = self.get_rgba_color(color);

        for dy in 0..16 {
          for dx in 0..16 {
            let x = (px * 16 + dx) as u32;
            let y = (py * 16 + dy) as u32;

            if art.is_empty(px, py) {
              img.put_pixel(x, y, Rgba([0, 0, 0, 0]));
            } else {
              img.put_pixel(x, y, Rgba(rgba));
            }
          }
        }
      }
    }

    let output_path = format!("assets/textures/{}.png", name);
    fs::create_dir_all("assets/textures").ok();
    img.save(&output_path).ok();
    println!("Generated texture: {}", output_path);
  }
}

impl Default for TextureGenerator {
  fn default() -> Self { Self::new() }
}

fn main() {
  let mut generator = TextureGenerator::new();
  generator.generate_all_textures();
  println!("All textures generated!");
}
