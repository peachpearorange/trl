mod ui;
use trl::level;
mod combat;
mod crafting;
mod loot;
mod npcs;
mod utils;

use {bevy::prelude::*,
     combat::{TileEntityIndex, enemy_ai, maintain_tile_index},
     level::{FovGrid, Item, Tile, ZONE_HEIGHT, ZONE_WIDTH, compute_fov},
     std::collections::{HashMap, HashSet},
     trl::entities::{AirlockDoor, BlocksSight, Collidable, Dialogue, DialogueNode,
                     DialogueTree, Door, Enemy, FlightConsole, Glyph, Location, LootChest,
                     Named, Stats, Tree, Visuals},
     ui::{LogEntries, log_message}};

use trl::{active_zone::{self, ActiveZone},
          docking, galaxy, galaxy_gen, prefabs, ship,
          sprites::{PaletteImageCache, palette_sprite_handle}};

/// Tile art is authored at this resolution (e.g. space_qud masks).
pub const SPRITE_TEXELS: f32 = 20.0;
/// Each source pixel is drawn as this many screen pixels (integer scale).
pub const SCREEN_PIXELS_PER_TEXEL: f32 = 2.0;
/// World-space size of one grid cell (`Sprite` quad). Pixel-perfect when camera maps 1 world unit ≈ 1 screen pixel.
pub const TILE_SIZE: f32 = SPRITE_TEXELS * SCREEN_PIXELS_PER_TEXEL;
/// Palette-mask doors (`door closed (1).png` / `door open (2).png`).
const DOOR_CLOSED_PRI: Color = Color::srgb(0.34, 0.37, 0.41);
const DOOR_CLOSED_SEC: Color = Color::srgb(0.52, 0.55, 0.58);
const DOOR_OPEN_PRI: Color = Color::srgb(0.48, 0.55, 0.58);
const DOOR_OPEN_SEC: Color = Color::srgb(0.72, 0.78, 0.82);
/// Simulated 60Hz display: one grid step / one input gate spans this many render updates.
pub const RENDER_FRAMES_PER_SIM_STEP: u32 = 6;
const FOV_RADIUS: i32 = 99;
const DIM_FACTOR: f32 = 0.3;
/// Haalka layout: game view is left of the sidebar (`GAME_VIEWPORT_WIDTH_FRAC`); sidebar is
/// `SIDEBAR_WIDTH_FRAC`. Status bar is `STATUS_BAR_HEIGHT` along the bottom.
pub const GAME_VIEWPORT_WIDTH_FRAC: f32 = 0.70;
pub const SIDEBAR_WIDTH_FRAC: f32 = 1.0 - GAME_VIEWPORT_WIDTH_FRAC;
pub const STATUS_BAR_HEIGHT: f32 = 32.0;

// ---------------------------------------------------------------------------
// Player actions
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug)]
enum PlayerAction {
  Move { dx: i32, dy: i32 },
  Wait
}

impl PlayerAction {
  fn time_cost(self) -> u32 {
    match self {
      PlayerAction::Move { dx, dy } if dx != 0 && dy != 0 => 2,
      PlayerAction::Move { .. } => 1,
      PlayerAction::Wait => 1
    }
  }
}

// ---------------------------------------------------------------------------
// Time system
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TimeMode {
  RealTime,
  TurnBased
}

// ---------------------------------------------------------------------------
// Interaction menu
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
struct InteractionOption {
  label: String,
  action: InteractionAction
}

#[derive(Clone, Debug)]
enum InteractionAction {
  ToggleDoor(Entity),
  Talk { speaker: &'static str, tree: &'static DialogueTree },
  ChopTree(Entity),
  PickUpItem(i32, i32),
  OpenChest(Entity),
  Navigate { dest: galaxy::LocationId },
  Salvage(Item),
  Craft(usize)
}

#[derive(Default)]
pub enum InteractMenu {
  #[default]
  Closed,
  Open {
    options: Vec<InteractionOption>
  }
}

// ---------------------------------------------------------------------------
// Pause / Esc menu
// ---------------------------------------------------------------------------

#[derive(Default, PartialEq, Eq)]
enum PauseMenu {
  #[default]
  Closed,
  Main,
  Controls
}

// ---------------------------------------------------------------------------
// Dialogue state
// ---------------------------------------------------------------------------

#[derive(Default)]
enum DialogueState {
  #[default]
  Closed,
  Open {
    speaker: &'static str,
    tree: &'static DialogueTree,
    node_name: &'static str
  }
}

#[derive(Component)]
// ---------------------------------------------------------------------------
// Merged UI state
// ---------------------------------------------------------------------------
#[derive(Resource, Default)]
struct UiState {
  pause: PauseMenu,
  interact: InteractMenu,
  dialogue: DialogueState
}

impl UiState {
  fn any_open(&self) -> bool {
    self.pause != PauseMenu::Closed
      || matches!(self.interact, InteractMenu::Open { .. })
      || matches!(self.dialogue, DialogueState::Open { .. })
  }
}

// ---------------------------------------------------------------------------
// Merged timing — all progression is in integer render / sim units (no `Time::delta`)
// ---------------------------------------------------------------------------

/// Monotonic `Update` count (one step per game tick at ~60Hz).
#[derive(Resource, Default)]
pub struct RenderFrame(pub u64);

#[derive(Resource)]
pub struct Clock {
  /// Cumulative sim-time from actions and (in RT) periodic ticks.
  pub time: u64,
  pub mode: TimeMode,
  /// Renders left before another step is accepted.
  move_cooldown_frames: u32
}

/// When `true`, [`update_time_mode`] may switch to turn-based when enemies are near.
/// `T` sets a manual mode and sets this to `false`; `Shift+T` restores auto.
#[derive(Resource)]
pub struct TimeModeAuto(pub bool);

impl Clock {
  fn new() -> Self {
    Clock { time: 0, mode: TimeMode::RealTime, move_cooldown_frames: 0 }
  }

  fn advance(&mut self, cost: u32) {
    self.time = self.time.saturating_add(u64::from(cost));
  }
}

/// In turn-based mode, the world only advances in [`SimStep`] after a player spends a turn and
/// move animation finishes (`move_cooldown_frames == 0`); this flag schedules that one tick.
/// Cleared at the end of [`combat::enemy_ai`] after that tick runs.
#[derive(Resource, Default)]
pub struct TurnBasedWorldState {
  pub pending_enemy_phase: bool
}

/// Filled by [`player_input`] when a move is blocked; [`resolve_bump_interact`] reads it the same frame.
#[derive(Resource, Default)]
struct PendingBumpInteract(pub Option<(i32, i32, usize)>);

/// Set when the player picks "Open chest" from the interact menu; applied next frame.
#[derive(Resource, Default)]
struct ChestOpenPending(pub Option<Entity>);

/// Flight-console chart selection; applied next frame by [`apply_pending_navigation`].
#[derive(Resource, Default)]
struct PendingNavigation(pub Option<galaxy::LocationId>);

/// Single interaction chosen after bumping a blocked tile ([`resolve_bump_interact`] → [`apply_bump_auto_interact`]).
#[derive(Resource, Default)]
struct BumpInteractFlash(pub Option<InteractionOption>);

fn note_player_turn_moved_world(clock: &Clock, tb: &mut TurnBasedWorldState) {
  if clock.mode == TimeMode::TurnBased {
    tb.pending_enemy_phase = true;
  }
}

/// Increments the display frame and, in real-time mode, advances the sim clock every
/// [`RENDER_FRAMES_PER_SIM_STEP`] frames (same ordering as the former separate systems).
fn bump_render_frame(mut frame: ResMut<RenderFrame>, mut clock: ResMut<Clock>) {
  frame.0 = frame.0.saturating_add(1);
  if clock.mode == TimeMode::RealTime
    && frame.0 > 0
    && frame.0 % u64::from(RENDER_FRAMES_PER_SIM_STEP) == 0
  {
    clock.time = clock.time.saturating_add(1);
  }
}

/// Real-time: one sim step every [`RENDER_FRAMES_PER_SIM_STEP`] display frames. Turn-based: a single
/// sim tick after the player’s action when animation is done (same as when enemies may act in RT).
fn should_run_sim_step(
  frame: Res<RenderFrame>,
  clock: Res<Clock>,
  tb: Res<TurnBasedWorldState>
) -> bool {
  if clock.mode == TimeMode::RealTime {
    frame.0 > 0 && frame.0 % u64::from(RENDER_FRAMES_PER_SIM_STEP) == 0
  } else {
    tb.pending_enemy_phase && clock.move_cooldown_frames == 0
  }
}

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
enum FramePipeline {
  ApplyNavigation,
  BumpRender,
  TileIndex,
  GlyphSetup,
  TimeMode,
  DialogueKey,
  Menus,
  FlushChest,
  InteractKey,
  UtilityMenus,
  PlayerMove,
  BumpResolve
}

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
struct SimStep;

// ---------------------------------------------------------------------------
// Resources & components
// ---------------------------------------------------------------------------

#[derive(Resource)]
pub struct CurrentZone(pub active_zone::ActiveZone);

#[derive(Resource)]
pub struct Fov(pub FovGrid);

#[derive(Component)]
pub struct Player;

#[derive(Component)]
pub struct PlayerPos {
  pub x: i32,
  pub y: i32,
  pub z: usize
}

#[derive(Component)]
struct TileGlyph {
  x: usize,
  y: usize
}

/// Marks a tile entity that uses a PNG sprite instead of a text glyph.
#[derive(Component)]
struct TilePng;

#[derive(Component)]
struct ItemGlyph {
  x: usize,
  y: usize,
  z: usize
}

#[derive(Component, Default)]
pub struct Inventory(pub HashMap<Item, u32>);

