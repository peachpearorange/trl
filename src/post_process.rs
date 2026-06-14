use {bevy::{asset::RenderAssetUsages,
            camera::{RenderTarget, visibility::RenderLayers},
            core_pipeline::core_2d::graph::{Core2d, Node2d},
            ecs::query::QueryItem,
            prelude::*,
            reflect::TypePath,
            render::{Render, RenderApp, RenderStartup, RenderSystems,
                     extract_component::{ExtractComponent, ExtractComponentPlugin},
                     extract_resource::{ExtractResource, ExtractResourcePlugin},
                     render_asset::RenderAssets,
                     render_graph::{self, RenderGraphExt, RenderLabel, ViewNode,
                                    ViewNodeRunner},
                     render_resource::{binding_types::{storage_buffer_sized,
                                                       texture_2d,
                                                       texture_storage_2d,
                                                       uniform_buffer},
                                       *},
                     renderer::{RenderContext, RenderDevice, RenderQueue},
                     texture::GpuImage},
            shader::{PipelineCacheError, ShaderRef},
            sprite_render::{Material2d, Material2dPlugin},
            window::WindowResized},
     std::borrow::Cow};

const WORKGROUP_SIZE: u32 = 8;
const LAYER_DISPLAY: usize = 1;
pub const LAYER_ENTITIES: usize = 2;
const ORDER_DISPLAY: isize = 10;

#[derive(Resource, Clone, ExtractResource)]
pub struct GameRenderTarget(pub Handle<Image>);

#[derive(Resource, Clone, ExtractResource)]
pub struct OutputImage(pub Handle<Image>);

#[derive(Resource, Clone, ExtractResource)]
pub struct EntityRenderTarget(pub Handle<Image>);

#[derive(Resource, Clone, Copy, Default, ExtractResource)]
pub struct CameraWorldOffset(pub IVec2);

#[derive(Resource, Clone, Copy, Default)]
pub struct PlayerScreenPos(pub Vec2);

#[derive(Component, Clone, Copy, ExtractComponent)]
pub struct GameCamera;

#[derive(Component)]
pub struct EntityCamera;

#[derive(Component)]
struct DisplayCam;

#[derive(Component)]
struct DisplayMesh;

#[derive(Resource)]
struct DisplayHandle(Handle<DisplayMaterial>);

#[derive(AsBindGroup, Asset, TypePath, Debug, Clone)]
pub struct DisplayMaterial {
  #[texture(0)]
  #[sampler(1)]
  screen: Handle<Image>,
  #[texture(2)]
  #[sampler(3)]
  entities: Handle<Image>,
  #[uniform(4)]
  time: f32,
  #[uniform(5)]
  world_offset: IVec2,
  #[uniform(6)]
  player_screen_pos: Vec2,
  // Per-tile FOV brightness (R8, one texel per tile of the active level). Sampled per screen
  // pixel to dim the whole scene by visibility — this is the single overlay that fades both
  // tiles and entities in/out of view. `map_dims` is the active level size in tiles and
  // `scale` is the window's physical-pixel scale factor, both needed to map a screen pixel
  // back to its world tile. This material is re-prepared every frame (see update_display_uniforms),
  // so binding the per-frame-mutated lightmap here is safe from stale bind groups.
  #[texture(7)]
  #[sampler(8)]
  fov_lightmap: Handle<Image>,
  #[uniform(9)]
  map_dims: Vec2,
  #[uniform(10)]
  scale: f32
}
impl Material2d for DisplayMaterial {
  fn fragment_shader() -> ShaderRef { "shaders/display.wgsl".into() }
}

pub struct PostProcessPlugin;

impl Plugin for PostProcessPlugin {
  fn build(&self, app: &mut App) {
    app
      .add_plugins((
        Material2dPlugin::<DisplayMaterial>::default(),
        ExtractComponentPlugin::<GameCamera>::default(),
        ExtractResourcePlugin::<GameRenderTarget>::default(),
        ExtractResourcePlugin::<OutputImage>::default(),
        ExtractResourcePlugin::<EntityRenderTarget>::default(),
        ExtractResourcePlugin::<CameraWorldOffset>::default()
      ))
      .init_resource::<CameraWorldOffset>()
      .init_resource::<PlayerScreenPos>()
      .add_systems(PreStartup, create_render_targets)
      .add_systems(PostStartup, setup_display)
      .add_systems(
        Update,
        (on_window_resized, update_camera_world_offset, update_display_uniforms)
      );

    let render_app = app.sub_app_mut(RenderApp);
    render_app
      .add_systems(RenderStartup, init_ccl_pipeline)
      .add_systems(Render, prepare_bind_group.in_set(RenderSystems::PrepareBindGroups))
      .add_render_graph_node::<ViewNodeRunner<CclNode>>(Core2d, CclLabel)
      .add_render_graph_edges(Core2d, (Node2d::MainTransparentPass, CclLabel, Node2d::EndMainPass));
  }
}

