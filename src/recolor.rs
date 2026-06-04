use bevy::{ecs::system::SystemParam,
           prelude::*,
           reflect::TypePath,
           render::render_resource::AsBindGroup,
           shader::ShaderRef,
           sprite_render::{Material2d, Material2dPlugin}};

use crate::sprites::PaletteImageCache;

#[derive(AsBindGroup, Asset, TypePath, Debug, Clone)]
pub struct RecolorMaterial {
  #[texture(0)]
  #[sampler(1)]
  pub texture: Handle<Image>,
  #[uniform(2)]
  pub primary: LinearRgba,
  #[uniform(3)]
  pub secondary: LinearRgba
}

impl Material2d for RecolorMaterial {
  fn fragment_shader() -> ShaderRef { "shaders/recolor.wgsl".into() }
}

#[derive(Resource)]
pub struct RecolorQuad(pub Handle<Mesh>);

#[derive(SystemParam)]
pub struct SpriteRes<'w> {
  pub palette_cache: ResMut<'w, PaletteImageCache>,
  pub images: ResMut<'w, Assets<Image>>,
  pub recolor_materials: ResMut<'w, Assets<RecolorMaterial>>,
  pub recolor_quad: Res<'w, RecolorQuad>
}

pub struct RecolorPlugin;

impl Plugin for RecolorPlugin {
  fn build(&self, app: &mut App) {
    app
      .add_plugins(Material2dPlugin::<RecolorMaterial>::default())
      .add_systems(Startup, |mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>| {
        commands.insert_resource(RecolorQuad(meshes.add(Rectangle::new(1.0, 1.0))));
      });
  }
}