/// Marker for entities that use [`Glyph`] visuals (tile sprite or [`Text2d`]).
#[derive(Component)]
struct GlyphVisual;

/// Semi-transparent cell highlight following the cursor over the current zone.
#[derive(Component)]
struct TileHoverHighlight;

// ---------------------------------------------------------------------------
// Glyph rendering systems
// ---------------------------------------------------------------------------

/// `true` if `a` and `b` are the same or orthogonally/diagonally adjacent on the grid.
/// If the logical local tile jumps further (e.g. zone wrap: 0 → 47), a lerp would cross the
/// whole zone; we treat that as a **snap** instead of a slide.
fn is_adjacent_or_same_local(a: Vec2, b: Vec2) -> bool {
  let d = (a - b).abs();
  d.x <= 1.0 && d.y <= 1.0
}

fn apply_visuals_move(vis: &mut Visuals, f: u64, local: Vec2) {
  if is_adjacent_or_same_local(local, vis.last_pos) {
    if (local - vis.last_pos).length_squared() > 0.5 {
      vis.prev = vis.display;
      vis.last_move_start_frame = Some(f);
      vis.last_pos = local;
    }
  } else {
    vis.prev = local;
    vis.display = local;
    vis.last_pos = local;
    vis.last_move_start_frame = None;
  }
}

/// After movement systems run, snapshot position changes into Visuals.
/// When an entity's Location changes, `prev` snaps to the current display pos
/// (so direction changes pivot smoothly) and the move timer resets.
fn track_movement(
  frame: Res<RenderFrame>,
  mut params: ParamSet<(
    Query<(&Location, &mut Visuals), Without<Player>>,
    Query<(&PlayerPos, &mut Visuals), With<Player>>
  )>
) {
  let f = frame.0;
  for (loc, mut vis) in params.p0().iter_mut() {
    if let Some(world_pos) = loc.as_vec2() {
      let local = Vec2::new(world_pos.x, world_pos.y);
      apply_visuals_move(&mut vis, f, local);
    }
  }
  if let Ok((pos, mut vis)) = params.p1().single_mut() {
    let local = Vec2::new(pos.x as f32, pos.y as f32);
    apply_visuals_move(&mut vis, f, local);
  }
}

/// One slide is [`RENDER_FRAMES_PER_SIM_STEP`] display frames with `t = (e + 1) / n` for
/// `e` in `0..n` (e.g. 1/6…1). The prior `t = e / (n - 1)` had `t = 0` on the first frame of
/// each move, which matched the previous move’s `t = 1` (same `display`), so the camera held
/// one extra frame on every grid integer while walking. First frame of a move now already
/// moves toward `local` (no zero lerp step).
fn interpolate_visual_one(vis: &mut Visuals, f: u64, local: Vec2) {
  if let Some(start) = vis.last_move_start_frame {
    let e = f.saturating_sub(start);
    let n = u64::from(RENDER_FRAMES_PER_SIM_STEP);
    if e >= n {
      vis.last_move_start_frame = None;
      vis.prev = local;
      vis.display = local;
    } else {
      let t = ((e + 1) as f32 / n as f32).min(1.0);
      vis.display = vis.prev.lerp(local, t);
      if t >= 1.0 {
        vis.last_move_start_frame = None;
        vis.prev = local;
        vis.display = local;
      }
    }
  } else if (vis.display - local).length_squared() > 1.0e-4 {
    vis.display = local;
  }
}

/// Each frame, compute interpolated display position: lerp from `prev` to current
/// local tile (see [`track_movement`]).
fn interpolate_visual_positions(
  frame: Res<RenderFrame>,
  mut params: ParamSet<(
    Query<(&Location, &mut Visuals), Without<Player>>,
    Query<(&PlayerPos, &mut Visuals), With<Player>>
  )>
) {
  let f = frame.0;
  for (loc, mut vis) in params.p0().iter_mut() {
    if let Some(world_pos) = loc.as_vec2() {
      let local = Vec2::new(world_pos.x, world_pos.y);
      interpolate_visual_one(&mut vis, f, local);
    }
  }
  if let Ok((pos, mut vis)) = params.p1().single_mut() {
    let local = Vec2::new(pos.x as f32, pos.y as f32);
    interpolate_visual_one(&mut vis, f, local);
  }
}

fn setup_glyph_visuals(
  mut commands: Commands,
  asset_server: Res<AssetServer>,
  mut images: ResMut<Assets<Image>>,
  mut palette_cache: ResMut<PaletteImageCache>,
  query: Query<(Entity, &Glyph, &Location), (Added<Glyph>, Without<GlyphVisual>)>
) {
  for (entity, glyph, location) in query.iter() {
    if let Location::Coords { x, y, .. } = location {
      let lx = *x;
      let ly = *y;
      let local = Vec2::new(lx as f32, ly as f32);
      let pos = tile_screen_pos(lx as f32, ly as f32, ZONE_WIDTH, ZONE_HEIGHT)
        + Vec3::new(0.0, 0.0, 2.0);
      if let Some(path) = glyph.texture {
        let img = if let Some((primary, secondary)) = glyph.sprite_palette {
          palette_sprite_handle(path, primary, secondary, &mut palette_cache, &mut images)
        } else {
          asset_server.load(path)
        };
        commands.entity(entity).insert((
          Sprite {
            image: img,
            custom_size: Some(Vec2::splat(TILE_SIZE)),
            color: Color::WHITE,
            ..default()
          },
          Transform::from_translation(pos),
          GlyphVisual,
          Visuals {
            prev: local,
            last_move_start_frame: None,
            display: local,
            last_pos: local
          }
        ));
      } else {
        commands.entity(entity).insert((
          Text2d::new(glyph.ch.to_string()),
          TextFont { font_size: TILE_SIZE, ..default() },
          TextColor(glyph.color),
          Transform::from_translation(pos),
          GlyphVisual,
          Visuals {
            prev: local,
            last_move_start_frame: None,
            display: local,
            last_pos: local
          }
        ));
      }
    }
  }
}

fn sync_entity_positions(
  mut query: Query<(&Visuals, &mut Transform), With<GlyphVisual>>
) {
  for (vis, mut transform) in query.iter_mut() {
    transform.translation =
      tile_screen_pos(vis.display.x, vis.display.y, ZONE_WIDTH, ZONE_HEIGHT)
        + Vec3::new(0.0, 0.0, 2.0);
  }
}

fn sync_player_location(mut player_q: Query<(&PlayerPos, &mut Location), With<Player>>) {
  if let Ok((pos, mut location)) = player_q.single_mut() {
    *location = Location::xyz(pos.x, pos.y, pos.z);
  }
}

// ---------------------------------------------------------------------------
// App
// ---------------------------------------------------------------------------

fn main() {
  let mut galaxy = galaxy::Galaxy::new();
  let ship_id: galaxy::LocationId = (-1, -1, -1);

  let ship_location = ship::build_ship_interior();
  galaxy.insert(ship_id, ship_location.clone());

  // Add starter planet at origin
  let origin: galaxy::LocationId = galaxy_gen::ID_STARTER_PLANET;
  let starter_planet = galaxy_gen::generate_starter_planet();
  galaxy.insert(origin, starter_planet.clone());
  galaxy.insert(galaxy_gen::ID_ASTEROID_FIELD, galaxy_gen::generate_asteroid_field());

  // Ship starts docked at the starter planet
  let active = active_zone::ActiveZone::docked(
    &ship_location,
    &starter_planet,
    0 // first landing spot
  )
  .expect("ship should dock at starter planet");

  let ship_res = ship::Ship {
    location_id: ship_id,
    docked_at: Some(origin),
    fuel: 500,
    max_fuel: 500
  };

  let fov = level::FovGrid::new(active.width, active.height);

  let _ = &active; // Keep 'active' in scope for init

  App::new()
    .add_plugins(haalka::HaalkaPlugin::default())
    .add_plugins(DefaultPlugins.set(ImagePlugin::default_nearest()).set(WindowPlugin {
      primary_window: Some(Window {
        title: format!("{} — space", ship::SHIP_NAME).into(),
        resolution: (1200u32, 800u32).into(),
        ..default()
      }),
      ..default()
    }))
    .insert_resource(ClearColor(Color::srgb(0.02, 0.02, 0.05)))
    .insert_resource(galaxy)
    .insert_resource(ship_res)
    .insert_resource(CurrentZone(active))
    .init_resource::<RenderFrame>()
    .init_resource::<TurnBasedWorldState>()
    .insert_resource(Clock::new())
    .insert_resource(TimeModeAuto(true))
    .init_resource::<ChestOpenPending>()
    .init_resource::<PendingNavigation>()
    .init_resource::<BumpInteractFlash>()
    .init_resource::<PendingBumpInteract>()
    .init_resource::<PaletteImageCache>()
    .insert_resource(UiState::default())
    .insert_resource(Fov(fov))
    .insert_resource(TileEntityIndex::default())
    .add_plugins(ui::UiPlugin)
    .add_systems(Startup, (setup, ui::spawn_haalka_root).chain())
    .configure_sets(Update, SimStep.run_if(should_run_sim_step))
    .configure_sets(
      Update,
      (
        FramePipeline::ApplyNavigation,
        FramePipeline::BumpRender,
        FramePipeline::TileIndex,
        FramePipeline::GlyphSetup,
        FramePipeline::TimeMode,
        FramePipeline::DialogueKey,
        FramePipeline::Menus,
        FramePipeline::FlushChest,
        FramePipeline::InteractKey,
        FramePipeline::UtilityMenus,
        FramePipeline::PlayerMove,
        FramePipeline::BumpResolve
      )
        .chain()
    )
    .add_systems(Update, apply_pending_navigation.in_set(FramePipeline::ApplyNavigation))
    .add_systems(
      Update,
      (bump_render_frame, sync_player_location).chain().in_set(FramePipeline::BumpRender)
    )
    .add_systems(Update, maintain_tile_index.in_set(FramePipeline::TileIndex))
    .add_systems(Update, setup_glyph_visuals.in_set(FramePipeline::GlyphSetup))
    .add_systems(Update, update_time_mode.in_set(FramePipeline::TimeMode))
    .add_systems(Update, handle_dialogue.in_set(FramePipeline::DialogueKey))
    .add_systems(Update, handle_menus.in_set(FramePipeline::Menus))
    .add_systems(Update, flush_pending_chest_open.in_set(FramePipeline::FlushChest))
    .add_systems(Update, handle_interact.in_set(FramePipeline::InteractKey))
    .add_systems(Update, handle_utility_menus.in_set(FramePipeline::UtilityMenus))
    .add_systems(Update, player_input.in_set(FramePipeline::PlayerMove))
    .add_systems(Update, auto_close_airlocks.after(FramePipeline::PlayerMove))
    .add_systems(
      Update,
      (resolve_bump_interact, apply_bump_auto_interact)
        .chain()
        .in_set(FramePipeline::BumpResolve)
    )
    .add_systems(
      Update,
      (
        ApplyDeferred,
        enemy_death_check.in_set(SimStep),
        enemy_ai.in_set(SimStep),
        update_fov.in_set(SimStep)
      )
        .chain()
    )
    .add_systems(
      Update,
      (
        track_movement,
        interpolate_visual_positions,
        sync_entity_positions,
        camera_follow,
        update_fov_visuals,
        hide_tile_under_occupants,
        update_tile_hover_highlight
      )
        .chain()
    )
    .run();
}

