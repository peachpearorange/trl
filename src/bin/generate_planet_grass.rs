use {rand::{SeedableRng, rngs::SmallRng, seq::SliceRandom},
     std::{env, fs, path::Path}};

#[path = "../utils.rs"]
mod utils;

const PLANET_SIZE: usize = 300;
const PATTERN_SIZE: usize = 5;
const SEED: u64 = 0xDEAD_BEEF;
const DEFAULT_SAVE_PATH: &str = "editor_saves/editor_save_1779637849486.txt";
const DEFAULT_OUTPUT_PATH: &str = "assets/generated/planets/planet_grass.bin";
const EDITOR_GRASS_TILE: u16 = 22;
const PLANET_GRASS_MAGIC: &[u8; 4] = b"PGR1";

fn encode_editor_cell(tile: u16, object: i16) -> u16 {
  if object >= 0 { tile | ((object as u16 + 1) << 8) } else { tile }
}

fn editor_save_cells(path: &str) -> (usize, usize, Vec<u16>) {
  let text = fs::read_to_string(path).expect("failed to read editor save");
  let mut nums = text.split_whitespace();
  let width: usize = nums.next().and_then(|s| s.parse().ok()).unwrap_or(1);
  let height: usize = nums.next().and_then(|s| s.parse().ok()).unwrap_or(1);
  let remaining: Vec<&str> = nums.collect();
  let cell_tokens = width.saturating_mul(height).saturating_mul(2);
  let mut data_idx = if remaining.len() >= cell_tokens + 2 { 2 } else { 0 };
  let cells = utils::mapv(
    |_| {
      let tile =
        remaining.get(data_idx).and_then(|s| s.parse().ok()).unwrap_or(EDITOR_GRASS_TILE);
      data_idx += 1;
      let object = remaining.get(data_idx).and_then(|s| s.parse().ok()).unwrap_or(-1);
      data_idx += 1;
      encode_editor_cell(tile, object)
    },
    0..width * height
  );

  (width, height, cells)
}

fn oriented_index(x: usize, y: usize, orientation: usize) -> usize {
  let max = PATTERN_SIZE - 1;
  let (sx, sy) = match orientation {
    0 => (x, y),
    1 => (y, max - x),
    2 => (max - x, max - y),
    3 => (max - y, x),
    4 => (max - x, y),
    5 => (y, x),
    6 => (x, max - y),
    _ => (max - y, max - x)
  };
  sy * PATTERN_SIZE + sx
}

fn editor_patterns(width: usize, height: usize, cells: &[u16]) -> Vec<Vec<u16>> {
  let mut patterns = Vec::new();
  for y in 0..=height.saturating_sub(PATTERN_SIZE) {
    for x in 0..=width.saturating_sub(PATTERN_SIZE) {
      let source = (0..PATTERN_SIZE)
        .flat_map(|dy| (0..PATTERN_SIZE).map(move |dx| cells[(y + dy) * width + x + dx]))
        .collect::<Vec<_>>();
      for orientation in 0..8 {
        patterns.push(
          (0..PATTERN_SIZE * PATTERN_SIZE)
            .map(|i| {
              source[oriented_index(i % PATTERN_SIZE, i / PATTERN_SIZE, orientation)]
            })
            .collect()
        );
      }
    }
  }
  patterns
}

fn fits(output: &[u16], block_x: usize, block_y: usize, pattern: &[u16]) -> bool {
  let start_x = block_x * PATTERN_SIZE;
  let start_y = block_y * PATTERN_SIZE;
  let top_ok = block_y == 0
    || (0..PATTERN_SIZE)
      .all(|dx| output[(start_y - 1) * PLANET_SIZE + start_x + dx] == pattern[dx]);
  let left_ok = block_x == 0
    || (0..PATTERN_SIZE).all(|dy| {
      output[(start_y + dy) * PLANET_SIZE + start_x - 1] == pattern[dy * PATTERN_SIZE]
    });
  top_ok && left_ok
}

fn generate_planet(patterns: &[Vec<u16>]) -> Vec<u16> {
  let mut rng = SmallRng::seed_from_u64(SEED);
  let mut output = vec![EDITOR_GRASS_TILE; PLANET_SIZE * PLANET_SIZE];
  let blocks = PLANET_SIZE / PATTERN_SIZE;

  for block_y in 0..blocks {
    for block_x in 0..blocks {
      let candidates = patterns
        .iter()
        .filter(|pattern| fits(&output, block_x, block_y, pattern))
        .collect::<Vec<_>>();
      let pattern = candidates
        .choose(&mut rng)
        .copied()
        .or_else(|| patterns.choose(&mut rng))
        .expect("editor save must provide at least one pattern");
      let start_x = block_x * PATTERN_SIZE;
      let start_y = block_y * PATTERN_SIZE;
      for dy in 0..PATTERN_SIZE {
        for dx in 0..PATTERN_SIZE {
          output[(start_y + dy) * PLANET_SIZE + start_x + dx] =
            pattern[dy * PATTERN_SIZE + dx];
        }
      }
    }
  }

  output
}

fn palette_encode(cells: &[u16]) -> Vec<u8> {
  let mut palette = Vec::<u16>::new();
  let mut indices = Vec::with_capacity(cells.len());
  for &cell in cells {
    let palette_index =
      palette.iter().position(|&existing| existing == cell).unwrap_or_else(|| {
        assert!(palette.len() < 256, "Planet Grass palette exceeded 256 cells");
        palette.push(cell);
        palette.len() - 1
      });
    indices.push(palette_index as u8);
  }

  let mut bytes =
    Vec::with_capacity(PLANET_GRASS_MAGIC.len() + 2 + palette.len() * 2 + indices.len());
  bytes.extend_from_slice(PLANET_GRASS_MAGIC);
  bytes.extend_from_slice(&(palette.len() as u16).to_le_bytes());
  for cell in palette {
    bytes.extend_from_slice(&cell.to_le_bytes());
  }
  bytes.extend(indices);
  bytes
}

fn main() {
  let save_path = env::args().nth(1).unwrap_or_else(|| DEFAULT_SAVE_PATH.to_string());
  let output_path = env::args().nth(2).unwrap_or_else(|| DEFAULT_OUTPUT_PATH.to_string());
  let (width, height, cells) = editor_save_cells(&save_path);
  let patterns = editor_patterns(width, height, &cells);
  let planet = generate_planet(&patterns);
  let bytes = palette_encode(&planet);

  if let Some(parent) = Path::new(&output_path).parent() {
    fs::create_dir_all(parent).expect("failed to create output directory");
  }
  fs::write(&output_path, bytes).expect("failed to write planet data");
  println!(
    "wrote {output_path} from {save_path} using {} {}x{} patterns",
    patterns.len(),
    PATTERN_SIZE,
    PATTERN_SIZE
  );
}
