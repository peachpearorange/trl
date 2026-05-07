use std::collections::HashMap;
use std::path::PathBuf;

use bevy::asset::RenderAssetUsages;
use bevy::color::LinearRgba;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use image::RgbaImage;

/// Cache key for baked palette sprites (path + instance colors).
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct PaletteKey {
    pub path: &'static str,
    pub primary: [u8; 4],
    pub secondary: [u8; 4],
}

fn quantize_srgba(c: Color) -> [u8; 4] {
    let s = c.to_srgba();
    [
        (s.red * 255.0).clamp(0.0, 255.0).round() as u8,
        (s.green * 255.0).clamp(0.0, 255.0).round() as u8,
        (s.blue * 255.0).clamp(0.0, 255.0).round() as u8,
        (s.alpha * 255.0).clamp(0.0, 255.0).round() as u8,
    ]
}

/// Maps PNG pixels: near-black → primary, near-white → secondary, transparent unchanged.
fn map_palette_pixel(r: u8, g: u8, b: u8, a: u8, primary: Color, secondary: Color) -> [u8; 4] {
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
            alpha: p.alpha + (s.alpha - p.alpha) * t,
        };
        let out = Color::LinearRgba(mixed).to_srgba();
        [
            (out.red * 255.0).round() as u8,
            (out.green * 255.0).round() as u8,
            (out.blue * 255.0).round() as u8,
            a,
        ]
    }
}

fn bake_palette_png(img: &RgbaImage, primary: Color, secondary: Color) -> Vec<u8> {
    img.enumerate_pixels()
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
    images: &mut Assets<Image>,
) -> Handle<Image> {
    let key = PaletteKey {
        path,
        primary: quantize_srgba(primary),
        secondary: quantize_srgba(secondary),
    };
    if let Some(h) = cache.0.get(&key) {
        h.clone()
    } else {
        let fs_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("assets")
            .join(path);
        let dyn_img = image::open(&fs_path).unwrap_or_else(|e| {
            panic!("palette_sprite_handle: failed to open {}: {e}", fs_path.display())
        });
        let rgba = dyn_img.to_rgba8();
        let (w, h) = rgba.dimensions();
        let data = bake_palette_png(&rgba, primary, secondary);
        let handle = images.add(Image::new(
            Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            data,
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::RENDER_WORLD,
        ));
        cache.0.insert(key, handle.clone());
        handle
    }
}

pub struct SpriteDef {
    pub width: usize,
    pub height: usize,
    pub pixels: Vec<[u8; 4]>,
}

impl SpriteDef {
    pub fn from_chars(assoc: &[(char, [u8; 4])], layout: &str) -> Self {
        let lines: Vec<&str> = layout
            .lines()
            .filter(|l| !l.trim().is_empty())
            .collect();
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

        SpriteDef {
            width,
            height,
            pixels,
        }
    }

    pub fn to_image(&self, images: &mut Assets<Image>) -> Handle<Image> {
        let data: Vec<u8> = self.pixels.iter().flat_map(|p| p.iter().copied()).collect();
        images.add(Image::new(
            Extent3d {
                width: self.width as u32,
                height: self.height as u32,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            data,
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::RENDER_WORLD,
        ))
    }
}