// ---------------------------------------------------------------------------
// Coordinate helpers
// ---------------------------------------------------------------------------

fn tile_screen_pos(x: f32, y: f32, w: usize, h: usize) -> Vec3 {
  Vec3::new((x - w as f32 / 2.0) * TILE_SIZE, (h as f32 / 2.0 - y) * TILE_SIZE, 0.0)
}

/// Game pane logical bounds (left portion of window, above status bar).
/// Camera renders full-window so UI isn't viewport-clipped; this is used for cursor picking.
pub(crate) fn game_pane_rect(w: &Window) -> Rect {
  Rect::new(
    0.0,
    0.0,
    w.resolution.width() * GAME_VIEWPORT_WIDTH_FRAC,
    w.resolution.height() - STATUS_BAR_HEIGHT
  )
}

/// Inverse of [`tile_screen_pos`] for a point in world: which level cell it falls into.
/// World units use `TILE_SIZE` and the same origin as the camera-facing grid.
pub(crate) fn world_to_level_cell(world: Vec2, w: usize, h: usize) -> (i32, i32) {
  // Tiny bias avoids float edge cases on cell boundaries.
  const E: f32 = 1.0e-4;
  let tx = (world.x / TILE_SIZE + w as f32 * 0.5 - E).floor() as i32;
  let ty = (h as f32 * 0.5 - world.y / TILE_SIZE - E).floor() as i32;
  (tx, ty)
}

// ---------------------------------------------------------------------------
// Gravity
// ---------------------------------------------------------------------------

/// Despawn enemies with 0 or fewer HP. Runs in SimStep so dead enemies
/// are removed before the next frame's rendering and AI.
fn enemy_death_check(
  mut commands: Commands,
  enemy_q: Query<(Entity, &Stats), With<Enemy>>
) {
  for (entity, stats) in enemy_q.iter() {
    if stats.hp <= 0 {
      commands.entity(entity).despawn();
    }
  }
}

// ---------------------------------------------------------------------------
// Camera follow
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// FOV visuals
// ---------------------------------------------------------------------------

fn white_pixel_image(images: &mut Assets<Image>) -> Handle<Image> {
  use bevy::{asset::RenderAssetUsages,
             render::render_resource::{Extent3d, TextureDimension, TextureFormat}};
  images.add(Image::new(
    Extent3d { width: 1, height: 1, depth_or_array_layers: 1 },
    TextureDimension::D2,
    vec![255, 255, 255, 255],
    TextureFormat::Rgba8UnormSrgb,
    RenderAssetUsages::RENDER_WORLD
  ))
}

fn update_tile_hover_highlight(
  windows: Query<&Window>,
  camera_q: Query<(&Camera, &GlobalTransform), With<Camera2d>>,
  current: Res<CurrentZone>,
  fov: Res<Fov>,
  player_q: Query<&PlayerPos, With<Player>>,
  mut q: Query<(&mut Transform, &mut Visibility), With<TileHoverHighlight>>
) {
  if let Ok((mut transform, mut vis)) = q.single_mut() {
    *vis = Visibility::Hidden;
    if let Ok(window) = windows.single()
      && let Ok((camera, cam_transform)) = camera_q.single()
      && let Ok(player_pos) = player_q.single()
    {
      let level = current.0.level(player_pos.z);
      let pick = |w: &Window,
                  c: &Camera,
                  ct: &GlobalTransform,
                  lw: usize,
                  lh: usize|
       -> Option<(i32, i32)> {
        let cursor = w.cursor_position()?;
        game_pane_rect(w)
          .contains(cursor)
          .then(|| c.viewport_to_world_2d(ct, cursor).ok())
          .flatten()
          .map(|world| world_to_level_cell(world, lw, lh))
          .filter(|&(tx, ty)| {
            tx >= 0 && ty >= 0 && (tx as usize) < lw && (ty as usize) < lh
          })
      };
      if let Some((tx, ty)) =
        pick(window, camera, cam_transform, level.width, level.height)
      {
        let visible = fov.0.is_visible(tx as usize, ty as usize);
        let revealed = fov.0.is_revealed(tx as usize, ty as usize);
        if visible || revealed {
          *vis = Visibility::Visible;
          transform.translation =
            tile_screen_pos(tx as f32, ty as f32, current.0.width, current.0.height)
              + Vec3::new(0.0, 0.0, 0.25);
        }
      }
    }
  }
}

fn update_fov_visuals(
  fov: Res<Fov>,
  current: Res<CurrentZone>,
  frame: Res<RenderFrame>,
  index: Res<TileEntityIndex>,
  mut player_q: Query<(Entity, &PlayerPos, &mut Visibility), With<Player>>,
  mut glyph_tiles: Query<
    (&TileGlyph, &mut TextColor),
    (Without<TilePng>, Without<ItemGlyph>)
  >,
  mut sprite_tiles: Query<(&TileGlyph, &mut Sprite), With<TilePng>>,
  mut item_q: Query<
    (Entity, &ItemGlyph, &mut TextColor, &mut Visibility),
    (Without<Player>, Without<GlyphVisual>)
  >,
  mut entity_q: Query<
    (Entity, &Location, &mut Visibility),
    (With<GlyphVisual>, Without<Player>)
  >
) {
  if let Ok((player_ent, pos, mut player_vis)) = player_q.single_mut() {
    let level = current.0.level(pos.z);
    let z = pos.z;
    let t = frame.0 as f32 * 0.052;
    let tau = std::f32::consts::TAU;
    let mut stacks: HashMap<(i32, i32), Vec<Entity>> = HashMap::new();
    stacks.entry((pos.x, pos.y)).or_default().push(player_ent);
    for (&(x, y, zz), ents) in index.0.iter() {
      if zz != z {
        continue;
      }
      stacks.entry((x, y)).or_default().extend(ents.iter().copied());
    }
    for (entity, item, _, _) in item_q.iter_mut() {
      if item.z == z {
        stacks.entry((item.x as i32, item.y as i32)).or_default().push(entity);
      }
    }
    for ents in stacks.values_mut() {
      ents.sort_by_key(|e| e.index());
    }

    for (tg, mut color) in glyph_tiles.iter_mut() {
      let tile = level.tiles[tg.y][tg.x];
      let [r, g, b] = tile.color();
      *color = if fov.0.is_visible(tg.x, tg.y) {
        TextColor(Color::srgb(r, g, b))
      } else if fov.0.is_revealed(tg.x, tg.y) {
        TextColor(Color::srgb(r * DIM_FACTOR, g * DIM_FACTOR, b * DIM_FACTOR))
      } else {
        TextColor(Color::srgba(0.0, 0.0, 0.0, 0.0))
      };
    }
    for (tg, mut sprite) in sprite_tiles.iter_mut() {
      sprite.color = if fov.0.is_visible(tg.x, tg.y) {
        Color::WHITE
      } else if fov.0.is_revealed(tg.x, tg.y) {
        Color::srgb(DIM_FACTOR, DIM_FACTOR, DIM_FACTOR)
      } else {
        Color::srgba(0.0, 0.0, 0.0, 0.0)
      };
    }
    for (entity, location, mut vis) in entity_q.iter_mut() {
      let visible_in_fov = if let Location::Coords { x, y, z: lz, .. } = location
        && *lz == pos.z
        && fov.0.is_visible(*x as usize, *y as usize)
      {
        true
      } else {
        false
      };
      if !visible_in_fov {
        *vis = Visibility::Hidden;
        continue;
      }
      let Location::Coords { x, y, .. } = location else {
        *vis = Visibility::Hidden;
        continue;
      };
      let key = (*x, *y);
      if let Some(list) = stacks.get(&key)
        && list.len() > 1
      {
        let n = list.len() as f32;
        let winner = list
          .iter()
          .enumerate()
          .max_by(|(i, _), (j, _)| {
            let a = (t + *i as f32 * tau / n).sin();
            let b = (t + *j as f32 * tau / n).sin();
            a.total_cmp(&b)
          })
          .map(|(_, &e)| e)
          .unwrap_or(entity);
        *vis = if entity == winner { Visibility::Visible } else { Visibility::Hidden };
      } else {
        *vis = Visibility::Visible;
      }
    }
    for (entity, item, mut color, mut vis) in item_q.iter_mut() {
      let visible_in_fov = item.z == pos.z && fov.0.is_visible(item.x, item.y);
      let revealed = item.z == pos.z && fov.0.is_revealed(item.x, item.y);
      let item_kind = level.items[item.y][item.x];
      *color = item_kind.map_or(TextColor(Color::NONE), |item_kind| {
        let [r, g, b] = item_kind.color();
        if visible_in_fov {
          TextColor(Color::srgb(r, g, b))
        } else if revealed {
          TextColor(Color::srgb(r * DIM_FACTOR, g * DIM_FACTOR, b * DIM_FACTOR))
        } else {
          TextColor(Color::NONE)
        }
      });
      if !visible_in_fov {
        *vis = Visibility::Hidden;
      } else if let Some(list) = stacks.get(&(item.x as i32, item.y as i32))
        && list.len() > 1
      {
        let n = list.len() as f32;
        let winner = list
          .iter()
          .enumerate()
          .max_by(|(i, _), (j, _)| {
            let a = (t + *i as f32 * tau / n).sin();
            let b = (t + *j as f32 * tau / n).sin();
            a.total_cmp(&b)
          })
          .map(|(_, &e)| e)
          .unwrap_or(entity);
        *vis = if entity == winner { Visibility::Visible } else { Visibility::Hidden };
      } else {
        *vis = Visibility::Visible;
      }
    }
    let key = (pos.x, pos.y);
    *player_vis = stacks.get(&key).map_or(Visibility::Visible, |list| {
      if list.len() <= 1 {
        Visibility::Visible
      } else {
        let n = list.len() as f32;
        let winner = list
          .iter()
          .enumerate()
          .max_by(|(i, _), (j, _)| {
            let a = (t + *i as f32 * tau / n).sin();
            let b = (t + *j as f32 * tau / n).sin();
            a.total_cmp(&b)
          })
          .map(|(_, &e)| e)
          .unwrap_or(player_ent);
        if player_ent == winner { Visibility::Visible } else { Visibility::Hidden }
      }
    });
  }
}

