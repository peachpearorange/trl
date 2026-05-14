use std::collections::HashMap;
#[cfg(not(target_arch = "wasm32"))]
use std::path::PathBuf;

#[cfg(target_arch = "wasm32")]
#[derive(rust_embed::RustEmbed)]
#[folder = "assets/"]
struct EmbeddedAssets;

fn load_asset_bytes(relative_path: &str) -> Vec<u8> {
  #[cfg(not(target_arch = "wasm32"))]
  {
    let fs_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets").join(relative_path);
    std::fs::read(&fs_path)
      .unwrap_or_else(|e| panic!("load_asset_bytes: failed to read {}: {e}", fs_path.display()))
  }
  #[cfg(target_arch = "wasm32")]
  {
    EmbeddedAssets::get(relative_path)
      .unwrap_or_else(|| panic!("load_asset_bytes: embedded asset not found: {relative_path}"))
      .data
      .into_owned()
  }
}

use {bevy::{asset::RenderAssetUsages,
            color::LinearRgba,
            prelude::*,
            render::render_resource::{
              Extent3d, TextureAspect, TextureDimension, TextureFormat, TextureViewDescriptor,
              TextureViewDimension
            }},
     image::{imageops, RgbaImage}};

/// Handle and per-tile layer-range data for the baked tileset.
pub struct TilesetInfo {
  pub handle: Handle<Image>,
  /// Indexed by `Tile as usize`. Each entry is (first_layer, layer_count).
  /// Single-sprite tiles have count=1; SpritePackRandom tiles have count = paths * 8.
  pub layer_range: Vec<(u16, u16)>
}

/// Cache key for baked palette sprites (path + instance colors).
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct PaletteKey {
  pub path: &'static str,
  pub primary: [u8; 4],
  pub secondary: [u8; 4]
}

fn quantize_srgba(c: Color) -> [u8; 4] {
  let s = c.to_srgba();
  [
    (s.red * 255.0).clamp(0.0, 255.0).round() as u8,
    (s.green * 255.0).clamp(0.0, 255.0).round() as u8,
    (s.blue * 255.0).clamp(0.0, 255.0).round() as u8,
    (s.alpha * 255.0).clamp(0.0, 255.0).round() as u8
  ]
}

/// Maps PNG pixels: near-black → primary, near-white → secondary, transparent unchanged.
fn map_palette_pixel(
  r: u8,
  g: u8,
  b: u8,
  a: u8,
  primary: Color,
  secondary: Color
) -> [u8; 4] {
  if a < 8 {
    [0, 0, 0, 0]
  } else {
    let rf = r as f32 / 255.0;
    let gf = g as f32 / 255.0;
    let bf = b as f32 / 255.0;
    let max_c = rf.max(gf).max(bf);
    let min_c = rf.min(gf).min(bf);
    let lum = 0.299 * rf + 0.587 * gf + 0.114 * bf;
    let t = if max_c < 0.06 && min_c < 0.06 {
      0.0
    } else if min_c > 0.94 && max_c > 0.94 {
      1.0
    } else {
      lum.clamp(0.0, 1.0)
    };
    let p = primary.to_linear();
    let s = secondary.to_linear();
    let mixed = LinearRgba {
      red: p.red + (s.red - p.red) * t,
      green: p.green + (s.green - p.green) * t,
      blue: p.blue + (s.blue - p.blue) * t,
      alpha: p.alpha + (s.alpha - p.alpha) * t
    };
    let out = Color::LinearRgba(mixed).to_srgba();
    [
      (out.red * 255.0).round() as u8,
      (out.green * 255.0).round() as u8,
      (out.blue * 255.0).round() as u8,
      a
    ]
  }
}

fn bake_palette_png(img: &RgbaImage, primary: Color, secondary: Color) -> Vec<u8> {
  img
    .enumerate_pixels()
    .flat_map(|(_, _, px)| {
      let [r, g, b, a] = px.0;
      map_palette_pixel(r, g, b, a, primary, secondary)
    })
    .collect()
}

/// CPU-baked image: black/white mask PNG becomes two-tone colors per instance.
#[derive(Resource, Default)]
pub struct PaletteImageCache(pub HashMap<PaletteKey, Handle<Image>>);

/// Loads `assets/<path>`, remaps black→`primary` and white→`secondary`, caches by key.
pub fn palette_sprite_handle(
  path: &'static str,
  primary: Color,
  secondary: Color,
  cache: &mut PaletteImageCache,
  images: &mut Assets<Image>
) -> Handle<Image> {
  let key = PaletteKey {
    path,
    primary: quantize_srgba(primary),
    secondary: quantize_srgba(secondary)
  };
  if let Some(h) = cache.0.get(&key) {
    h.clone()
  } else {
    let bytes = load_asset_bytes(path);
    let dyn_img = image::load_from_memory(&bytes)
      .unwrap_or_else(|e| panic!("palette_sprite_handle: failed to decode {path}: {e}"));
    let rgba = dyn_img.to_rgba8();
    let (w, h) = rgba.dimensions();
    let data = bake_palette_png(&rgba, primary, secondary);
    let handle = images.add(Image::new(
      Extent3d { width: w, height: h, depth_or_array_layers: 1 },
      TextureDimension::D2,
      data,
      TextureFormat::Rgba8UnormSrgb,
      RenderAssetUsages::RENDER_WORLD
    ));
    cache.0.insert(key, handle.clone());
    handle
  }
}

