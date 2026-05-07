mod ui;
use trl::level;
mod combat;
mod crafting;
mod loot;
mod npcs;
mod utils;

use {bevy::{anti_alias::fxaa::Fxaa, prelude::*},
  combat::{TileEntityIndex, enemy_ai, maintain_tile_index},
  level::{FovGrid, Item, Tile, ZONE_HEIGHT, ZONE_WIDTH, compute_fov},
  std::collections::{HashMap, HashSet},
     trl::entities::{BlocksSight, Collidable, Dialogue, DialogueNode, DialogueTree, Door,
                     Enemy, Glyph, Location, LootChest, Named, Object, Stats, Tree,
                     Visuals},
     ui::{LogEntries, WorldMapView, log_message}};

use trl::{active_zone::{self, ActiveZone},
          galaxy, galaxy_gen, ship,
          sprites::{palette_sprite_handle, PaletteImageCache}};

const TILE_SIZE: f32 = 64.0;
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
  fn new() -> Self { Clock { time: 0, mode: TimeMode::RealTime, move_cooldown_frames: 0 } }

  fn advance(&mut self, cost: u32) { self.time = self.time.saturating_add(u64::from(cost)); }
}

/// In turn-based mode, the world only advances in [`SimStep`] after a player spends a turn and
/// move animation finishes (`move_cooldown_frames == 0`); this flag schedules that one tick.
/// Cleared at the end of [`combat::enemy_ai`] after that tick runs.
#[derive(Resource, Default)]
pub struct TurnBasedWorldState {
  pub pending_enemy_phase: bool
}

/// Set when the player picks "Open chest" from the interact menu; applied next frame.
#[derive(Resource, Default)]
struct ChestOpenPending(pub Option<Entity>);

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
  y: usize
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
    Query<(&Location, &mut Visuals)>,
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
    Query<(&Location, &mut Visuals)>,
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

