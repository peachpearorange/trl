use bevy::{asset::RenderAssetUsages,
           ecs::system::SystemParam,
           image::ImageSampler,
           prelude::*,
           reflect::TypePath,
           render::render_resource::{AsBindGroup, Extent3d, TextureDimension, TextureFormat},
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

/// Shared FOV brightness texture (R8Unorm, one texel per tile of the active level).
/// `update_fov_visuals` rewrites its pixels each frame; the display composite shader
/// (`display.wgsl`) samples it once to dim the whole scene by FOV, so neither sprites nor
/// tiles need to bake brightness into themselves.
#[derive(Resource)]
pub struct FovLightmap(pub Handle<Image>);

impl FromWorld for FovLightmap {
  fn from_world(world: &mut World) -> Self {
    let mut images = world.resource_mut::<Assets<Image>>();
    FovLightmap(images.add(fov_image(1, 1)))
  }
}

/// A zeroed `width`x`height` R8Unorm brightness image with nearest sampling, kept in both
/// worlds so its pixels can be mutated each frame and re-uploaded to the GPU.
pub fn fov_image(width: usize, height: usize) -> Image {
  let mut img = Image::new(
    Extent3d { width: width as u32, height: height as u32, depth_or_array_layers: 1 },
    TextureDimension::D2,
    vec![0u8; width * height],
    TextureFormat::R8Unorm,
    RenderAssetUsages::default()
  );
  img.sampler = ImageSampler::nearest();
  img
}

#[derive(SystemParam)]
pub struct SpriteRes<'w> {
  pub palette_cache: ResMut<'w, PaletteImageCache>,
  pub images: ResMut<'w, Assets<Image>>,
  pub recolor_materials: ResMut<'w, Assets<RecolorMaterial>>,
  pub recolor_quad: Res<'w, RecolorQuad>
}

impl SpriteRes<'_> {
  pub fn add_recolor(
    &mut self,
    texture: Handle<Image>,
    primary: LinearRgba,
    secondary: LinearRgba
  ) -> Handle<RecolorMaterial> {
    self.recolor_materials.add(RecolorMaterial { texture, primary, secondary })
  }
}

impl RecolorMaterial {
  pub fn set_colors(&mut self, primary: LinearRgba, secondary: LinearRgba) {
    self.primary = primary;
    self.secondary = secondary;
  }
}

pub struct RecolorPlugin;

impl Plugin for RecolorPlugin {
  fn build(&self, app: &mut App) {
    app
      .add_plugins(Material2dPlugin::<RecolorMaterial>::default())
      .init_resource::<FovLightmap>()
      .add_systems(Startup, |mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>| {
        commands.insert_resource(RecolorQuad(meshes.add(Rectangle::new(1.0, 1.0))));
      });
  }
}
