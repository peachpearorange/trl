use bevy::{
    camera::{RenderTarget, visibility::RenderLayers},
    prelude::*,
    reflect::TypePath,
    render::render_resource::{AsBindGroup, TextureFormat},
    shader::ShaderRef,
    sprite_render::{Material2d, Material2dPlugin},
};

/// Render target the game camera writes into; sampled by the display pass.
#[derive(Resource)]
pub struct GameRenderTarget(pub Handle<Image>);

/// Marks the game-world camera (renders to texture). Use this instead of `With<Camera2d>`
/// when you need to distinguish it from the display camera.
#[derive(Component)]
pub struct GameCamera;

#[derive(AsBindGroup, Asset, TypePath, Debug, Clone)]
pub struct DisplayMaterial {
    #[texture(0)]
    #[sampler(1)]
    screen: Handle<Image>,
    /// xy = game camera world position; zw unused (Vec4 for alignment).
    #[uniform(2)]
    cam_pos: Vec4,
}

impl Material2d for DisplayMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/display.wgsl".into()
    }
}

pub struct PostProcessPlugin;

impl Plugin for PostProcessPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(Material2dPlugin::<DisplayMaterial>::default())
           .add_systems(PreStartup, create_render_target)
           .add_systems(PostStartup, setup_display)
           .add_systems(Update, sync_camera_pos);
    }
}

fn create_render_target(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    windows: Query<&Window>,
) {
    let window = windows.single().expect("no window");
    let image = Image::new_target_texture(
        window.physical_width(),
        window.physical_height(),
        TextureFormat::bevy_default(),
        None,
    );
    commands.insert_resource(GameRenderTarget(images.add(image)));
}

fn setup_display(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<DisplayMaterial>>,
    render_target: Res<GameRenderTarget>,
    windows: Query<&Window>,
) {
    let window = windows.single().expect("no window");
    let (w, h) = (window.width(), window.height());

    commands.spawn((
        Mesh2d(meshes.add(Rectangle::new(w, h))),
        MeshMaterial2d(materials.add(DisplayMaterial { screen: render_target.0.clone(), cam_pos: Vec4::ZERO })),
        RenderLayers::layer(1),
    ));

    commands.spawn((
        Camera2d,
        Camera { order: 1, clear_color: ClearColorConfig::Custom(Color::BLACK), ..default() },
        RenderLayers::layer(1),
        IsDefaultUiCamera,
        Msaa::Off,
    ));
}

fn sync_camera_pos(
    cam_q: Query<&Transform, With<GameCamera>>,
    mut materials: ResMut<Assets<DisplayMaterial>>,
    q: Query<&MeshMaterial2d<DisplayMaterial>>,
) {
    let Ok(cam_tf) = cam_q.single() else { return };
    for handle in &q {
        if let Some(mat) = materials.get_mut(handle) {
            mat.cam_pos = cam_tf.translation.extend(0.0);
        }
    }
}

pub fn game_render_target(render_target: &GameRenderTarget) -> RenderTarget {
    RenderTarget::Image(render_target.0.clone().into())
}
