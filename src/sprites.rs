use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

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