fn make_game_image(w: u32, h: u32) -> Image {
  Image::new_target_texture(w, h, TextureFormat::bevy_default(), None)
}

fn make_output_image(w: u32, h: u32) -> Image {
  let mut image = Image::new_target_texture(w, h, TextureFormat::Rgba8Unorm, None);
  image.asset_usage = RenderAssetUsages::RENDER_WORLD;
  image.texture_descriptor.usage = TextureUsages::STORAGE_BINDING
    | TextureUsages::TEXTURE_BINDING
    | TextureUsages::COPY_DST;
  image
}

fn make_entity_image(w: u32, h: u32) -> Image {
  Image::new_target_texture(w, h, TextureFormat::bevy_default(), None)
}

fn create_render_targets(
  mut commands: Commands,
  mut images: ResMut<Assets<Image>>,
  windows: Single<&Window>
) {
  let (w, h) = (windows.physical_width(), windows.physical_height());
  commands.insert_resource(GameRenderTarget(images.add(make_game_image(w, h))));
  commands.insert_resource(OutputImage(images.add(make_output_image(w, h))));
  commands.insert_resource(EntityRenderTarget(images.add(make_entity_image(w, h))));
}

fn setup_display(
  mut commands: Commands,
  mut meshes: ResMut<Assets<Mesh>>,
  mut display_mats: ResMut<Assets<DisplayMaterial>>,
  output: Res<OutputImage>,
  entity_rt: Res<EntityRenderTarget>,
  fov_lightmap: Res<crate::recolor::FovLightmap>,
  windows: Single<&Window>
) {
  let (w, h) = (windows.width(), windows.height());
  let quad = meshes.add(Rectangle::new(1.0, 1.0));
  let display_mat = display_mats.add(DisplayMaterial {
    screen: output.0.clone(),
    entities: entity_rt.0.clone(),
    time: 0.0,
    world_offset: IVec2::ZERO,
    player_screen_pos: Vec2::ZERO,
    fov_lightmap: fov_lightmap.0.clone(),
    map_dims: Vec2::ONE,
    scale: 1.0
  });
  commands.spawn((
    Mesh2d(quad),
    MeshMaterial2d(display_mat.clone()),
    Transform::from_scale(Vec3::new(w, h, 1.0)),
    RenderLayers::layer(LAYER_DISPLAY),
    DisplayMesh
  ));
  commands.spawn((
    Camera2d,
    DisplayCam,
    RenderLayers::layer(LAYER_DISPLAY),
    IsDefaultUiCamera,
    Msaa::Off,
    Camera {
      order: ORDER_DISPLAY,
      clear_color: ClearColorConfig::Custom(Color::BLACK),
      ..default()
    }
  ));
  commands.insert_resource(DisplayHandle(display_mat));
}

fn on_window_resized(
  mut events: MessageReader<WindowResized>,
  mut images: ResMut<Assets<Image>>,
  mut game_rt: ResMut<GameRenderTarget>,
  mut output: ResMut<OutputImage>,
  mut entity_rt: ResMut<EntityRenderTarget>,
  handle: Res<DisplayHandle>,
  mut display_mats: ResMut<Assets<DisplayMaterial>>,
  mut game_cam_rt: Query<
    &mut RenderTarget,
    (With<GameCamera>, Without<DisplayCam>, Without<EntityCamera>)
  >,
  mut entity_cam_rt: Query<
    &mut RenderTarget,
    (With<EntityCamera>, Without<GameCamera>, Without<DisplayCam>)
  >,
  mut mesh_tfs: Query<&mut Transform, With<DisplayMesh>>,
  windows: Single<&Window>
) {
  if events.read().last().is_some() {
    let (pw, ph) = (windows.physical_width(), windows.physical_height());
    let (w, h) = (windows.width(), windows.height());
    let new_game = images.add(make_game_image(pw, ph));
    let new_output = images.add(make_output_image(pw, ph));
    let new_entity = images.add(make_entity_image(pw, ph));
    game_rt.0 = new_game.clone();
    output.0 = new_output.clone();
    entity_rt.0 = new_entity.clone();
    if let Ok(mut rt) = game_cam_rt.single_mut() {
      *rt = RenderTarget::Image(new_game.into());
    }
    if let Ok(mut rt) = entity_cam_rt.single_mut() {
      *rt = RenderTarget::Image(new_entity.clone().into());
    }
    if let Some(m) = display_mats.get_mut(&handle.0) {
      m.screen = new_output;
      m.entities = new_entity;
    }
    let scale = Vec3::new(w, h, 1.0);
    for mut tf in &mut mesh_tfs {
      tf.scale = scale;
    }
  }
}