fn sync_entity_positions(mut query: Query<(&Visuals, &mut Transform), With<GlyphVisual>>) {
  for (vis, mut transform) in query.iter_mut() {
    transform.translation =
      tile_screen_pos(vis.display.x, vis.display.y, ZONE_WIDTH, ZONE_HEIGHT)
        + Vec3::new(0.0, 0.0, 2.0);
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
    let origin: galaxy::LocationId = (0, 0, 0);
    let starter_planet = galaxy_gen::generate_starter_planet();
    galaxy.insert(origin, starter_planet.clone());

    // Ship starts docked at the starter planet
    let active = active_zone::ActiveZone::docked(
        &ship_location,
        &starter_planet,
    0 // first landing spot
  )
  .expect("ship should dock at starter planet");

  let ship_res =
    ship::Ship { location_id: ship_id, docked_at: Some(origin), fuel: 500, max_fuel: 500 };

    let fov = level::FovGrid::new(active.width, active.height);

    let _ = &active; // Keep 'active' in scope for init

    App::new()
        .add_plugins(haalka::HaalkaPlugin::default())
    .add_plugins(DefaultPlugins.set(ImagePlugin::default_linear()).set(WindowPlugin {
                primary_window: Some(Window {
                    title: "trl — space".into(),
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
        .init_resource::<PaletteImageCache>()
        .insert_resource(UiState::default())
        .insert_resource(Fov(fov))
        .insert_resource(TileEntityIndex::default())
        .add_plugins(ui::UiPlugin)
        .add_systems(Startup, (setup, ui::spawn_haalka_root).chain())
        .configure_sets(Update, SimStep.run_if(should_run_sim_step))
    .add_systems(
      Update,
      (
            bump_render_frame,
            maintain_tile_index,
            setup_glyph_visuals,
            update_time_mode,
            handle_world_map,
            handle_dialogue,
            handle_menus,
            handle_interact,
            handle_utility_menus,
        player_input
      )
        .chain()
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
fn enemy_death_check(mut commands: Commands, enemy_q: Query<(Entity, &Stats), With<Enemy>>) {
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
  world_map: Res<WorldMapView>,
  mut q: Query<(&mut Transform, &mut Visibility), With<TileHoverHighlight>>
) {
  if let Ok((mut transform, mut vis)) = q.single_mut() {
    *vis = Visibility::Hidden;
    if !world_map.open
      && let Ok(window) = windows.single()
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
          .filter(|&(tx, ty)| tx >= 0 && ty >= 0 && (tx as usize) < lw && (ty as usize) < lh)
      };
      if let Some((tx, ty)) = pick(window, camera, cam_transform, level.width, level.height)
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
  player_q: Query<&PlayerPos, With<Player>>,
  mut glyph_tiles: Query<(&TileGlyph, &mut TextColor), Without<TilePng>>,
  mut sprite_tiles: Query<(&TileGlyph, &mut Sprite), With<TilePng>>,
  mut entity_q: Query<(&Location, &mut Visibility), With<GlyphVisual>>
) {
  if let Ok(pos) = player_q.single() {
    let level = current.0.level(pos.z);
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
    for (location, mut vis) in entity_q.iter_mut() {
      *vis = if let Location::Coords { x, y, z, .. } = location
        && *z == pos.z
        && fov.0.is_visible(*x as usize, *y as usize)
      {
        Visibility::Visible
      } else {
        Visibility::Hidden
      };
    }
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

fn handle_menus(
  keys: Res<ButtonInput<KeyCode>>,
  mut ui: ResMut<UiState>,
  mut world_map: ResMut<WorldMapView>,
  mut commands: Commands,
  mut gw: ResMut<CurrentZone>,
  mut clock: ResMut<Clock>,
  mut tb: ResMut<TurnBasedWorldState>,
  mut log: ResMut<LogEntries>,
  mut player_query: Query<(&mut PlayerPos, &mut Inventory), With<Player>>,
  mut door_q: Query<(&mut Door, &mut Glyph, &mut Collidable)>,
  mut pending_chest: ResMut<ChestOpenPending>,
  mut exit: MessageWriter<AppExit>
) {
  if keys.just_pressed(KeyCode::Escape) && world_map.open {
    world_map.open = false;
    return;
  }

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
      if let InteractionAction::OpenChest(ent) = option.action {
        pending_chest.0 = Some(ent);
      } else if let InteractionAction::ToggleDoor(entity) = option.action {
        if let Ok((mut door, mut glyph, mut collidable)) = door_q.get_mut(entity) {
          door.open = !door.open;
          collidable.0 = !door.open;
          if door.open {
            commands.entity(entity).remove::<(Collidable, BlocksSight, Sprite, Text2d)>();
            glyph.ch = '/';
            glyph.color = Color::srgb(0.3, 0.5, 0.3);
            commands.entity(entity).insert((
              Text2d::new("/"),
              TextFont { font_size: TILE_SIZE, ..default() },
              TextColor(Color::srgb(0.3, 0.5, 0.3))
            ));
          } else {
            commands.entity(entity).insert((Collidable(true), BlocksSight));
            glyph.ch = '+';
            glyph.color = door.closed_color;
            commands.entity(entity).remove::<(Sprite, Text2d)>();
            commands.entity(entity).insert((
              Text2d::new("+"),
              TextFont { font_size: TILE_SIZE, ..default() },
              TextColor(door.closed_color)
            ));
          }
        }
        clock.advance(1);
        note_player_turn_moved_world(&*clock, &mut *tb);
      } else {
        execute_interaction(
          &option.action,
          &mut gw.0,
          &mut clock,
          &mut *tb,
          &mut ui,
          &mut *log,
          &mut commands,
          &mut player_query
        );
      }
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
  world_map: Res<WorldMapView>,
  mut ui: ResMut<UiState>,
  mut log: ResMut<LogEntries>
) {
  // Digit keys are shared with the interact list; `handle_menus` runs after us. While the
  // interact overlay is up, that same key would otherwise apply to dialogue the same frame.
  if world_map.open || matches!(&ui.interact, InteractMenu::Open { .. }) {
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
    TextColor(glyph.color),
  ));
  log_message(
    log,
    format!(
      "You empty the chest ({} stack{}).",
      kinds,
      if kinds == 1 { "" } else { "s" }
    ),
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
  world_map: Res<WorldMapView>,
  player_q: Query<(&PlayerPos, &Inventory), With<Player>>,
  mut ui: ResMut<UiState>,
  mut log: ResMut<LogEntries>
) {
  if ui.any_open() || world_map.open {
    return;
  }
  if let Ok((_, inv)) = player_q.single() {
    if keys.just_pressed(KeyCode::KeyG) {
      let opts = build_salvage_options(&inv.0);
      if opts.is_empty() {
        log_message(&mut *log, "Nothing to salvage (gear, consumables, some junk).".into());
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
    return;
  }

  if let Ok((pos, mut inventory)) = player_query.single_mut() {
    match action {
      InteractionAction::ToggleDoor(_) => {}
      InteractionAction::Talk { .. } => unreachable!(),
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

// ---------------------------------------------------------------------------
// Player input
// ---------------------------------------------------------------------------

/// Check if pos would step off the zone edge in direction (dx, dy).
/// If so and the adjacent zone exists and target tile is walkable, perform the transition.
/// Returns true if a transition happened (or was blocked at world boundary) — caller skips normal move.

/// Separate system for Space key interactions to avoid Bevy's system param limit.
fn handle_interact(
  keys: Res<ButtonInput<KeyCode>>,
  current: Res<CurrentZone>,
  mut ui: ResMut<UiState>,
  world_map: Res<WorldMapView>,
  index: Res<TileEntityIndex>,
  mut commands: Commands,
  mut player_q: Query<(&mut PlayerPos, &mut Inventory), With<Player>>,
  dialogue_q: Query<(&Named, &Dialogue)>,
  tree_q: Query<Entity, With<Tree>>,
  mut pending_chest: ResMut<ChestOpenPending>,
  mut loot_chest_q: Query<(&mut LootChest, &mut Glyph, &Location)>,
  door_q: Query<&Door>,
  named_q: Query<&Named>,
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

  if ui.any_open() || world_map.open || !keys.just_pressed(KeyCode::Space) {
    return;
  }

  if let Ok((pos, _)) = player_q.single() {
    let level = current.0.level(pos.z);
    let options: Vec<_> = (-1i32..=1)
      .flat_map(|dy| (-1i32..=1).map(move |dx| (dx, dy)))
      .flat_map(|(dx, dy)| {
      let wx = pos.x + dx;
      let wy = pos.y + dy;
        let dir =
          if dx == 0 && dy == 0 { "here".to_string() } else { direction_name(dx, dy) };
        let mut tile_opts = Vec::new();
      if let Some(entities) = index.0.get(&(wx, wy, pos.z)) {
        for &e in entities.iter() {
          if tree_q.get(e).is_ok() {
              tile_opts.push(InteractionOption {
              label: format!("Chop tree ({dir})"),
                action: InteractionAction::ChopTree(e)
            });
          }
          if let Ok((named, dialogue)) = dialogue_q.get(e) {
              tile_opts.push(InteractionOption {
              label: format!("Talk to {}", named.name),
                action: InteractionAction::Talk { speaker: named.name, tree: dialogue.0 }
            });
          }
          if let Ok((chest, _, _)) = loot_chest_q.get(e)
            && !chest.opened
          {
              tile_opts.push(InteractionOption {
              label: format!("Open chest ({dir})"),
                action: InteractionAction::OpenChest(e)
            });
          }
          if let Ok(door) = door_q.get(e) {
            let verb = if door.open { "Close" } else { "Open" };
            let name = named_q.get(e).map_or("door", |n| n.name);
              tile_opts.push(InteractionOption {
              label: format!("{verb} {name} ({dir})"),
                action: InteractionAction::ToggleDoor(e)
            });
          }
        }
      }
        if (wy as usize) < level.height
          && (wx as usize) < level.width
        && level.items[wy as usize][wx as usize].is_some()
      {
          tile_opts.push(InteractionOption {
          label: format!("Pick up item ({dir})"),
            action: InteractionAction::PickUpItem(wx, wy)
        });
      }
        tile_opts
      })
      .collect();

    if !options.is_empty() {
      ui.interact = InteractMenu::Open { options };
    }
  }
}

fn handle_world_map(
  keys: Res<ButtonInput<KeyCode>>,
  mut world_map: ResMut<WorldMapView>,
  ui: Res<UiState>
) {
  if !keys.just_pressed(KeyCode::KeyM) {
    return;
  }
  if world_map.open {
    world_map.open = false;
  } else if !ui.any_open() {
    world_map.open = true;
  }
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    current: Res<CurrentZone>,
    mut images: ResMut<Assets<Image>>,
    mut palette_cache: ResMut<PaletteImageCache>,
  mut world_map: ResMut<WorldMapView>
) {
    commands.spawn((Camera2d, Fxaa::default(), Msaa::Off));

    spawn_level_tiles(
      &mut commands,
      &asset_server,
      &mut palette_cache,
      &mut images,
      &current.0,
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
        &mut images,
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

    // Spawn flight console on the ship
    let console_x = sox + ship::CONSOLE_X;
    let console_y = soy + ship::CONSOLE_Y;
    Object::flight_console().spawn_at(&mut commands, console_x, console_y, 0);

    // Spawn starter planet NPCs at destination-local coords mapped into the active zone
    if let Some((dox, doy)) = current.0.dest_origin {
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
            obj.spawn_at(&mut commands, wx, wy, 0);
        }

        // Spawn trees as entities at destination coords
        for &(lx, ly) in &[(5, 5), (8, 12), (40, 8), (38, 30)] {
            let wx = dox + lx;
            let wy = doy + ly;
            Object::tree().spawn_at(&mut commands, wx, wy, 0);
        }
    }

    world_map.image = generate_world_map_image(&current.0, &mut images);
}

fn update_fov(
    mut fov: ResMut<Fov>,
    current: Res<CurrentZone>,
    player_q: Query<&PlayerPos, With<Player>>,
  sight_q: Query<&Location, With<BlocksSight>>
) {
    let Ok(pos) = player_q.single() else { return };
    let level = current.0.level(pos.z);
    // Space mode uses contiguous local coords: no zone wrapping.
    let blockers: HashSet<(i32, i32)> = sight_q
        .iter()
        .filter_map(|loc| {
            if let Location::Coords { x, y, z, .. } = *loc
                && z == pos.z
            {
                Some((x, y))
            } else {
                None
            }
        })
        .collect();
    compute_fov(&mut fov.0, level, pos.x, pos.y, FOV_RADIUS, |tx, ty| {
        blockers.contains(&(tx, ty))
    });
}

/// Space Qud mask sprites under `assets/textures/space_qud/`: black→primary, white→secondary.
fn space_qud_tile_sprite(tile: Tile) -> Option<(&'static str, Color, Color)> {
  match tile {
    Tile::DeckPlate => Some((
      "textures/space_qud/grid.png",
      Color::srgb(0.38, 0.42, 0.48),
      Color::srgb(0.72, 0.76, 0.82),
    )),
    Tile::Bulkhead => Some((
      "textures/space_qud/door closed.png",
      Color::srgb(0.32, 0.35, 0.4),
      Color::srgb(0.52, 0.55, 0.58),
    )),
    Tile::Window => Some((
      "textures/space_qud/window.png",
      Color::srgb(0.22, 0.32, 0.52),
      Color::srgb(0.62, 0.76, 0.94),
    )),
    _ => None,
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
                if tile == Tile::Air || tile == Tile::Vacuum {
                    continue;
                }
                let pos = tile_screen_pos(x as f32, y as f32, zone.width, zone.height);
                if tile == Tile::Door || tile == Tile::AirlockDoor {
                    let [r, g, b] = tile.color();
                    commands.spawn((
                        Door { open: false, closed_color: Color::srgb(r, g, b) },
                        Collidable(true),
                        BlocksSight,
                        Glyph::ascii('+', Color::srgb(r, g, b)),
                        Location::xyz(x as i32, y as i32, z),
            Named { name: tile.name(), flavor: "Press Space to open." }
                    ));
                } else {
                let tex_palette = space_qud_tile_sprite(tile).map(|(p, c1, c2)| (p, Some((c1, c2))));
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
            TilePng
                    ));
                } else {
                    let [r, g, b] = tile.color();
                    commands.spawn((
                        Text2d::new(tile.glyph()),
                        TextFont { font_size: TILE_SIZE, ..default() },
                        TextColor(Color::srgba(r, g, b, 0.0)),
                        Transform::from_translation(pos),
            TileGlyph { x, y }
                    ));
                }
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
            ItemGlyph { x, y }
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
        cam_tf.translation = (world_pos - offset).extend(0.0);
    }
}

fn generate_world_map_image(
    zone: &active_zone::ActiveZone,
  images: &mut Assets<Image>
) -> Handle<Image> {
  use bevy::{asset::RenderAssetUsages,
             render::render_resource::{Extent3d, TextureDimension, TextureFormat}};

    let w = zone.width;
    let h = zone.height;
    let mut data = vec![0u8; w * h * 4];

    let level = zone.level(0);
    for ty in 0..h {
        for tx in 0..w {
            let [r, g, b] = level.tiles[ty][tx].minimap_color();
            let idx = (ty * w + tx) * 4;
            data[idx]     = (r * 255.0) as u8;
            data[idx + 1] = (g * 255.0) as u8;
            data[idx + 2] = (b * 255.0) as u8;
            data[idx + 3] = 255;
        }
    }

    images.add(Image::new(
    Extent3d { width: w as u32, height: h as u32, depth_or_array_layers: 1 },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
    RenderAssetUsages::RENDER_WORLD
    ))
}

fn player_input(
    keys: Res<ButtonInput<KeyCode>>,
    current: Res<CurrentZone>,
    ui: Res<UiState>,
    world_map: Res<WorldMapView>,
    mut clock: ResMut<Clock>,
    mut tb: ResMut<TurnBasedWorldState>,
    mut time_mode_auto: ResMut<TimeModeAuto>,
    index: Res<TileEntityIndex>,
    mut player_query: Query<(&mut PlayerPos, &Stats, &mut Inventory), With<Player>>,
    mut enemy_query: Query<&mut Stats, (With<Enemy>, Without<Player>)>,
  collidable_q: Query<&Collidable>
) {
    if !ui.any_open() && !world_map.open && keys.just_pressed(KeyCode::KeyT) {
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
    && !world_map.open
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
                    }
                }

                clock.advance(PlayerAction::Move { dx, dy }.time_cost());
                clock.move_cooldown_frames = RENDER_FRAMES_PER_SIM_STEP;
                note_player_turn_moved_world(&*clock, &mut *tb);
            }
        }
    }
}