/// Hide the base floor tile when any object entity or floor item occupies the cell.
fn hide_tile_under_occupants(
  index: Res<TileEntityIndex>,
  current: Res<CurrentZone>,
  player_q: Query<&PlayerPos, With<Player>>,
  mut tile_png: Query<(&TileGlyph, &mut Visibility), With<TilePng>>,
  mut tile_text: Query<
    (&TileGlyph, &mut Visibility),
    (Without<TilePng>, Without<ItemGlyph>)
  >
) {
  let Ok(pos) = player_q.single() else {
    return;
  };
  let level = current.0.level(pos.z);
  let occluded = |ix: i32, iy: i32| {
    index.0.get(&(ix, iy, pos.z)).is_some_and(|ents| !ents.is_empty())
      || (iy >= 0
        && ix >= 0
        && (iy as usize) < level.height
        && (ix as usize) < level.width
        && level.items[iy as usize][ix as usize].is_some())
  };
  for (tg, mut vis) in tile_png.iter_mut() {
    let ix = tg.x as i32;
    let iy = tg.y as i32;
    *vis = if occluded(ix, iy) { Visibility::Hidden } else { Visibility::Visible };
  }
  for (tg, mut vis) in tile_text.iter_mut() {
    let ix = tg.x as i32;
    let iy = tg.y as i32;
    *vis = if occluded(ix, iy) { Visibility::Hidden } else { Visibility::Visible };
  }
}

// ---------------------------------------------------------------------------
// Time
// ---------------------------------------------------------------------------

/// In real-time mode, one abstract time tick every [`RENDER_FRAMES_PER_SIM_STEP`] display frames.
const ENEMY_ALERT_RADIUS: i32 = 8;

fn update_time_mode(
  mut clock: ResMut<Clock>,
  time_mode_auto: Res<TimeModeAuto>,
  player_q: Query<&PlayerPos, With<Player>>,
  enemy_q: Query<&Location, With<Enemy>>
) {
  if !time_mode_auto.0 {
    return;
  }
  let enemy_near = player_q.single().is_ok_and(|pos| {
    enemy_q.iter().any(|loc| {
      if let Location::Coords { x, y, z, .. } = *loc {
        z == pos.z
          && (x - pos.x).abs() <= ENEMY_ALERT_RADIUS
          && (y - pos.y).abs() <= ENEMY_ALERT_RADIUS
      } else {
        false
      }
    })
  });
  clock.mode = if enemy_near { TimeMode::TurnBased } else { TimeMode::RealTime };
}

// ---------------------------------------------------------------------------
// Input helpers
// ---------------------------------------------------------------------------

fn read_direction(keys: &ButtonInput<KeyCode>) -> (i32, i32) {
  let up = keys.pressed(KeyCode::KeyW) || keys.pressed(KeyCode::ArrowUp);
  let down = keys.pressed(KeyCode::KeyS) || keys.pressed(KeyCode::ArrowDown);
  let left = keys.pressed(KeyCode::KeyA) || keys.pressed(KeyCode::ArrowLeft);
  let right = keys.pressed(KeyCode::KeyD) || keys.pressed(KeyCode::ArrowRight);

  let dx = match (left, right) {
    (true, false) => -1,
    (false, true) => 1,
    _ => 0
  };
  let dy = match (up, down) {
    (true, false) => -1,
    (false, true) => 1,
    _ => 0
  };
  (dx, dy)
}

fn any_direction_pressed(keys: &ButtonInput<KeyCode>) -> bool {
  keys.pressed(KeyCode::KeyW)
    || keys.pressed(KeyCode::KeyA)
    || keys.pressed(KeyCode::KeyS)
    || keys.pressed(KeyCode::KeyD)
    || keys.pressed(KeyCode::ArrowUp)
    || keys.pressed(KeyCode::ArrowDown)
    || keys.pressed(KeyCode::ArrowLeft)
    || keys.pressed(KeyCode::ArrowRight)
}

fn any_direction_just_pressed(keys: &ButtonInput<KeyCode>) -> bool {
  keys.just_pressed(KeyCode::KeyW)
    || keys.just_pressed(KeyCode::KeyA)
    || keys.just_pressed(KeyCode::KeyS)
    || keys.just_pressed(KeyCode::KeyD)
    || keys.just_pressed(KeyCode::ArrowUp)
    || keys.just_pressed(KeyCode::ArrowDown)
    || keys.just_pressed(KeyCode::ArrowLeft)
    || keys.just_pressed(KeyCode::ArrowRight)
}

fn resolve_move(level: &level::Level, px: i32, py: i32, dx: i32, dy: i32) -> (i32, i32) {
  if level.walkable(px + dx, py + dy) {
    (dx, dy)
  } else if dx != 0 && dy != 0 {
    if level.walkable(px + dx, py) {
      (dx, 0)
    } else if level.walkable(px, py + dy) {
      (0, dy)
    } else {
      (0, 0)
    }
  } else {
    (0, 0)
  }
}

// ---------------------------------------------------------------------------
// Pause / Esc menu
// ---------------------------------------------------------------------------

fn door_glyph(open: bool) -> Glyph {
  if open {
    Glyph::palette_sprite(
      "textures/space_qud/door open (2).png",
      '/',
      DOOR_OPEN_PRI,
      DOOR_OPEN_SEC
    )
  } else {
    Glyph::palette_sprite(
      "textures/space_qud/door closed (1).png",
      '+',
      DOOR_CLOSED_PRI,
      DOOR_CLOSED_SEC
    )
  }
}

fn set_door_open_state(
  commands: &mut Commands,
  entity: Entity,
  door: &mut Door,
  glyph: &mut Glyph,
  location: &Location,
  open: bool,
  airlock: Option<&mut AirlockDoor>,
  clock_time: u64,
  palette_cache: &mut PaletteImageCache,
  images: &mut Assets<Image>,
  asset_server: &AssetServer
) {
  door.open = open;
  if open {
    commands.entity(entity).remove::<Collidable>();
    commands.entity(entity).remove::<BlocksSight>();
  } else {
    commands.entity(entity).insert((Collidable(true), BlocksSight));
  }
  if let Some(airlock) = airlock {
    airlock.opened_at_sim_time = open.then_some(clock_time);
  }
  *glyph = door_glyph(open);
  if let Location::Coords { x, y, .. } = *location {
    let lx = x as f32;
    let ly = y as f32;
    let local = Vec2::new(lx, ly);
    let pos = tile_screen_pos(lx, ly, ZONE_WIDTH, ZONE_HEIGHT) + Vec3::new(0.0, 0.0, 2.0);
    commands.entity(entity).remove::<Sprite>();
    commands.entity(entity).remove::<Text2d>();
    commands.entity(entity).remove::<TextFont>();
    commands.entity(entity).remove::<TextColor>();
    commands.entity(entity).remove::<GlyphVisual>();
    commands.entity(entity).remove::<Visuals>();
    if let Some(path) = glyph.texture {
      let img = if let Some((pri, sec)) = glyph.sprite_palette {
        palette_sprite_handle(path, pri, sec, palette_cache, images)
      } else {
        asset_server.load(path)
      };
      commands.entity(entity).insert((
        Sprite {
          image: img,
          custom_size: Some(Vec2::splat(TILE_SIZE)),
          color: Color::WHITE,
          ..default()
        },
        Transform::from_translation(pos),
        GlyphVisual,
        Visuals {
          prev: local,
          last_move_start_frame: None,
          display: local,
          last_pos: local
        }
      ));
    } else {
      commands.entity(entity).insert((
        Text2d::new(glyph.ch.to_string()),
        TextFont { font_size: TILE_SIZE, ..default() },
        TextColor(glyph.color),
        Transform::from_translation(pos),
        GlyphVisual,
        Visuals {
          prev: local,
          last_move_start_frame: None,
          display: local,
          last_pos: local
        }
      ));
    }
  }
}