fn update_camera_world_offset(
  cam: Query<&GlobalTransform, With<GameCamera>>,
  windows: Single<&Window>,
  mut offset: ResMut<CameraWorldOffset>
) {
  if let Ok(tf) = cam.single() {
    let scale = windows.scale_factor();
    let (pw, ph) = (windows.physical_width() as f32, windows.physical_height() as f32);
    let t = tf.translation();
    let cx = (t.x * scale).round() - pw * 0.5;
    let cy = -((t.y * scale).round() + ph * 0.5);
    offset.0 = IVec2::new(cx as i32, cy as i32);
  }
}

fn update_display_uniforms(
  handle: Res<DisplayHandle>,
  mut display_mats: ResMut<Assets<DisplayMaterial>>,
  time: Res<Time>,
  offset: Res<CameraWorldOffset>,
  player_screen: Res<PlayerScreenPos>,
  current: Res<crate::CurrentZone>,
  windows: Single<&Window>
) {
  if let Some(m) = display_mats.get_mut(&handle.0) {
    m.time = time.elapsed_secs();
    m.world_offset = offset.0;
    m.player_screen_pos = player_screen.0;
    m.map_dims = Vec2::new(current.0.width as f32, current.0.height as f32);
    m.scale = windows.scale_factor();
  }
}

pub fn game_render_target(render_target: &GameRenderTarget) -> RenderTarget {
  RenderTarget::Image(render_target.0.clone().into())
}

pub fn entity_render_target(entity_rt: &EntityRenderTarget) -> RenderTarget {
  RenderTarget::Image(entity_rt.0.clone().into())
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
struct CclLabel;

#[derive(ShaderType, Clone, Copy)]
struct CclParams {
  size: UVec2,
  seed: u32,
  _pad: u32,
  world_offset: IVec2,
  _pad2: IVec2
}

#[derive(Resource)]
struct CclPipeline {
  layout: BindGroupLayoutDescriptor,
  init: CachedComputePipelineId,
  union: CachedComputePipelineId,
  compress: CachedComputePipelineId,
  recolor: CachedComputePipelineId
}

#[derive(Resource)]
struct CclResources {
  bind_group: BindGroup,
  parents_buffer: Buffer,
  parents_capacity: u64,
  size: UVec2
}

fn init_ccl_pipeline(
  mut commands: Commands,
  asset_server: Res<AssetServer>,
  pipeline_cache: Res<PipelineCache>
) {
  let layout = BindGroupLayoutDescriptor::new(
    "CclLayout",
    &BindGroupLayoutEntries::sequential(
      ShaderStages::COMPUTE,
      (
        texture_2d(TextureSampleType::Float { filterable: false }),
        storage_buffer_sized(false, None),
        texture_storage_2d(TextureFormat::Rgba8Unorm, StorageTextureAccess::WriteOnly),
        uniform_buffer::<CclParams>(false)
      )
    )
  );
  let shader = asset_server.load("shaders/ccl.wgsl");
  let make_pipeline = |entry: &'static str| {
    pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
      layout: vec![layout.clone()],
      shader: shader.clone(),
      entry_point: Some(Cow::from(entry)),
      ..default()
    })
  };
  commands.insert_resource(CclPipeline {
    init: make_pipeline("init_components"),
    union: make_pipeline("union_components"),
    compress: make_pipeline("compress_components"),
    recolor: make_pipeline("recolor_components"),
    layout
  });
}

