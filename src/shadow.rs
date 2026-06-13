use bevy::{camera::visibility::RenderLayers,
           prelude::*,
           reflect::TypePath,
           render::render_resource::AsBindGroup,
           shader::ShaderRef,
           sprite_render::{AlphaMode2d, Material2d, Material2dPlugin}};

#[derive(AsBindGroup, Asset, TypePath, Debug, Clone)]
pub struct ShadowMaterial {
  #[texture(0)]
  #[sampler(1)]
  pub texture: Handle<Image>,
  /// x = inflate (quad-uv to sprite-uv scale), y = radius in sprite pixels,
  /// z = max shadow alpha, w = unused.
  #[uniform(2)]
  pub params: Vec4
}

impl Material2d for ShadowMaterial {
  fn fragment_shader() -> ShaderRef { "shaders/shadow.wgsl".into() }
  fn alpha_mode(&self) -> AlphaMode2d { AlphaMode2d::Blend }
}

/// Marker on the shadow child entity so it can be located and removed when a
/// glyph's visuals get rebuilt (e.g. doors swapping textures on open/close).
#[derive(Component)]
pub struct ShadowChild;

/// Quad inflation factor: the shadow quad is this much bigger than the sprite
/// quad, so the wgsl shader has room to draw the halo around the silhouette.
pub const SHADOW_INFLATE: f32 = 1.35;
/// Halo radius in sprite-texture pixels.
pub const SHADOW_RADIUS_PX: f32 = 1.5;
/// Peak shadow alpha at the silhouette edge.
pub const SHADOW_ALPHA: f32 = 0.75;

impl ShadowMaterial {
  pub fn new(texture: Handle<Image>) -> Self {
    Self {
      texture,
      params: Vec4::new(SHADOW_INFLATE, SHADOW_RADIUS_PX, SHADOW_ALPHA, 0.0)
    }
  }
}

pub struct ShadowPlugin;

impl Plugin for ShadowPlugin {
  fn build(&self, app: &mut App) {
    app.add_plugins(Material2dPlugin::<ShadowMaterial>::default());
  }
}

/// Spawns a shadow as a child of `parent`. `parent_scale` is the world-space
/// scale already applied to the parent's transform (`TILE_SIZE` for Mesh2d
/// glyphs, or `1.0` for Sprite-based glyphs where size lives on the Sprite
/// itself). The child's local scale fills in whatever the parent doesn't, so
/// the shadow lands at `TILE_SIZE * SHADOW_INFLATE` either way. A small
/// negative local-z drops the shadow just behind the sprite.
pub fn spawn_shadow_child(
  commands: &mut Commands,
  parent: Entity,
  texture: Handle<Image>,
  mesh: Handle<Mesh>,
  shadow_materials: &mut Assets<ShadowMaterial>,
  render_layer: usize,
  tile_size: f32,
  parent_scale: f32
) {
  let mat = shadow_materials.add(ShadowMaterial::new(texture));
  let local_scale = (tile_size / parent_scale) * SHADOW_INFLATE;
  let local_z = -0.5 / parent_scale;
  commands.entity(parent).with_child((
    Mesh2d(mesh),
    MeshMaterial2d(mat),
    Transform::from_xyz(0.0, 0.0, local_z).with_scale(Vec3::splat(local_scale)),
    RenderLayers::layer(render_layer),
    ShadowChild
  ));
}