fn handle_menus(
  keys: Res<ButtonInput<KeyCode>>,
  mut ui: ResMut<UiState>,
  mut commands: Commands,
  mut gw: ResMut<CurrentZone>,
  mut clock: ResMut<Clock>,
  mut tb: ResMut<TurnBasedWorldState>,
  mut log: ResMut<LogEntries>,
  mut player_query: Query<(&mut PlayerPos, &mut Inventory), With<Player>>,
  asset_server: Res<AssetServer>,
  mut palette_cache: ResMut<PaletteImageCache>,
  mut images: ResMut<Assets<Image>>,
  mut door_q: Query<(
    &mut Door,
    &mut Glyph,
    Option<&mut Collidable>,
    &Location,
    Option<&mut AirlockDoor>
  )>,
  mut pending_chest: ResMut<ChestOpenPending>,
  mut pending_nav: ResMut<PendingNavigation>,
  mut exit: MessageWriter<AppExit>
) {
  if let InteractMenu::Open { options } = &ui.interact {
    if keys.just_pressed(KeyCode::Escape) {
      ui.interact = InteractMenu::Closed;
    } else if let Some(idx) = [
      KeyCode::Digit1,
      KeyCode::Digit2,
      KeyCode::Digit3,
      KeyCode::Digit4,
      KeyCode::Digit5,
      KeyCode::Digit6,
      KeyCode::Digit7,
      KeyCode::Digit8,
      KeyCode::Digit9
    ]
    .iter()
    .position(|k| keys.just_pressed(*k))
      && idx < options.len()
    {
      let option = options[idx].clone();
      ui.interact = InteractMenu::Closed;
      dispatch_interactive_choice(
        option,
        &mut commands,
        &mut gw.0,
        &mut clock,
        &mut tb,
        &mut ui,
        &mut log,
        &mut player_query,
        &mut pending_chest,
        &mut pending_nav,
        &mut door_q,
        &asset_server,
        &mut palette_cache,
        &mut images
      );
    }
  } else {
    match ui.pause {
      PauseMenu::Closed => {
        let open = keys.just_pressed(KeyCode::Escape)
          || (keys.just_pressed(KeyCode::Slash)
            && (keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight)));

        if open {
          ui.pause = if keys.just_pressed(KeyCode::Slash) {
            PauseMenu::Controls
          } else {
            PauseMenu::Main
          };
        }
      }
      PauseMenu::Main => {
        if keys.just_pressed(KeyCode::Escape) || keys.just_pressed(KeyCode::Digit1) {
          ui.pause = PauseMenu::Closed;
        } else if keys.just_pressed(KeyCode::Digit2) {
          ui.pause = PauseMenu::Controls;
        } else if keys.just_pressed(KeyCode::Digit3) {
          exit.write(AppExit::Success);
        }
      }
      PauseMenu::Controls => {
        if keys.just_pressed(KeyCode::Escape) {
          ui.pause = PauseMenu::Main;
        }
      }
    }
  }
}

fn log_dialogue_node_block(log: &mut LogEntries, speaker: &str, node: &DialogueNode) {
  log_message(log, format!("{speaker}: {}", node.text));
}

fn handle_dialogue(
  keys: Res<ButtonInput<KeyCode>>,
  mut ui: ResMut<UiState>,
  mut log: ResMut<LogEntries>
) {
  // Digit keys are shared with the interact list; `handle_menus` runs after us. While the
  // interact overlay is up, that same key would otherwise apply to dialogue the same frame.
  if matches!(&ui.interact, InteractMenu::Open { .. }) {
    return;
  }

  if let DialogueState::Open { speaker, tree, node_name } = &ui.dialogue {
    let (speaker, tree, node_name) = (*speaker, *tree, *node_name);
    let node = tree.find(node_name);
    if keys.just_pressed(KeyCode::Escape) {
      ui.dialogue = DialogueState::Closed;
    } else if let Some(idx) = [
      KeyCode::Digit1,
      KeyCode::Digit2,
      KeyCode::Digit3,
      KeyCode::Digit4,
      KeyCode::Digit5,
      KeyCode::Digit6,
      KeyCode::Digit7,
      KeyCode::Digit8,
      KeyCode::Digit9
    ]
    .iter()
    .position(|k| keys.just_pressed(*k))
      && idx < node.choices.len()
    {
      let choice = &node.choices[idx];
      log_message(&mut *log, format!("You: {}", choice.text));
      if let Some(next_name) = choice.next {
        ui.dialogue = DialogueState::Open { speaker, tree, node_name: next_name };
        let next_node = tree.find(next_name);
        log_dialogue_node_block(&mut *log, speaker, next_node);
      } else {
        ui.dialogue = DialogueState::Closed;
      }
    }
  }
}

// ---------------------------------------------------------------------------
// Interaction menu
// ---------------------------------------------------------------------------

fn direction_name(dx: i32, dy: i32) -> String {
  match (dx, dy) {
    (0, -1) => "N",
    (0, 1) => "S",
    (-1, 0) => "W",
    (1, 0) => "E",
    (-1, -1) => "NW",
    (1, -1) => "NE",
    (-1, 1) => "SW",
    (1, 1) => "SE",
    _ => "?"
  }
  .to_string()
}

fn apply_open_chest(
  commands: &mut Commands,
  entity: Entity,
  player_query: &mut Query<(&mut PlayerPos, &mut Inventory), With<Player>>,
  loot_chest_q: &mut Query<(&mut LootChest, &mut Glyph, &Location)>,
  log: &mut LogEntries,
  clock: &mut Clock,
  tb: &mut TurnBasedWorldState
) {
  if let Ok((mut chest, mut glyph, loc)) = loot_chest_q.get_mut(entity)
    && !chest.opened
    && let Ok((_, mut inventory)) = player_query.single_mut()
    && let &Location::Coords { x: cx, y: cy, z: cz, .. } = loc
  {
    let bundles = loot::roll_chest_loot(42u64, cx, cy, cz);
    let kinds = bundles.len();
    for (item, qty) in bundles {
      *inventory.0.entry(item).or_insert(0) += qty;
    }
    chest.opened = true;
    glyph.texture = None;
    glyph.sprite_palette = None;
    glyph.ch = '□';
    glyph.color = Color::srgb(0.45, 0.38, 0.32);
    commands.entity(entity).remove::<Sprite>();
    commands.entity(entity).insert((
      Text2d::new(glyph.ch.to_string()),
      TextFont { font_size: TILE_SIZE, ..default() },
      TextColor(glyph.color)
    ));
    log_message(
      log,
      format!(
        "You empty the chest ({} stack{}).",
        kinds,
        if kinds == 1 { "" } else { "s" }
      )
    );
    clock.advance(1);
    note_player_turn_moved_world(clock, tb);
  }
}

fn salvage_label(item: Item) -> String {
  let y = item.scrap_yield();
  let preview = y
    .iter()
    .take(4)
    .map(|&(i, q)| format!("{}× {}", q, i.name()))
    .collect::<Vec<_>>()
    .join(", ");
  let tail = if y.len() > 4 { ", …" } else { "" };
  format!("Salvage {} → {}{}", item.name(), preview, tail)
}

fn format_recipe_ingredients(r: &crafting::Recipe) -> String {
  r.ingredients
    .iter()
    .map(|&(i, q)| format!("{}× {}", q, i.name()))
    .collect::<Vec<_>>()
    .join(", ")
}

fn build_salvage_options(inv: &HashMap<Item, u32>) -> Vec<InteractionOption> {
  let mut items =
    utils::mapv(|(&k, _)| k, utils::filter(|(k, n)| **n > 0 && k.can_salvage(), inv));
  items.sort_by(|a, b| a.name().cmp(b.name()));
  utils::mapv(
    |item| InteractionOption {
      label: salvage_label(item),
      action: InteractionAction::Salvage(item)
    },
    items
  )
}

fn build_craft_options(inv: &HashMap<Item, u32>) -> Vec<InteractionOption> {
  crafting::RECIPES
    .iter()
    .enumerate()
    .filter(|(_, r)| crafting::can_craft(inv, r))
    .map(|(i, r)| InteractionOption {
      label: format!("Craft {} ({})", r.output.name(), format_recipe_ingredients(r)),
      action: InteractionAction::Craft(i)
    })
    .collect()
}

fn handle_utility_menus(
  keys: Res<ButtonInput<KeyCode>>,
  player_q: Query<(&PlayerPos, &Inventory), With<Player>>,
  mut ui: ResMut<UiState>,
  mut log: ResMut<LogEntries>
) {
  if ui.any_open() {
    return;
  }
  if let Ok((_, inv)) = player_q.single() {
    if keys.just_pressed(KeyCode::KeyG) {
      let opts = build_salvage_options(&inv.0);
      if opts.is_empty() {
        log_message(
          &mut *log,
          "Nothing to salvage (gear, consumables, some junk).".into()
        );
      } else {
        ui.interact = InteractMenu::Open { options: opts };
      }
    } else if keys.just_pressed(KeyCode::KeyC) {
      let opts = build_craft_options(&inv.0);
      if opts.is_empty() {
        log_message(&mut *log, "No recipes available — gather base components.".into());
      } else {
        ui.interact = InteractMenu::Open { options: opts };
      }
    }
  }
}