fn prepare_bind_group(
  mut commands: Commands,
  pipeline: Res<CclPipeline>,
  pipeline_cache: Res<PipelineCache>,
  gpu_images: Res<RenderAssets<GpuImage>>,
  game_rt: Res<GameRenderTarget>,
  output: Res<OutputImage>,
  render_device: Res<RenderDevice>,
  queue: Res<RenderQueue>,
  existing: Option<Res<CclResources>>,
  cam_offset: Res<CameraWorldOffset>
) {
  let (Some(src), Some(dst)) = (gpu_images.get(&game_rt.0), gpu_images.get(&output.0))
  else {
    return;
  };
  let size = UVec2::new(src.size.width, src.size.height);
  if size.x == 0 || size.y == 0 {
    return;
  }
  let needed = (size.x as u64) * (size.y as u64) * 4;
  let reuse_buffer =
    existing.as_ref().is_some_and(|r| r.parents_capacity >= needed && r.size == size);
  let parents_buffer = if reuse_buffer {
    existing.as_ref().unwrap().parents_buffer.clone()
  } else {
    render_device.create_buffer(&BufferDescriptor {
      label: Some("ccl_parents"),
      size: needed,
      usage: BufferUsages::STORAGE,
      mapped_at_creation: false
    })
  };

  let mut params_buffer = UniformBuffer::from(CclParams {
    size,
    seed: 0,
    _pad: 0,
    world_offset: cam_offset.0,
    _pad2: IVec2::ZERO
  });
  params_buffer.write_buffer(&render_device, &queue);

  let bind_group = render_device.create_bind_group(
    Some("ccl_bind_group"),
    &pipeline_cache.get_bind_group_layout(&pipeline.layout),
    &BindGroupEntries::sequential((
      &src.texture_view,
      parents_buffer.as_entire_binding(),
      &dst.texture_view,
      &params_buffer
    ))
  );

  commands.insert_resource(CclResources {
    bind_group,
    parents_buffer,
    parents_capacity: needed,
    size
  });
}

// Runs inside Core2d after the main transparent pass so Dawn sees the explicit
// producer→consumer ordering: GameCamera writes `game_rt` → CCL reads it and writes
// `output` → DisplayCam (higher order) samples `output`. ViewQuery on `GameCamera`
// makes this a no-op on Entity/Display cameras' sub-graph runs.
#[derive(Default)]
struct CclNode;

impl ViewNode for CclNode {
  type ViewQuery = &'static GameCamera;

  fn run(
    &self,
    _graph: &mut render_graph::RenderGraphContext,
    render_context: &mut RenderContext,
    _view: QueryItem<Self::ViewQuery>,
    world: &World
  ) -> Result<(), render_graph::NodeRunError> {
    let Some(res) = world.get_resource::<CclResources>() else {
      return Ok(());
    };
    let pipeline = world.resource::<CclPipeline>();
    let cache = world.resource::<PipelineCache>();
    for id in [pipeline.init, pipeline.union, pipeline.compress, pipeline.recolor] {
      match cache.get_compute_pipeline_state(id) {
        CachedPipelineState::Ok(_) => {}
        CachedPipelineState::Err(PipelineCacheError::ShaderNotLoaded(_)) => return Ok(()),
        CachedPipelineState::Err(err) => panic!("ccl pipeline: {err}"),
        _ => return Ok(())
      }
    }
    let (Some(init), Some(union), Some(compress), Some(recolor)) = (
      cache.get_compute_pipeline(pipeline.init),
      cache.get_compute_pipeline(pipeline.union),
      cache.get_compute_pipeline(pipeline.compress),
      cache.get_compute_pipeline(pipeline.recolor)
    ) else {
      return Ok(());
    };
    let gx = res.size.x.div_ceil(WORKGROUP_SIZE);
    let gy = res.size.y.div_ceil(WORKGROUP_SIZE);
    let mut pass = render_context
      .command_encoder()
      .begin_compute_pass(&ComputePassDescriptor::default());
    pass.set_bind_group(0, &res.bind_group, &[]);
    pass.set_pipeline(init);
    pass.dispatch_workgroups(gx, gy, 1);
    for _ in 0..4 {
      pass.set_pipeline(union);
      pass.dispatch_workgroups(gx, gy, 1);
      pass.set_pipeline(compress);
      pass.dispatch_workgroups(gx, gy, 1);
    }
    pass.set_pipeline(recolor);
    pass.dispatch_workgroups(gx, gy, 1);
    Ok(())
  }
}
