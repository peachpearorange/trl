use bevy::{
    camera::{RenderTarget, visibility::RenderLayers},
    prelude::*,
    reflect::TypePath,
    render::render_resource::{AsBindGroup, TextureFormat},
    shader::ShaderRef,
    sprite_render::{Material2d, Material2dPlugin},
    window::WindowResized,
};

/// Render target the game camera writes into; sampled by the display pass.
#[derive(Resource)]
pub struct GameRenderTarget(pub Handle<Image>);

/// Marks the game-world camera (renders to texture).
#[derive(Component)]
pub struct GameCamera;

/// Marks the fullscreen display mesh.
#[derive(Component)]
struct DisplayMesh;

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
           .add_systems(Update, sync_camera_pos)
           .add_systems(Update, on_window_resized);
    }
}

fn create_render_target(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    windows: Single<&Window>,
) {
    let image = Image::new_target_texture(
        windows.physical_width(),
        windows.physical_height(),
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
    windows: Single<&Window>,
) {
    let (w, h) = (windows.width(), windows.height());

    // Unit rectangle scaled via Transform so resizing only needs a Transform update.
    commands.spawn((
        Mesh2d(meshes.add(Rectangle::new(1.0, 1.0))),
        MeshMaterial2d(materials.add(DisplayMaterial {
            screen: render_target.0.clone(),
            cam_pos: Vec4::ZERO,
        })),
        Transform::from_scale(Vec3::new(w, h, 1.0)),
        RenderLayers::layer(1),
        DisplayMesh,
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
    cam_tf: Single<&Transform, With<GameCamera>>,
    mut materials: ResMut<Assets<DisplayMaterial>>,
    q: Query<&MeshMaterial2d<DisplayMaterial>>,
) {
    for handle in &q {
        if let Some(mat) = materials.get_mut(handle) {
            mat.cam_pos = cam_tf.translation.extend(0.0);
        }
    }
}

fn on_window_resized(
    mut events: MessageReader<WindowResized>,
    mut images: ResMut<Assets<Image>>,
    mut render_target: ResMut<GameRenderTarget>,
    mut game_cam_rt: Query<&mut RenderTarget, With<GameCamera>>,
    display_mat_q: Query<&MeshMaterial2d<DisplayMaterial>>,
    mut materials: ResMut<Assets<DisplayMaterial>>,
    mut mesh_tf: Single<&mut Transform, With<DisplayMesh>>,
    windows: Single<&Window>,
) {
    let Some(_) = events.read().last() else { return };

    let new_image = Image::new_target_texture(
        windows.physical_width(),
        windows.physical_height(),
        TextureFormat::bevy_default(),
        None,
    );
    let new_handle = images.add(new_image);
    render_target.0 = new_handle.clone();

    if let Ok(mut rt) = game_cam_rt.single_mut() {
        *rt = RenderTarget::Image(new_handle.clone().into());
    }

    for handle in &display_mat_q {
        if let Some(mat) = materials.get_mut(handle) {
            mat.screen = new_handle.clone();
        }
    }

    let (w, h) = (windows.width(), windows.height());
    mesh_tf.scale = Vec3::new(w, h, 1.0);
}

pub fn game_render_target(render_target: &GameRenderTarget) -> RenderTarget {
    RenderTarget::Image(render_target.0.clone().into())
}