/// Returns all 8 flip/rotation variants of an image (4 rotations × 2 horizontal mirrors).
fn all_orientations(img: &RgbaImage) -> [RgbaImage; 8] {
  let r0 = img.clone();
  let r90 = imageops::rotate90(img);
  let r180 = imageops::rotate180(img);
  let r270 = imageops::rotate270(img);
  let fh = imageops::flip_horizontal(img);
  let fh90 = imageops::rotate90(&fh);
  let fh180 = imageops::rotate180(&fh);
  let fh270 = imageops::rotate270(&fh);
  [r0, r90, r180, r270, fh, fh90, fh180, fh270]
}

/// Build a 2D array image with one layer per tile variant.
/// `SpritePackRandom` tiles expand to paths × 8 orientation layers each.
/// Returns `TilesetInfo` with the image handle and a per-tile (base_layer, count) index.
pub fn build_tileset(images: &mut Assets<Image>) -> TilesetInfo {
  use crate::tiles::{Tile, TileRenderMode};
  let s = crate::SPRITE_TEXELS as u32;
  let layer_bytes = (s * s * 4) as usize;
  let tiles: Vec<Tile> = Tile::all().collect();
  let mut data: Vec<u8> = Vec::new();
  let mut layer_range: Vec<(u16, u16)> = Vec::with_capacity(tiles.len());
  let mut current_layer: u16 = 0;

  for tile in tiles {
    let base = current_layer;
    if tile == Tile::Air || tile == Tile::Blank {
      data.extend(std::iter::repeat(0u8).take(layer_bytes));
      current_layer += 1;
    } else {
      match tile.render_mode() {
        TileRenderMode::SolidColor => {
          let [r, g, b] = tile.color();
          let px = [(r * 255.0).round() as u8, (g * 255.0).round() as u8, (b * 255.0).round() as u8, 255u8];
          for _ in 0..(s * s) {
            data.extend_from_slice(&px);
          }
          current_layer += 1;
        }
        TileRenderMode::Sprite(path, pri, sec) => {
          let bytes = load_asset_bytes(path);
          let rgba = image::load_from_memory(&bytes)
            .unwrap_or_else(|e| panic!("build_tileset: failed to decode {path}: {e}"))
            .to_rgba8();
          data.extend_from_slice(&bake_palette_png(
            &rgba,
            Color::srgb(pri[0], pri[1], pri[2]),
            Color::srgb(sec[0], sec[1], sec[2])
          ));
          current_layer += 1;
        }
        TileRenderMode::SpritePackRandom(paths, pri, sec) => {
          let prim = Color::srgb(pri[0], pri[1], pri[2]);
          let sec_col = Color::srgb(sec[0], sec[1], sec[2]);
          for path in paths {
            let bytes = load_asset_bytes(path);
            let rgba = image::load_from_memory(&bytes)
              .unwrap_or_else(|e| panic!("build_tileset: failed to decode {path}: {e}"))
              .to_rgba8();
            for oriented in all_orientations(&rgba) {
              data.extend_from_slice(&bake_palette_png(&oriented, prim, sec_col));
              current_layer += 1;
            }
          }
        }
      }
    }
    layer_range.push((base, current_layer - base));
  }

  let n = current_layer as u32;
  let mut img = Image::new(
    Extent3d { width: s, height: s, depth_or_array_layers: n },
    TextureDimension::D2,
    data,
    TextureFormat::Rgba8UnormSrgb,
    RenderAssetUsages::RENDER_WORLD
  );
  img.texture_view_descriptor = Some(TextureViewDescriptor {
    label: None,
    format: None,
    dimension: Some(TextureViewDimension::D2Array),
    usage: None,
    aspect: TextureAspect::All,
    base_mip_level: 0,
    mip_level_count: None,
    base_array_layer: 0,
    array_layer_count: Some(n)
  });
  TilesetInfo { handle: images.add(img), layer_range }
}

pub struct SpriteDef {
  pub width: usize,
  pub height: usize,
  pub pixels: Vec<[u8; 4]>
}

impl SpriteDef {
  pub fn from_chars(assoc: &[(char, [u8; 4])], layout: &str) -> Self {
    let lines: Vec<&str> = layout.lines().filter(|l| !l.trim().is_empty()).collect();
    let height = lines.len();
    let width = lines.iter().map(|l| l.len()).max().unwrap_or(0);

    let mut pixels = Vec::with_capacity(width * height);
    for line in &lines {
      let padded: String = format!("{:<width$}", line, width = width);
      for ch in padded.chars() {
        let color = assoc
          .iter()
          .find(|(c, _)| *c == ch)
          .map(|(_, rgba)| *rgba)
          .unwrap_or([0, 0, 0, 0]);
        pixels.push(color);
      }
    }

    SpriteDef { width, height, pixels }
  }

  pub fn to_image(&self, images: &mut Assets<Image>) -> Handle<Image> {
    let data: Vec<u8> = self.pixels.iter().flat_map(|p| p.iter().copied()).collect();
    images.add(Image::new(
      Extent3d {
        width: self.width as u32,
        height: self.height as u32,
        depth_or_array_layers: 1
      },
      TextureDimension::D2,
      data,
      TextureFormat::Rgba8UnormSrgb,
      RenderAssetUsages::RENDER_WORLD
    ))
  }
}
