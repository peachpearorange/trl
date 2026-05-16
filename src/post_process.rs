use bevy::{prelude::*, render::render_resource::AsBindGroup, shader::ShaderRef};

#[derive(AsBindGroup, Asset, TypePath, Debug, Clone)]
pub struct ScanlineMaterial {}

impl UiMaterial for ScanlineMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/scanlines.wgsl".into()
    }
}

pub struct PostProcessPlugin;

impl Plugin for PostProcessPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(UiMaterialPlugin::<ScanlineMaterial>::default())
           .add_systems(Startup, spawn_scanlines);
    }
}

fn spawn_scanlines(mut commands: Commands, mut materials: ResMut<Assets<ScanlineMaterial>>) {
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(0.0),
            top: Val::Px(0.0),
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            ..default()
        },
        MaterialNode(materials.add(ScanlineMaterial {})),
        GlobalZIndex(i32::MAX),
        Pickable::IGNORE,
    ));
}
