use bevy::{prelude::*, reflect::TypePath, render::render_resource::AsBindGroup, shader::ShaderRef, sprite_render::{Material2d, Material2dPlugin}};

#[derive(Component)]
pub struct InteractOutline;

#[derive(AsBindGroup, Asset, TypePath, Debug, Clone)]
pub struct OutlineMaterial {
  #[texture(0)]
  #[sampler(1)]
  pub texture: Handle<Image>,
  #[uniform(2)]
  pub color: LinearRgba,
}

impl Material2d for OutlineMaterial {
  fn fragment_shader() -> ShaderRef {
    "shaders/outline.wgsl".into()
  }
}

const MAX_OUTLINES: usize = 16;

#[derive(Resource)]
pub struct OutlinePool {
  pub quad: Handle<Mesh>,
  pub entities: Vec<Entity>,
}

pub struct OutlinePlugin;

impl Plugin for OutlinePlugin {
  fn build(&self, app: &mut App) {
    app
      .add_plugins(Material2dPlugin::<OutlineMaterial>::default())
      .add_systems(Startup, spawn_outline_pool);
  }
}

fn spawn_outline_pool(
  mut commands: Commands,
  mut meshes: ResMut<Assets<Mesh>>,
) {
  let quad = meshes.add(Rectangle::new(1.0, 1.0));
  let pad_texels = 1.0;
  let padded = crate::TILE_SIZE + pad_texels * crate::SCREEN_PIXELS_PER_TEXEL * 2.0;
  let entities: Vec<Entity> = (0..MAX_OUTLINES).map(|_| {
    commands.spawn((
      InteractOutline,
      Mesh2d(quad.clone()),
      Transform::from_translation(Vec3::ZERO).with_scale(Vec3::new(padded, padded, 1.0)),
      Visibility::Hidden,
    )).id()
  }).collect();
  commands.insert_resource(OutlinePool { quad, entities });
}