fn execute_interaction(
  action: &InteractionAction,
  zone: &mut ActiveZone,
  clock: &mut Clock,
  tb: &mut TurnBasedWorldState,
  ui: &mut UiState,
  log: &mut LogEntries,
  commands: &mut Commands,
  player_query: &mut Query<(&mut PlayerPos, &mut Inventory), With<Player>>
) {
  // No player/position needed; must not sit behind `player_query` or logging can be skipped.
  if let InteractionAction::Talk { speaker, tree } = action {
    let node = tree.find(tree.nodes[0].name);
    ui.dialogue = DialogueState::Open { speaker, tree, node_name: tree.nodes[0].name };
    log_dialogue_node_block(log, speaker, node);
  } else if let InteractionAction::Navigate { .. } = action {
  } else if let Ok((pos, mut inventory)) = player_query.single_mut() {
    match action {
      InteractionAction::Talk { .. } => unreachable!(),
      InteractionAction::Navigate { .. } => {}
      InteractionAction::ToggleDoor(_) => {}
      InteractionAction::ChopTree(entity) => {
        commands.entity(*entity).despawn();
        *inventory.0.entry(Item::Wood).or_insert(0) += 1;
        clock.advance(2);
        note_player_turn_moved_world(clock, tb);
      }
      InteractionAction::PickUpItem(wx, wy) => {
        let level = zone.level_mut(pos.z);
        if (*wy as usize) < level.height && (*wx as usize) < level.width {
          if let Some(item) = level.items[*wy as usize][*wx as usize] {
            *inventory.0.entry(item).or_insert(0) += 1;
            level.set_item(*wx, *wy, None);
          }
        }
        clock.advance(1);
        note_player_turn_moved_world(clock, tb);
      }
      InteractionAction::OpenChest(_) => {}
      InteractionAction::Salvage(item) => {
        let Some(n) = inventory.0.get_mut(item) else {
          return;
        };
        if *n == 0 {
          return;
        }
        *n -= 1;
        if *n == 0 {
          inventory.0.remove(item);
        }
        for &(comp, q) in item.scrap_yield() {
          *inventory.0.entry(comp).or_insert(0) += q;
        }
        log_message(log, format!("Salvaged {} into scrap.", item.name()));
        clock.advance(1);
        note_player_turn_moved_world(clock, tb);
      }
      InteractionAction::Craft(recipe_idx) => {
        let Some(recipe) = crafting::RECIPES.get(*recipe_idx) else {
          return;
        };
        if !crafting::can_craft(&inventory.0, recipe) {
          return;
        }
        crafting::apply_craft(&mut inventory.0, recipe);
        log_message(log, format!("Crafted {}.", recipe.output.name()));
        clock.advance(2);
        note_player_turn_moved_world(clock, tb);
      }
    }
  }
}

fn dispatch_interactive_choice(
  option: InteractionOption,
  commands: &mut Commands,
  zone: &mut ActiveZone,
  clock: &mut Clock,
  tb: &mut TurnBasedWorldState,
  ui: &mut UiState,
  log: &mut LogEntries,
  player_query: &mut Query<(&mut PlayerPos, &mut Inventory), With<Player>>,
  pending_chest: &mut ChestOpenPending,
  pending_nav: &mut PendingNavigation,
  door_q: &mut Query<(
    &mut Door,
    &mut Glyph,
    Option<&mut Collidable>,
    &Location,
    Option<&mut AirlockDoor>
  )>,
  asset_server: &AssetServer,
  palette_cache: &mut PaletteImageCache,
  images: &mut Assets<Image>
) {
  match &option.action {
    InteractionAction::OpenChest(ent) => {
      pending_chest.0 = Some(*ent);
    }
    InteractionAction::Navigate { dest } => {
      pending_nav.0 = Some(*dest);
    }
    InteractionAction::ToggleDoor(entity) => {
      if let Ok((mut door, mut glyph, _collidable, location, mut airlock)) =
        door_q.get_mut(*entity)
      {
        let open = !door.open;
        set_door_open_state(
          commands,
          *entity,
          &mut door,
          &mut glyph,
          location,
          open,
          airlock.as_deref_mut(),
          clock.time,
          palette_cache,
          images,
          asset_server
        );
      }
      clock.advance(1);
      note_player_turn_moved_world(clock, tb);
    }
    other => {
      execute_interaction(other, zone, clock, tb, ui, log, commands, player_query);
    }
  }
}

fn auto_close_airlocks(
  mut commands: Commands,
  clock: Res<Clock>,
  asset_server: Res<AssetServer>,
  mut palette_cache: ResMut<PaletteImageCache>,
  mut images: ResMut<Assets<Image>>,
  mut airlock_q: Query<(Entity, &mut Door, &mut Glyph, &Location, &mut AirlockDoor)>
) {
  const AUTO_CLOSE_SIM_FRAMES: u64 = 100;
  for (entity, mut door, mut glyph, location, mut airlock) in airlock_q.iter_mut() {
    if door.open
      && airlock
        .opened_at_sim_time
        .is_some_and(|opened| clock.time.saturating_sub(opened) >= AUTO_CLOSE_SIM_FRAMES)
    {
      set_door_open_state(
        &mut commands,
        entity,
        &mut door,
        &mut glyph,
        location,
        false,
        Some(&mut airlock),
        clock.time,
        &mut palette_cache,
        &mut images,
        &asset_server
      );
    }
  }
}

// ---------------------------------------------------------------------------
// Player input
// ---------------------------------------------------------------------------

/// Check if pos would step off the zone edge in direction (dx, dy).
/// If so and the adjacent zone exists and target tile is walkable, perform the transition.
/// Returns true if a transition happened (or was blocked at world boundary) — caller skips normal move.

/// Separate system for Space key interactions to avoid Bevy's system param limit.
fn gather_interactions_at_tile(
  wx: i32,
  wy: i32,
  dir_label: &str,
  level: &level::Level,
  tile_entities: Option<&Vec<Entity>>,
  tree_q: &Query<Entity, With<Tree>>,
  dialogue_q: &Query<(&Named, &Dialogue)>,
  loot_chest_q: &mut Query<(&mut LootChest, &mut Glyph, &Location)>,
  door_q: &Query<&Door>,
  named_q: &Query<&Named>,
  flight_console_q: &Query<Entity, With<FlightConsole>>
) -> Vec<InteractionOption> {
  tile_entities
    .into_iter()
    .flat_map(|entities| entities.iter().copied())
    .flat_map(|e| {
      std::iter::empty::<InteractionOption>()
        .chain(
          tree_q
            .get(e)
            .ok()
            .map(|_| InteractionOption {
              label: format!("Chop tree ({dir_label})"),
              action: InteractionAction::ChopTree(e)
            })
            .into_iter()
        )
        .chain(
          dialogue_q
            .get(e)
            .ok()
            .map(|(named, dialogue)| InteractionOption {
              label: format!("Talk to {}", named.name),
              action: InteractionAction::Talk { speaker: named.name, tree: dialogue.0 }
            })
            .into_iter()
        )
        .chain(
          loot_chest_q
            .get_mut(e)
            .ok()
            .filter(|(c, _, _)| !c.opened)
            .map(|_| InteractionOption {
              label: format!("Open chest ({dir_label})"),
              action: InteractionAction::OpenChest(e)
            })
            .into_iter()
        )
        .chain(door_q.get(e).ok().into_iter().map(move |door| {
          let verb = if door.open { "Close" } else { "Open" };
          let name = named_q.get(e).map_or("door", |n| n.name);
          InteractionOption {
            label: format!("{verb} {name} ({dir_label})"),
            action: InteractionAction::ToggleDoor(e)
          }
        }))
        .chain(flight_console_q.get(e).ok().into_iter().flat_map(|_| {
          [
            InteractionOption {
              label: "Chart course — Origin planet".into(),
              action: InteractionAction::Navigate { dest: galaxy_gen::ID_STARTER_PLANET }
            },
            InteractionOption {
              label: "Chart course — Space asteroid field".into(),
              action: InteractionAction::Navigate { dest: galaxy_gen::ID_ASTEROID_FIELD }
            }
          ]
          .into_iter()
        }))
    })
    .chain(
      ((wy as usize) < level.height
        && (wx as usize) < level.width
        && level.items[wy as usize][wx as usize].is_some())
      .then_some(InteractionOption {
        label: format!("Pick up item ({dir_label})"),
        action: InteractionAction::PickUpItem(wx, wy)
      })
      .into_iter()
    )
    .collect()
}

fn resolve_bump_interact(
  mut pending: ResMut<PendingBumpInteract>,
  mut flash: ResMut<BumpInteractFlash>,
  mut ui: ResMut<UiState>,
  current: Res<CurrentZone>,
  index: Res<TileEntityIndex>,
  dialogue_q: Query<(&Named, &Dialogue)>,
  tree_q: Query<Entity, With<Tree>>,
  mut loot_chest_q: Query<(&mut LootChest, &mut Glyph, &Location)>,
  door_q: Query<&Door>,
  named_q: Query<&Named>,
  flight_console_q: Query<Entity, With<FlightConsole>>
) {
  let Some((tx, ty, tz)) = pending.0.take() else {
    return;
  };
  let level = current.0.level(tz);
  let opts = gather_interactions_at_tile(
    tx,
    ty,
    "ahead",
    level,
    index.0.get(&(tx, ty, tz)),
    &tree_q,
    &dialogue_q,
    &mut loot_chest_q,
    &door_q,
    &named_q,
    &flight_console_q
  );
  if opts.is_empty() {
    return;
  }
  if opts.len() == 1 {
    flash.0 = Some(opts.into_iter().next().unwrap());
  } else {
    ui.interact = InteractMenu::Open { options: opts };
  }
}

fn apply_bump_auto_interact(
  mut flash: ResMut<BumpInteractFlash>,
  mut commands: Commands,
  mut gw: ResMut<CurrentZone>,
  mut clock: ResMut<Clock>,
  mut tb: ResMut<TurnBasedWorldState>,
  mut ui: ResMut<UiState>,
  mut log: ResMut<LogEntries>,
  mut player_query: Query<(&mut PlayerPos, &mut Inventory), With<Player>>,
  mut pending_chest: ResMut<ChestOpenPending>,
  mut pending_nav: ResMut<PendingNavigation>,
  mut door_q: Query<(
    &mut Door,
    &mut Glyph,
    Option<&mut Collidable>,
    &Location,
    Option<&mut AirlockDoor>
  )>,
  asset_server: Res<AssetServer>,
  mut palette_cache: ResMut<PaletteImageCache>,
  mut images: ResMut<Assets<Image>>
) {
  let Some(option) = flash.0.take() else {
    return;
  };
  dispatch_interactive_choice(
    option,
    &mut commands,
    &mut gw.0,
    &mut clock,
    &mut tb,
    &mut ui,
    &mut log,
    &mut player_query,
    &mut pending_chest,
    &mut pending_nav,
    &mut door_q,
    &asset_server,
    &mut palette_cache,
    &mut images
  );
}

fn flush_pending_chest_open(
  mut pending_chest: ResMut<ChestOpenPending>,
  mut commands: Commands,
  mut player_q: Query<(&mut PlayerPos, &mut Inventory), With<Player>>,
  mut loot_chest_q: Query<(&mut LootChest, &mut Glyph, &Location)>,
  mut log: ResMut<LogEntries>,
  mut clock: ResMut<Clock>,
  mut tb: ResMut<TurnBasedWorldState>
) {
  if let Some(ent) = pending_chest.0.take() {
    apply_open_chest(
      &mut commands,
      ent,
      &mut player_q,
      &mut loot_chest_q,
      &mut *log,
      &mut *clock,
      &mut *tb
    );
  }
}

fn handle_interact(
  keys: Res<ButtonInput<KeyCode>>,
  current: Res<CurrentZone>,
  mut ui: ResMut<UiState>,
  index: Res<TileEntityIndex>,
  player_q: Query<&PlayerPos, With<Player>>,
  dialogue_q: Query<(&Named, &Dialogue)>,
  tree_q: Query<Entity, With<Tree>>,
  mut loot_chest_q: Query<(&mut LootChest, &mut Glyph, &Location)>,
  door_q: Query<&Door>,
  named_q: Query<&Named>,
  flight_console_q: Query<Entity, With<FlightConsole>>
) {
  if ui.any_open() || !keys.just_pressed(KeyCode::Space) {
    return;
  }

  if let Ok(pos) = player_q.single() {
    let level = current.0.level(pos.z);
    let mut options = Vec::new();
    for dy in -1i32..=1 {
      for dx in -1i32..=1 {
        let wx = pos.x + dx;
        let wy = pos.y + dy;
        let dir =
          if dx == 0 && dy == 0 { "here".to_string() } else { direction_name(dx, dy) };
        options.extend(gather_interactions_at_tile(
          wx,
          wy,
          &dir,
          level,
          index.0.get(&(wx, wy, pos.z)),
          &tree_q,
          &dialogue_q,
          &mut loot_chest_q,
          &door_q,
          &named_q,
          &flight_console_q
        ));
      }
    }

    if !options.is_empty() {
      ui.interact = InteractMenu::Open { options };
    }
  }
}

fn spawn_zone_geometry(
  commands: &mut Commands,
  asset_server: &AssetServer,
  palette_cache: &mut PaletteImageCache,
  images: &mut Assets<Image>,
  zone: &active_zone::ActiveZone,
  docked_at: Option<galaxy::LocationId>
) {
  spawn_level_tiles(commands, asset_server, palette_cache, images, zone);
  let (sox, soy) = zone.ship_origin;
  prefabs::Prefab::starting_ship().stamp_entities(commands, sox, soy, 0);
  if docked_at == Some(galaxy_gen::ID_STARTER_PLANET)
    && let Some((dox, doy)) = zone.dest_origin
  {
    prefabs::Prefab::starter_planet_surface().stamp_entities(commands, dox, doy, 0);
    for &(lx, ly) in galaxy_gen::STARTER_NPC_COORDS {
      let wx = dox + lx;
      let wy = doy + ly;
      let (obj, _dx, _dy) = match (lx, ly) {
        (22, 25) => (npcs::mira::mira(), 0, 0),
        (20, 23) => (npcs::chronos::chronos(), 0, 0),
        (26, 22) => (npcs::unit7::unit7(), 0, 0),
        (22, 21) => (npcs::kong::kong(), 0, 0),
        (24, 23) => (npcs::guard::guard(), 0, 0),
        _ => continue
      };
      obj.spawn_at(commands, wx, wy, 0);
    }
  }
}

fn apply_pending_navigation(
  mut pending: ResMut<PendingNavigation>,
  mut commands: Commands,
  galaxy: Res<galaxy::Galaxy>,
  mut ship: ResMut<ship::Ship>,
  mut current: ResMut<CurrentZone>,
  mut fov: ResMut<Fov>,
  asset_server: Res<AssetServer>,
  mut palette_cache: ResMut<PaletteImageCache>,
  mut images: ResMut<Assets<Image>>,
  mut log: ResMut<LogEntries>,
  tile_glyphs: Query<Entity, With<TileGlyph>>,
  item_glyphs: Query<Entity, With<ItemGlyph>>,
  glyph_vis: Query<Entity, (With<GlyphVisual>, Without<Player>)>,
  located: Query<Entity, (With<Location>, Without<Player>)>,
  mut player: Query<
    (&mut PlayerPos, &mut Location, &mut Visuals, &mut Transform),
    With<Player>
  >
) {
  let Some(dest) = pending.0.take() else {
    return;
  };
  if ship.docked_at == Some(dest) {
    log_message(
      &mut *log,
      "Astrogation: already holding position at that chart solution.".into()
    );
    return;
  }
  let Some(new_zone) = docking::dock(&galaxy, &mut ship, dest, 0) else {
    log_message(
      &mut *log,
      "Astrogation: cannot plot a dock for that destination.".into()
    );
    return;
  };
  let despawn_entities: HashSet<Entity> = tile_glyphs
    .iter()
    .chain(item_glyphs.iter())
    .chain(glyph_vis.iter())
    .chain(located.iter())
    .collect();
  for e in despawn_entities {
    commands.entity(e).despawn();
  }
  *current = CurrentZone(new_zone);
  fov.0 = FovGrid::new(current.0.width, current.0.height);
  spawn_zone_geometry(
    &mut commands,
    &asset_server,
    &mut palette_cache,
    &mut images,
    &current.0,
    ship.docked_at
  );
  let (sox, soy) = current.0.ship_origin;
  let start_x = ship::SHIP_WIDTH / 2;
  let start_y = ship::SHIP_HEIGHT / 2;
  let local_x = sox + start_x as i32;
  let local_y = soy + start_y as i32;
  let start_local = Vec2::new(local_x as f32, local_y as f32);
  if let Ok((mut pos, mut location, mut vis, mut tf)) = player.single_mut() {
    pos.x = local_x;
    pos.y = local_y;
    pos.z = 0;
    *location = Location::xyz(local_x, local_y, 0);
    vis.prev = start_local;
    vis.display = start_local;
    vis.last_pos = start_local;
    vis.last_move_start_frame = None;
    tf.translation =
      tile_screen_pos(local_x as f32, local_y as f32, current.0.width, current.0.height)
        + Vec3::Z;
  }
  let dest_name = match dest {
    galaxy_gen::ID_STARTER_PLANET => "origin planet",
    galaxy_gen::ID_ASTEROID_FIELD => "asteroid field",
    _ => "destination"
  };
  log_message(&mut *log, format!("Astrogation: docked — {dest_name} sector."));
}

fn setup(
  mut commands: Commands,
  asset_server: Res<AssetServer>,
  current: Res<CurrentZone>,
  ship: Res<ship::Ship>,
  mut images: ResMut<Assets<Image>>,
  mut palette_cache: ResMut<PaletteImageCache>,
  mut log: ResMut<LogEntries>
) {
  commands.spawn((Camera2d, Msaa::Off));

  spawn_zone_geometry(
    &mut commands,
    &asset_server,
    &mut palette_cache,
    &mut images,
    &current.0,
    ship.docked_at
  );

  let hover_img = white_pixel_image(&mut images);
  commands.spawn((
    TileHoverHighlight,
    Sprite {
      image: hover_img,
      custom_size: Some(Vec2::splat(TILE_SIZE)),
      color: Color::srgba(0.95, 0.92, 0.45, 0.28),
      ..default()
    },
    Transform::from_translation(Vec3::new(0.0, 0.0, 0.25)),
    Visibility::Hidden
  ));

  let start_x: i32 = ship::SHIP_WIDTH as i32 / 2;
  let start_y: i32 = ship::SHIP_HEIGHT as i32 / 2;
  let (sox, soy) = current.0.ship_origin;
  let local_x = sox + start_x;
  let local_y = soy + start_y;

  let start_local = Vec2::new(local_x as f32, local_y as f32);

  commands.spawn((
    Sprite {
      image: palette_sprite_handle(
        "textures/space_qud/mongus.png",
        Color::srgb(0.18, 0.42, 0.92),
        Color::srgb(0.98, 0.88, 0.22),
        &mut palette_cache,
        &mut images
      ),
      custom_size: Some(Vec2::splat(TILE_SIZE)),
      color: Color::WHITE,
      ..default()
    },
    Transform::from_translation(
      tile_screen_pos(local_x as f32, local_y as f32, current.0.width, current.0.height)
        + Vec3::Z
    ),
    Player,
    PlayerPos { x: local_x, y: local_y, z: 0 },
    Location::xyz(local_x, local_y, 0),
    Stats { hp: 20, max_hp: 20, attack: 5, move_speed: 3.0, attack_speed: 1.0 },
    Inventory::default(),
    GlyphVisual,
    Visuals {
      prev: start_local,
      last_move_start_frame: None,
      display: start_local,
      last_pos: start_local
    }
  ));

  log_message(
    &mut *log,
    format!(
      "{} — deck gravity nominal. You're on your ship (docked at the origin world).",
      ship::SHIP_NAME
    )
  );
}

fn update_fov(
  mut fov: ResMut<Fov>,
  current: Res<CurrentZone>,
  player_q: Query<&PlayerPos, With<Player>>,
  sight_q: Query<&Location, With<BlocksSight>>
) {
  if let Ok(&PlayerPos { x, y, z }) = player_q.single() {
    let blockers: HashSet<(i32, i32)> = sight_q
      .iter()
      .filter_map(|loc| {
        if let Location::Coords { x: lx, y: ly, z: lz, .. } = *loc
          && lz == z
        {
          Some((lx, ly))
        } else {
          None
        }
      })
      .collect();
    compute_fov(&mut fov.0, current.0.level(z), x, y, FOV_RADIUS, |tx, ty| {
      blockers.contains(&(tx, ty))
    });
  }
}

fn spawn_level_tiles(
  commands: &mut Commands,
  asset_server: &AssetServer,
  palette_cache: &mut PaletteImageCache,
  images: &mut Assets<Image>,
  zone: &active_zone::ActiveZone
) {
  for z in 0..zone.depth {
    let level = zone.level(z);
    for y in 0..level.height {
      for x in 0..level.width {
        let tile = level.tiles[y][x];
        if tile == Tile::Air {
          continue;
        }
        let pos = tile_screen_pos(x as f32, y as f32, zone.width, zone.height);

        if tile == Tile::Door || tile == Tile::AirlockDoor {
          continue;
        }

        if tile == Tile::Vacuum {
          if let Some((path, c1, c2)) = tile.properties().space_qud_sprite {
            let handle = palette_sprite_handle(
              path,
              Color::srgb(c1[0], c1[1], c1[2]),
              Color::srgb(c2[0], c2[1], c2[2]),
              palette_cache,
              images
            );
            commands.spawn((
              Sprite {
                image: handle,
                custom_size: Some(Vec2::splat(TILE_SIZE)),
                color: Color::srgba(0.0, 0.0, 0.0, 0.0),
                ..default()
              },
              Transform::from_translation(pos),
              TileGlyph { x, y },
              TilePng,
              Visibility::Visible
            ));
          }
          continue;
        }

        let tex_palette = tile.properties().space_qud_sprite.map(|(p, c1, c2)| {
          (p, Some((Color::srgb(c1[0], c1[1], c1[2]), Color::srgb(c2[0], c2[1], c2[2]))))
        });
        let tex_plain = tile.texture_path().map(|p| (p, None));
        let tex = tex_palette.or(tex_plain);
        if let Some((path, palette_opt)) = tex {
          let handle = if let Some((primary, secondary)) = palette_opt {
            palette_sprite_handle(path, primary, secondary, palette_cache, images)
          } else {
            asset_server.load(path)
          };
          commands.spawn((
            Sprite {
              image: handle,
              custom_size: Some(Vec2::splat(TILE_SIZE)),
              color: Color::srgba(0.0, 0.0, 0.0, 0.0),
              ..default()
            },
            Transform::from_translation(pos),
            TileGlyph { x, y },
            TilePng,
            Visibility::Visible
          ));
        } else {
          let [r, g, b] = tile.color();
          commands.spawn((
            Text2d::new(tile.glyph()),
            TextFont { font_size: TILE_SIZE, ..default() },
            TextColor(Color::srgba(r, g, b, 0.0)),
            Transform::from_translation(pos),
            TileGlyph { x, y },
            Visibility::Visible
          ));
        }

        if let Some(item) = level.items[y][x] {
          let [r, g, b] = item.color();
          commands.spawn((
            Text2d::new(item.glyph()),
            TextFont { font_size: TILE_SIZE, ..default() },
            TextColor(Color::srgba(r, g, b, 0.0)),
            Transform::from_translation(
              tile_screen_pos(x as f32, y as f32, zone.width, zone.height)
                + Vec3::new(0.0, 0.0, 1.0)
            ),
            ItemGlyph { x, y, z }
          ));
        }
      }
    }
  }
}

fn camera_follow(
  player_q: Query<&Visuals, With<Player>>,
  current: Res<CurrentZone>,
  mut cam_q: Query<&mut Transform, With<Camera2d>>,
  windows: Query<&Window>
) {
  if let Ok(vis) = player_q.single()
    && let Ok(mut cam_tf) = cam_q.single_mut()
    && let Ok(win) = windows.single()
  {
    let w = win.resolution.width();
    let h = win.resolution.height();
    let screen_center = Vec2::new(w / 2.0, h / 2.0);
    let viewport_center = Vec2::new(w * 0.35, (h - STATUS_BAR_HEIGHT) / 2.0);
    let offset = viewport_center - screen_center;

    let local = vis.display;
    let world_pos = Vec2::new(
      (local.x - current.0.width as f32 / 2.0) * TILE_SIZE,
      (current.0.height as f32 / 2.0 - local.y) * TILE_SIZE
    );
    let mut t = (world_pos - offset).extend(0.0);
    t.x = t.x.round();
    t.y = t.y.round();
    cam_tf.translation = t;
  }
}

fn player_input(
  keys: Res<ButtonInput<KeyCode>>,
  current: Res<CurrentZone>,
  ui: Res<UiState>,
  mut clock: ResMut<Clock>,
  mut tb: ResMut<TurnBasedWorldState>,
  mut time_mode_auto: ResMut<TimeModeAuto>,
  index: Res<TileEntityIndex>,
  mut pending_bump: ResMut<PendingBumpInteract>,
  mut player_query: Query<(&mut PlayerPos, &Stats, &mut Inventory), With<Player>>,
  mut enemy_query: Query<&mut Stats, (With<Enemy>, Without<Player>)>,
  collidable_q: Query<&Collidable>
) {
  if !ui.any_open() && keys.just_pressed(KeyCode::KeyT) {
    if keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight) {
      time_mode_auto.0 = true;
    } else {
      time_mode_auto.0 = false;
      clock.mode = match clock.mode {
        TimeMode::RealTime => TimeMode::TurnBased,
        TimeMode::TurnBased => {
          tb.pending_enemy_phase = false;
          TimeMode::RealTime
        }
      };
    }
  }

  if !ui.any_open()
    && let Ok((mut pos, stats, mut inventory)) = player_query.single_mut()
  {
    let player_attack = stats.attack;
    if clock.move_cooldown_frames > 0 {
      clock.move_cooldown_frames -= 1;
    }

    let turn_based_block = clock.mode == TimeMode::TurnBased
      && (clock.move_cooldown_frames > 0 || tb.pending_enemy_phase);

    if !turn_based_block
      && keys.just_pressed(KeyCode::Period)
      && !keys.pressed(KeyCode::ShiftLeft)
      && !keys.pressed(KeyCode::ShiftRight)
    {
      clock.advance(PlayerAction::Wait.time_cost());
      note_player_turn_moved_world(&*clock, &mut *tb);
    } else if !turn_based_block
      && (if clock.mode == TimeMode::TurnBased {
        any_direction_just_pressed(&keys)
      } else {
        any_direction_pressed(&keys)
      })
      && clock.move_cooldown_frames == 0
    {
      let level = current.0.level(pos.z);
      let dir = read_direction(&keys);
      let (raw_dx, raw_dy) = (dir.0, dir.1);

      let (dx, dy) = resolve_move(level, pos.x, pos.y, raw_dx, raw_dy);

      if (dx, dy) != (0, 0) {
        let target_x = pos.x + dx;
        let target_y = pos.y + dy;

        let enemy_hit = index.0.get(&(target_x, target_y, pos.z)).and_then(|entities| {
          entities.iter().find(|&&e| enemy_query.get(e).is_ok()).copied()
        });

        if let Some(hostile) = enemy_hit {
          if let Ok(mut es) = enemy_query.get_mut(hostile) {
            es.hp -= player_attack;
          }
        } else {
          let blocked = !level.walkable(target_x, target_y)
            || index.0.get(&(target_x, target_y, pos.z)).is_some_and(|entities| {
              entities.iter().any(|&e| collidable_q.get(e).is_ok_and(|c| c.0))
            });

          if !blocked {
            pos.x = target_x;
            pos.y = target_y;

            let lvl = current.0.level(pos.z);
            if (pos.y as usize) < lvl.height && (pos.x as usize) < lvl.width {
              if let Some(item) = lvl.items[pos.y as usize][pos.x as usize] {
                *inventory.0.entry(item).or_insert(0) += 1;
              }
            }
          } else {
            pending_bump.0 = Some((target_x, target_y, pos.z));
          }
        }

        clock.advance(PlayerAction::Move { dx, dy }.time_cost());
        clock.move_cooldown_frames = RENDER_FRAMES_PER_SIM_STEP;
        note_player_turn_moved_world(&*clock, &mut *tb);
      }
    }
  }
}
