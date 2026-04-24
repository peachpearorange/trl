mod ui;
mod level;
mod combat;
mod dialogue;
mod worldgen;
mod world_data;

use {
  bevy::prelude::*,
  combat::{TileEntityIndex, enemy_ai, maintain_tile_index},
  level::{FovGrid, Tile, ZoneWorld, ZONE_WIDTH, ZONE_HEIGHT, WORLD_DEPTH, build_test_world, compute_fov},
  trl::entities::{Collidable, Dialogue, DialogueTree, Enemy, Glyph, Gravity, Location, Named, Object, Stats, Tree},
};

const TILE_SIZE: f32 = 32.0;
const MOVE_COOLDOWN: f32 = 0.12;
const FOV_RADIUS: i32 = 12;
const DIM_FACTOR: f32 = 0.3;

// ---------------------------------------------------------------------------
// Player actions
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug)]
enum PlayerAction {
  Move { dx: i32, dy: i32 },
  Ascend,
  Descend,
  Wait
}

impl PlayerAction {
  fn time_cost(self) -> f32 {
    match self {
      PlayerAction::Move { dx, dy } if dx != 0 && dy != 0 => 2.0,
      PlayerAction::Move { .. } => 1.0,
      PlayerAction::Ascend | PlayerAction::Descend => 3.0,
      PlayerAction::Wait => 1.0
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
  Ascend,
  Descend,
  OpenDoor(i32, i32),
  Talk { speaker: &'static str, tree: &'static DialogueTree },
  ChopTree(Entity),
  PickUpItem(i32, i32),
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
  Open { speaker: &'static str, tree: &'static DialogueTree, node_name: &'static str },
}

#[derive(Component)]
struct DialogueOverlay;

// ---------------------------------------------------------------------------
// Merged UI state
// ---------------------------------------------------------------------------

#[derive(Resource, Default)]
struct UiState {
  pause: PauseMenu,
  interact: InteractMenu,
  dialogue: DialogueState,
}

impl UiState {
  fn any_open(&self) -> bool {
    self.pause != PauseMenu::Closed
      || matches!(self.interact, InteractMenu::Open { .. })
      || matches!(self.dialogue, DialogueState::Open { .. })
  }
}

// ---------------------------------------------------------------------------
// Merged timing
// ---------------------------------------------------------------------------

#[derive(Resource)]
pub struct Clock {
  time: f32,
  mode: TimeMode,
  move_cooldown: f32,
}

impl Clock {
  fn new() -> Self { Clock { time: 0.0, mode: TimeMode::RealTime, move_cooldown: 0.0 } }

  fn advance(&mut self, cost: f32) { self.time += cost; }

  fn tick_realtime(&mut self, dt: f32) {
    if self.mode == TimeMode::RealTime {
      self.time += dt;
    }
  }
}

// ---------------------------------------------------------------------------
// Resources & components
// ---------------------------------------------------------------------------

#[derive(Resource)]
pub struct GameWorld(pub ZoneWorld);

#[derive(Resource)]
pub struct Fov(pub FovGrid);

#[derive(Component)]
pub struct Player;

#[derive(Component)]
pub struct PlayerPos {
  pub x: i32,
  pub y: i32,
  pub z: usize,
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

#[derive(Component)]
struct InteractOverlay;

#[derive(Component)]
struct PauseOverlay;

#[derive(Component, Default)]
pub struct Inventory(std::collections::HashMap<level::Item, u32>);

/// Marker for entities that have had their Text2d visual set up.
#[derive(Component)]
struct GlyphVisual;

// ---------------------------------------------------------------------------
// HUD component markers
// ---------------------------------------------------------------------------

#[derive(Component)]
struct HudElement;

#[derive(Component)]
struct TimeDisplay;

#[derive(Component)]
struct LevelDisplay;

#[derive(Component)]
struct TileInfoDisplay;

#[derive(Component)]
struct InventoryDisplay;

// ---------------------------------------------------------------------------
// Glyph rendering systems
// ---------------------------------------------------------------------------

fn setup_glyph_visuals(
  mut commands: Commands,
  query: Query<(Entity, &Glyph, &Location), (Added<Glyph>, Without<GlyphVisual>)>,
) {
  for (entity, glyph, location) in query.iter() {
    if let Location::Coords { x, y, .. } = location {
      let (lx, ly) = world_to_local(*x, *y);
      let pos = tile_screen_pos(lx, ly, ZONE_WIDTH, ZONE_HEIGHT)
        + Vec3::new(0.0, 0.0, 2.0);
      commands.entity(entity).insert((
        Text2d::new(glyph.ch.to_string()),
        TextFont { font_size: TILE_SIZE, ..default() },
        TextColor(glyph.color),
        Transform::from_translation(pos),
        GlyphVisual,
      ));
    }
  }
}

fn sync_entity_positions(
  mut query: Query<(&Location, &mut Transform), (With<GlyphVisual>, Changed<Location>)>,
) {
  for (location, mut transform) in query.iter_mut() {
    if let Location::Coords { x, y, .. } = location {
      let (lx, ly) = world_to_local(*x, *y);
      transform.translation =
        tile_screen_pos(lx, ly, ZONE_WIDTH, ZONE_HEIGHT)
          + Vec3::new(0.0, 0.0, 2.0);
    }
  }
}

// ---------------------------------------------------------------------------
// App
// ---------------------------------------------------------------------------

fn main() {
  let world = build_test_world();
  let fov = FovGrid::new(ZONE_WIDTH, ZONE_HEIGHT);

  App::new()
    .add_plugins(DefaultPlugins.set(ImagePlugin::default_nearest()).set(WindowPlugin {
      primary_window: Some(Window {
        title: "trl".into(),
        resolution: (1200u32, 800u32).into(),
        ..default()
      }),
      ..default()
    }))
    .insert_resource(ClearColor(Color::srgb(0.02, 0.02, 0.05)))
    .insert_resource(GameWorld(world))
    .insert_resource(Clock::new())
    .insert_resource(UiState::default())
    .insert_resource(Fov(fov))
    .insert_resource(TileEntityIndex::default())
    .add_plugins(ui::UiPlugin)
    .add_systems(Startup, setup)
    .add_systems(
      Update,
      (
        maintain_tile_index,
        setup_glyph_visuals,
        sync_entity_positions,
        update_entity_visibility,
        advance_realtime,
        update_time_mode,
        handle_menus,
        handle_dialogue,
        handle_interact,
        player_input,
        ApplyDeferred,
        apply_gravity,
        enemy_ai,
        camera_follow,
        update_fov_visuals,
      )
        .chain()
    )
    .run();
}

// ---------------------------------------------------------------------------
// Coordinate helpers
// ---------------------------------------------------------------------------

fn tile_screen_pos(x: usize, y: usize, w: usize, h: usize) -> Vec3 {
  Vec3::new(
    (x as f32 - w as f32 / 2.0) * TILE_SIZE,
    (h as f32 / 2.0 - y as f32) * TILE_SIZE,
    0.0
  )
}

pub fn screen_to_tile(world_pos: Vec2, w: usize, h: usize) -> (i32, i32) {
  let tx = (world_pos.x / TILE_SIZE + w as f32 / 2.0).floor() as i32;
  let ty = (h as f32 / 2.0 - world_pos.y / TILE_SIZE).floor() as i32;
  (tx, ty)
}

fn world_to_local(wx: i32, wy: i32) -> (usize, usize) {
  (wx as usize % ZONE_WIDTH, wy as usize % ZONE_HEIGHT)
}

pub fn world_to_zone(wx: i32, wy: i32) -> (usize, usize) {
  (wx as usize / ZONE_WIDTH, wy as usize / ZONE_HEIGHT)
}

// ---------------------------------------------------------------------------
// Setup
// ---------------------------------------------------------------------------

fn setup(
  mut commands: Commands,
  asset_server: Res<AssetServer>,
  gw: Res<GameWorld>,
  mut fov: ResMut<Fov>
) {
  const START_ZX: usize = 0;
  const START_ZY: usize = 0;
  const START_Z:  usize = 2;

  let cam_entity = commands.spawn(Camera2d).id();

  spawn_level_tiles(&mut commands, &asset_server, &gw.0, START_ZX, START_ZY, START_Z);

  let level = gw.0.zone(START_ZX, START_ZY, START_Z);
  let (lx, ly) = find_walkable(level, 15, 15);
  let (px, py) = (
    (START_ZX * ZONE_WIDTH) as i32 + lx as i32,
    (START_ZY * ZONE_HEIGHT) as i32 + ly as i32,
  );
  compute_fov(&mut fov.0, level, lx as i32, ly as i32, FOV_RADIUS);

  commands.spawn((
    Text2d::new("@"),
    TextFont { font_size: TILE_SIZE, ..default() },
    TextColor(Color::srgb(1.0, 1.0, 0.0)),
    Transform::from_translation(
      tile_screen_pos(lx, ly, ZONE_WIDTH, ZONE_HEIGHT) + Vec3::Z
    ),
    Player,
    PlayerPos { x: px, y: py, z: START_Z },
    Stats { hp: 20, max_hp: 20, attack: 5, move_speed: 3.0, attack_speed: 1.0 },
    Inventory::default(),
  ));

  // Spawn enemies and NPCs — compute walkable in local coords, convert to world
  let (lex1, ley1) = find_walkable(level, lx + 5, ly);
  let (ex1, ey1) = (
    (START_ZX * ZONE_WIDTH) as i32 + lex1 as i32,
    (START_ZY * ZONE_HEIGHT) as i32 + ley1 as i32,
  );
  let (lex2, ley2) = find_walkable(level, lx + 3, ly + 4);
  let (ex2, ey2) = (
    (START_ZX * ZONE_WIDTH) as i32 + lex2 as i32,
    (START_ZY * ZONE_HEIGHT) as i32 + ley2 as i32,
  );
  let (lcx1, lcy1) = find_walkable(level, lx.saturating_sub(4), ly + 2);
  let (cx1, cy1) = (
    (START_ZX * ZONE_WIDTH) as i32 + lcx1 as i32,
    (START_ZY * ZONE_HEIGHT) as i32 + lcy1 as i32,
  );

  Object::rat_soldier().spawn_at(&mut commands, ex1, ey1, START_Z);
  Object::armored_rat_soldier().spawn_at(&mut commands, ex2, ey2, START_Z);
  Object::catgirl()
    .add(Dialogue(&dialogue::MIRA))
    .spawn_at(&mut commands, cx1, cy1, START_Z);

  // Trees — place a few on grass tiles around the starting area
  for &(tx, ty) in &[(5, 5), (40, 5), (5, 40), (35, 35), (20, 3), (3, 20), (42, 30)] {
    let (ltx, lty) = find_walkable(level, tx, ty);
    let (wx, wy) = (
      (START_ZX * ZONE_WIDTH) as i32 + ltx as i32,
      (START_ZY * ZONE_HEIGHT) as i32 + lty as i32,
    );
    Object::tree().spawn_at(&mut commands, wx, wy, START_Z);
  }

  // HUD — children of camera so they stay fixed on screen
  let time_id = commands
    .spawn((
      Text2d::new("RT T:0"),
      TextFont { font_size: 14.0, ..default() },
      TextColor(Color::srgb(0.5, 0.7, 0.5)),
      Transform::from_xyz(-580.0, 380.0, 5.0),
      HudElement,
      TimeDisplay
    ))
    .id();

  let level_id = commands
    .spawn((
      Text2d::new("Surface (z=2)"),
      TextFont { font_size: 14.0, ..default() },
      TextColor(Color::srgb(0.6, 0.6, 0.6)),
      Transform::from_xyz(-580.0, 360.0, 5.0),
      HudElement,
      LevelDisplay
    ))
    .id();

  let tile_info_id = commands
    .spawn((
      Text2d::new(""),
      TextFont { font_size: 13.0, ..default() },
      TextColor(Color::srgb(0.7, 0.7, 0.6)),
      Transform::from_xyz(460.0, 380.0, 5.0),
      HudElement,
      TileInfoDisplay
    ))
    .id();

  let inv_id = commands
    .spawn((
      Text2d::new(""),
      TextFont { font_size: 13.0, ..default() },
      TextColor(Color::srgb(0.7, 0.65, 0.45)),
      Transform::from_xyz(-580.0, 340.0, 5.0),
      HudElement,
      InventoryDisplay
    ))
    .id();

  commands.entity(cam_entity).add_children(&[time_id, level_id, tile_info_id, inv_id]);
}

fn find_walkable(level: &level::Level, hint_x: usize, hint_y: usize) -> (usize, usize) {
  let (hx, hy) = (hint_x as i32, hint_y as i32);
  std::iter::once((0i32, 0i32))
    .chain((1..30i32).flat_map(|r| (-r..=r).flat_map(move |dy| (-r..=r).map(move |dx| (dx, dy)))))
    .map(|(dx, dy)| (hx + dx, hy + dy))
    .filter(|&(x, y)| x >= 0 && y >= 0)
    .find(|&(x, y)| level.walkable(x, y))
    .map(|(x, y)| (x as usize, y as usize))
    .unwrap_or((hint_x, hint_y))
}

// ---------------------------------------------------------------------------
// Level rendering
// ---------------------------------------------------------------------------

fn spawn_level_tiles(
  commands: &mut Commands,
  asset_server: &AssetServer,
  world: &ZoneWorld,
  zx: usize,
  zy: usize,
  z: usize,
) {
  let level = world.zone(zx, zy, z);
  for y in 0..level.height {
    for x in 0..level.width {
      let tile = level.tiles[y][x];
      if tile == Tile::Air {
        continue;
      }
      let pos = tile_screen_pos(x, y, ZONE_WIDTH, ZONE_HEIGHT);
      if let Some(path) = tile.texture_path() {
        commands.spawn((
          Sprite {
            image: asset_server.load(path),
            custom_size: Some(Vec2::splat(TILE_SIZE)),
            color: Color::srgba(0.0, 0.0, 0.0, 0.0),
            ..default()
          },
          Transform::from_translation(pos),
          TileGlyph { x, y },
          TilePng,
        ));
      } else {
        let [r, g, b] = tile.color();
        commands.spawn((
          Text2d::new(tile.glyph()),
          TextFont { font_size: TILE_SIZE, ..default() },
          TextColor(Color::srgba(r, g, b, 0.0)),
          Transform::from_translation(pos),
          TileGlyph { x, y },
        ));
      };

      if let Some(item) = level.items[y][x] {
        let [r, g, b] = item.color();
        commands.spawn((
          Text2d::new(item.glyph()),
          TextFont { font_size: TILE_SIZE, ..default() },
          TextColor(Color::srgba(r, g, b, 0.0)),
          Transform::from_translation(
            tile_screen_pos(x, y, ZONE_WIDTH, ZONE_HEIGHT) + Vec3::new(0.0, 0.0, 1.0)
          ),
          ItemGlyph { x, y }
        ));
      }
    }
  }
}

fn despawn_level_tiles(commands: &mut Commands, query: &Query<Entity, With<TileGlyph>>) {
  for entity in query.iter() {
    commands.entity(entity).despawn();
  }
}

fn rebuild_level(
  commands: &mut Commands,
  asset_server: &AssetServer,
  tile_query: &Query<Entity, With<TileGlyph>>,
  world: &ZoneWorld,
  zx: usize,
  zy: usize,
  z: usize,
) {
  despawn_level_tiles(commands, tile_query);
  spawn_level_tiles(commands, asset_server, world, zx, zy, z);
}

// ---------------------------------------------------------------------------
// Gravity
// ---------------------------------------------------------------------------

/// Show entities on the current z-level in the current zone only if their tile is in the FoV.
fn update_entity_visibility(
  player_q: Query<&PlayerPos, With<Player>>,
  fov: Res<Fov>,
  mut entity_q: Query<(&Location, &mut Visibility), With<GlyphVisual>>,
) {
  let Ok(pos) = player_q.single() else { return };
  let (player_zx, player_zy) = world_to_zone(pos.x, pos.y);
  for (location, mut vis) in entity_q.iter_mut() {
    *vis = if let Location::Coords { x, y, z, .. } = location
      && world_to_zone(*x, *y) == (player_zx, player_zy)
      && *z == pos.z
    {
      let (lx, ly) = world_to_local(*x, *y);
      if fov.0.is_visible(lx, ly) { Visibility::Visible } else { Visibility::Hidden }
    } else {
      Visibility::Hidden
    };
  }
}

fn should_fall(gw: &ZoneWorld, wx: i32, wy: i32, z: usize) -> bool {
  let (zx, zy) = world_to_zone(wx, wy);
  let (lx, ly) = world_to_local(wx, wy);
  let here = gw.zone(zx, zy, z).tiles[ly][lx];
  let below = z.checked_sub(1).map(|z1| gw.zone(zx, zy, z1).tiles[ly][lx]);
  here.causes_falling() || below.is_some_and(|t| t.causes_falling())
}

/// Drop entities with Gravity standing on open space or over a void.
/// Non-player entities: update their Location z. Player: rebuild level display.
fn apply_gravity(
  gw: Res<GameWorld>,
  asset_server: Res<AssetServer>,
  mut fov: ResMut<Fov>,
  mut commands: Commands,
  tile_query: Query<Entity, With<TileGlyph>>,
  mut player_q: Query<(&mut PlayerPos, &mut Transform), With<Player>>,
  mut entity_q: Query<&mut Location, (With<Gravity>, Without<Player>)>,
) {
  let Ok((mut pos, _transform)) = player_q.single_mut() else { return };
  let (player_zx, player_zy) = world_to_zone(pos.x, pos.y);

  // Non-player gravity entities: only simulate current zone
  for mut location in entity_q.iter_mut() {
    if let Location::Coords { x, y, z, .. } = *location
      && world_to_zone(x, y) == (player_zx, player_zy)
      && z == pos.z
      && z > 0
      && should_fall(&gw.0, x, y, z)
    {
      *location = Location::xyz(x, y, z - 1);
    }
  }

  // Player gravity
  if pos.z > 0 && should_fall(&gw.0, pos.x, pos.y, pos.z) {
    pos.z -= 1;
    let (lx, ly) = world_to_local(pos.x, pos.y);
    rebuild_level(&mut commands, &asset_server, &tile_query, &gw.0, player_zx, player_zy, pos.z);
    fov.0 = FovGrid::new(ZONE_WIDTH, ZONE_HEIGHT);
    compute_fov(&mut fov.0, gw.0.zone(player_zx, player_zy, pos.z), lx as i32, ly as i32, FOV_RADIUS);
  }
}

// ---------------------------------------------------------------------------
// Camera follow
// ---------------------------------------------------------------------------

fn camera_follow(
  player_q: Query<&Transform, (With<Player>, Without<Camera2d>)>,
  mut cam_q: Query<&mut Transform, (With<Camera2d>, Without<Player>)>
) {
  if let Ok(player_tf) = player_q.single()
    && let Ok(mut cam_tf) = cam_q.single_mut()
  {
    let target = player_tf.translation.truncate();
    let current = cam_tf.translation.truncate();
    let smoothed = current.lerp(target, 0.15);
    cam_tf.translation.x = smoothed.x;
    cam_tf.translation.y = smoothed.y;
  }
}

// ---------------------------------------------------------------------------
// FOV visuals
// ---------------------------------------------------------------------------

fn update_fov_visuals(
  fov: Res<Fov>,
  gw: Res<GameWorld>,
  player_q: Query<&PlayerPos, With<Player>>,
  mut glyph_tiles: Query<(&TileGlyph, &mut TextColor), Without<TilePng>>,
  mut sprite_tiles: Query<(&TileGlyph, &mut Sprite), With<TilePng>>,
) {
  let Ok(pos) = player_q.single() else { return };
  let (zx, zy) = world_to_zone(pos.x, pos.y);
  let level = gw.0.zone(zx, zy, pos.z);
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
}

// ---------------------------------------------------------------------------
// Mouse hover tile info
// ---------------------------------------------------------------------------

fn mouse_hover_tile(
  windows: Query<&Window>,
  camera_q: Query<(&Camera, &GlobalTransform), With<Camera2d>>,
  gw: Res<GameWorld>,
  fov: Res<Fov>,
  index: Res<TileEntityIndex>,
  player_q: Query<&PlayerPos, With<Player>>,
  named_q: Query<(&Named, Option<&Stats>)>,
  mut info_q: Query<&mut Text2d, With<TileInfoDisplay>>,
) {
  if let Ok(mut info_text) = info_q.single_mut()
    && let Ok(window) = windows.single()
    && let Ok((camera, cam_transform)) = camera_q.single()
    && let Ok(pos) = player_q.single()
  {
    *info_text = Text2d::new(
      tile_hover_text(window, camera, cam_transform, &gw, pos, &fov, &index, &named_q)
        .unwrap_or_default()
    );
  }
}

fn tile_hover_text(
  window: &Window,
  camera: &Camera,
  cam_transform: &GlobalTransform,
  gw: &GameWorld,
  player_pos: &PlayerPos,
  fov: &Fov,
  index: &TileEntityIndex,
  named_q: &Query<(&Named, Option<&Stats>)>,
) -> Option<String> {
  window.cursor_position()
    .and_then(|cursor| camera.viewport_to_world_2d(cam_transform, cursor).ok())
    .and_then(|world_pos| {
      let (tx, ty) = screen_to_tile(world_pos, ZONE_WIDTH, ZONE_HEIGHT);
      let (player_zx, player_zy) = world_to_zone(player_pos.x, player_pos.y);
      let level = gw.0.zone(player_zx, player_zy, player_pos.z);
      (tx >= 0 && ty >= 0 && (tx as usize) < level.width && (ty as usize) < level.height)
        .then(|| {
          let visible  = fov.0.is_visible(tx as usize, ty as usize);
          let revealed = fov.0.is_revealed(tx as usize, ty as usize);
          (visible || revealed).then(|| {
            let tile = level.tiles[ty as usize][tx as usize];
            let tile_line = if revealed && !visible {
              format!("({tx}, {ty})\n{} (remembered)", tile.name())
            } else {
              format!("({tx}, {ty})\n{}", tile.name())
            };
            // Convert local tile coords to world coords for index lookup
            let wx = (player_zx * ZONE_WIDTH) as i32 + tx;
            let wy = (player_zy * ZONE_HEIGHT) as i32 + ty;
            let entity_lines: String = visible.then(|| {
              index.0.get(&(wx, wy, player_pos.z))
                .and_then(|entities| entities.first())
                .and_then(|&e| named_q.get(e).ok())
                .map(|(named, stats)| {
                  let hp_bar: String = stats.map(|s| {
                    let filled = (((s.hp as f32 / s.max_hp as f32) * 10.0).round() as usize).min(10);
                    let empty  = 10usize.saturating_sub(filled);
                    format!("\n[{}{}] {}/{}", "█".repeat(filled), "░".repeat(empty), s.hp, s.max_hp)
                  }).unwrap_or_default();
                  format!("\n\n{}{}\n{}", named.name, hp_bar, named.flavor)
                })
                .unwrap_or_default()
            }).unwrap_or_default();
            format!("{tile_line}{entity_lines}")
          })
        })
        .flatten()
    })
}

// ---------------------------------------------------------------------------
// Time
// ---------------------------------------------------------------------------

fn advance_realtime(time: Res<Time>, mut clock: ResMut<Clock>) {
  clock.tick_realtime(time.delta_secs());
}

const ENEMY_ALERT_RADIUS: i32 = 8;

fn update_time_mode(
  mut clock: ResMut<Clock>,
  player_q: Query<&PlayerPos, With<Player>>,
  enemy_q: Query<&Location, With<Enemy>>,
) {
  let enemy_near = player_q.single().is_ok_and(|pos| {
    let (pzx, pzy) = world_to_zone(pos.x, pos.y);
    enemy_q.iter().any(|loc| {
      let Location::Coords { x, y, z, .. } = *loc else { return false };
      world_to_zone(x, y) == (pzx, pzy)
        && z == pos.z
        && (x - pos.x).abs() <= ENEMY_ALERT_RADIUS
        && (y - pos.y).abs() <= ENEMY_ALERT_RADIUS
    })
  });
  clock.mode = if enemy_near { TimeMode::TurnBased } else { TimeMode::RealTime };
}

fn update_hud(
  clock: Res<Clock>,
  player_q: Query<(&PlayerPos, &Inventory), With<Player>>,
  mut time_q: Query<
    (&mut Text2d, &mut TextColor),
    (With<TimeDisplay>, Without<LevelDisplay>, Without<InventoryDisplay>)
  >,
  mut level_q: Query<&mut Text2d, (With<LevelDisplay>, Without<TimeDisplay>, Without<InventoryDisplay>)>,
  mut inv_q: Query<&mut Text2d, (With<InventoryDisplay>, Without<TimeDisplay>, Without<LevelDisplay>)>,
) {
  if let Ok((mut text, mut color)) = time_q.single_mut() {
    let mode_str = match clock.mode {
      TimeMode::RealTime => "RT",
      TimeMode::TurnBased => "TB"
    };
    *text = Text2d::new(format!("{mode_str} T:{:.0}", clock.time));
    *color = TextColor(match clock.mode {
      TimeMode::RealTime => Color::srgb(0.5, 0.7, 0.5),
      TimeMode::TurnBased => Color::srgb(0.9, 0.4, 0.4)
    });
  }

  if let Ok(mut text) = level_q.single_mut()
    && let Ok((pos, _)) = player_q.single()
  {
    let (zx, zy) = world_to_zone(pos.x, pos.y);
    let label = match pos.z {
      0 => "Deep Cave (z=0)",
      1 => "Shallow Cave (z=1)",
      2 => "Surface (z=2)",
      3 => "Building Upper (z=3)",
      z => { *text = Text2d::new(format!("z={z} [{zx},{zy}]")); return; }
    };
    *text = Text2d::new(format!("{label} [{zx},{zy}]"));
  }

  // Inventory display
  if let Ok(mut text) = inv_q.single_mut()
    && let Ok((_, inventory)) = player_q.single()
  {
    let contents = if inventory.0.is_empty() {
      "Inv: (empty)".to_string()
    } else {
      let items: Vec<String> = inventory.0.iter()
        .map(|(item, count)| format!("{}x{}", item.name(), count))
        .collect();
      format!("Inv: {}", items.join(" "))
    };
    *text = Text2d::new(contents);
  }
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
  asset_server: Res<AssetServer>,
  mut ui: ResMut<UiState>,
  mut commands: Commands,
  pause_overlay_q: Query<Entity, With<PauseOverlay>>,
  interact_overlay_q: Query<Entity, With<InteractOverlay>>,
  mut gw: ResMut<GameWorld>,
  mut clock: ResMut<Clock>,
  mut fov: ResMut<Fov>,
  tile_query: Query<Entity, With<TileGlyph>>,
  mut player_query: Query<(&mut PlayerPos, &mut Transform, &mut Inventory), With<Player>>,
  mut exit: MessageWriter<AppExit>
) {
  // 1. Interact menu takes priority over pause menu
  if let InteractMenu::Open { options } = &ui.interact {
    if keys.just_pressed(KeyCode::Escape) {
      ui.interact = InteractMenu::Closed;
      despawn_interact_overlays(&mut commands, &interact_overlay_q);
    } else if let Some(idx) = [
        KeyCode::Digit1, KeyCode::Digit2, KeyCode::Digit3,
        KeyCode::Digit4, KeyCode::Digit5, KeyCode::Digit6,
        KeyCode::Digit7, KeyCode::Digit8, KeyCode::Digit9,
      ].iter().position(|k| keys.just_pressed(*k))
      && idx < options.len()
    {
      let option = options[idx].clone();
      ui.interact = InteractMenu::Closed;
      despawn_interact_overlays(&mut commands, &interact_overlay_q);
      execute_interaction(
        &option.action, &mut gw, &mut clock, &mut fov,
        &mut ui, &mut commands, &asset_server, &tile_query, &mut player_query
      );
    }
  } else {
  // 2. Pause menu
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
        spawn_pause_overlay(&mut commands, &ui);
      }
    }
    PauseMenu::Main => {
      if keys.just_pressed(KeyCode::Escape) || keys.just_pressed(KeyCode::Digit1) {
        ui.pause = PauseMenu::Closed;
        despawn_overlays(&mut commands, &pause_overlay_q);
      } else if keys.just_pressed(KeyCode::Digit2) {
        despawn_overlays(&mut commands, &pause_overlay_q);
        ui.pause = PauseMenu::Controls;
        spawn_pause_overlay(&mut commands, &ui);
      } else if keys.just_pressed(KeyCode::Digit3) {
        exit.write(AppExit::Success);
      }
    }
    PauseMenu::Controls => {
      if keys.just_pressed(KeyCode::Escape) {
        despawn_overlays(&mut commands, &pause_overlay_q);
        ui.pause = PauseMenu::Main;
        spawn_pause_overlay(&mut commands, &ui);
      }
    }
  }
  } // end else (pause menu)
}

fn spawn_pause_overlay(commands: &mut Commands, ui: &UiState) {
  let text = match ui.pause {
    PauseMenu::Main => {
      "\
            Paused\n\
            \n\
            1) Resume\n\
            2) Controls\n\
            3) Quit Game\n\
            \n\
            Esc to resume"
    }
    PauseMenu::Controls => {
      "\
            Controls\n\
            \n\
            WASD / Arrows   move (diagonal: hold two)\n\
            Space           use / interact\n\
            .               wait\n\
            ?               controls\n\
            Esc             menu / back"
    }
    PauseMenu::Closed => return
  };

  commands.spawn((
    Text2d::new(text),
    TextFont { font_size: 16.0, ..default() },
    TextColor(Color::srgb(0.9, 0.9, 0.9)),
    Transform::from_xyz(0.0, 0.0, 20.0),
    PauseOverlay
  ));
}

fn despawn_overlays(commands: &mut Commands, query: &Query<Entity, With<PauseOverlay>>) {
  for entity in query.iter() {
    commands.entity(entity).despawn();
  }
}

fn despawn_interact_overlays(commands: &mut Commands, query: &Query<Entity, With<InteractOverlay>>) {
  for entity in query.iter() {
    commands.entity(entity).despawn();
  }
}

// ---------------------------------------------------------------------------
// Dialogue
// ---------------------------------------------------------------------------

fn format_dialogue(speaker: &str, node: &trl::entities::DialogueNode) -> String {
  let choices = node
    .choices
    .iter()
    .enumerate()
    .map(|(i, c)| format!("  {}) {}", i + 1, c.text))
    .collect::<Vec<_>>()
    .join("\n");
  format!("{speaker}\n{}\n\n{}\n\n{choices}", "─".repeat(30), node.text)
}

fn handle_dialogue(
  keys: Res<ButtonInput<KeyCode>>,
  mut ui: ResMut<UiState>,
  mut commands: Commands,
  overlay_q: Query<Entity, With<DialogueOverlay>>,
) {
  if let DialogueState::Open { speaker, tree, node_name } = &ui.dialogue {
    let (speaker, tree, node_name) = (*speaker, *tree, *node_name);
    let node = tree.find(node_name);

    if overlay_q.is_empty() {
      // Spawn overlay for this node; defer input until next frame.
      commands.spawn((
        Text2d::new(format_dialogue(speaker, node)),
        TextFont { font_size: 15.0, ..default() },
        TextColor(Color::srgb(0.95, 0.9, 0.75)),
        Transform::from_xyz(0.0, 0.0, 20.0),
        DialogueOverlay,
      ));
    } else if keys.just_pressed(KeyCode::Escape) {
      ui.dialogue = DialogueState::Closed;
    } else if let Some(idx) = [
        KeyCode::Digit1, KeyCode::Digit2, KeyCode::Digit3,
        KeyCode::Digit4, KeyCode::Digit5, KeyCode::Digit6,
        KeyCode::Digit7, KeyCode::Digit8, KeyCode::Digit9,
      ].iter().position(|k| keys.just_pressed(*k))
      && idx < node.choices.len()
    {
      for e in overlay_q.iter() { commands.entity(e).despawn(); }
      ui.dialogue = node.choices[idx].next
        .map_or(DialogueState::Closed, |next_name| DialogueState::Open { speaker, tree, node_name: next_name });
    }
  } else {
    for e in overlay_q.iter() { commands.entity(e).despawn(); }
  }
}

// ---------------------------------------------------------------------------
// Interaction menu
// ---------------------------------------------------------------------------

fn gather_interactions(
  level: &level::Level,
  lx: i32,
  ly: i32,
  z: usize,
) -> Vec<InteractionOption> {
  let mut options = Vec::new();

  for dy in -1..=1 {
    for dx in -1..=1 {
      let (tx, ty) = (lx + dx, ly + dy);
      if let Some(tile) = level.get(tx, ty) {
        let here = dx == 0 && dy == 0;
        let dir = if here { "here".to_string() } else { direction_name(dx, dy) };

        match tile {
          Tile::StairsUp if here && z + 1 < WORLD_DEPTH => {
            options.push(InteractionOption {
              label: format!("Go upstairs ({dir})"),
              action: InteractionAction::Ascend
            });
          }
          Tile::StairsDown if here && z > 0 => {
            options.push(InteractionOption {
              label: format!("Go downstairs ({dir})"),
              action: InteractionAction::Descend
            });
          }
          Tile::Door => {
            options.push(InteractionOption {
              label: format!("Open door ({dir})"),
              action: InteractionAction::OpenDoor(tx, ty)
            });
          }
          _ => {}
        }
      }
    }
  }
  options
}

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

fn show_interact_menu(
  ui: &mut UiState,
  commands: &mut Commands,
  options: Vec<InteractionOption>
) {
  if options.is_empty() {
    return;
  }

  let text = options
    .iter()
    .enumerate()
    .map(|(i, opt)| format!("{}) {}", i + 1, opt.label))
    .collect::<Vec<_>>()
    .join("\n");

  commands.spawn((
    Text2d::new(format!("Use what?\n\n{text}\n\nEsc to cancel")),
    TextFont { font_size: 16.0, ..default() },
    TextColor(Color::srgb(0.9, 0.9, 0.8)),
    Transform::from_xyz(0.0, 60.0, 20.0),
    InteractOverlay
  ));

  ui.interact = InteractMenu::Open { options };
}

fn execute_interaction(
  action: &InteractionAction,
  gw: &mut ResMut<GameWorld>,
  clock: &mut Clock,
  fov: &mut ResMut<Fov>,
  ui: &mut UiState,
  commands: &mut Commands,
  asset_server: &AssetServer,
  tile_query: &Query<Entity, With<TileGlyph>>,
  player_query: &mut Query<(&mut PlayerPos, &mut Transform, &mut Inventory), With<Player>>
) {
  if let Ok((mut pos, mut transform, mut inventory)) = player_query.single_mut() {
    let (zx, zy) = world_to_zone(pos.x, pos.y);
    let (lx, ly) = world_to_local(pos.x, pos.y);
    match action {
      InteractionAction::Ascend => {
        if pos.z + 1 < WORLD_DEPTH {
          pos.z += 1;
          rebuild_level(commands, asset_server, tile_query, &gw.0, zx, zy, pos.z);
          fov.0 = FovGrid::new(ZONE_WIDTH, ZONE_HEIGHT);
          compute_fov(&mut fov.0, gw.0.zone(zx, zy, pos.z), lx as i32, ly as i32, FOV_RADIUS);
          transform.translation = tile_screen_pos(lx, ly, ZONE_WIDTH, ZONE_HEIGHT) + Vec3::Z;
          clock.advance(PlayerAction::Ascend.time_cost());
        }
      }
      InteractionAction::Descend => {
        if pos.z > 0 {
          pos.z -= 1;
          rebuild_level(commands, asset_server, tile_query, &gw.0, zx, zy, pos.z);
          fov.0 = FovGrid::new(ZONE_WIDTH, ZONE_HEIGHT);
          compute_fov(&mut fov.0, gw.0.zone(zx, zy, pos.z), lx as i32, ly as i32, FOV_RADIUS);
          transform.translation = tile_screen_pos(lx, ly, ZONE_WIDTH, ZONE_HEIGHT) + Vec3::Z;
          clock.advance(PlayerAction::Descend.time_cost());
        }
      }
      InteractionAction::OpenDoor(dx, dy) => {
        gw.0.zone_mut(zx, zy, pos.z).set(*dx, *dy, Tile::Floor);
        rebuild_level(commands, asset_server, tile_query, &gw.0, zx, zy, pos.z);
        compute_fov(&mut fov.0, gw.0.zone(zx, zy, pos.z), lx as i32, ly as i32, FOV_RADIUS);
        clock.advance(1.0);
      }
      InteractionAction::Talk { speaker, tree } => {
        ui.dialogue = DialogueState::Open { speaker, tree, node_name: tree.nodes[0].name };
      }
      InteractionAction::ChopTree(entity) => {
        commands.entity(*entity).despawn();
        *inventory.0.entry(level::Item::Wood).or_insert(0) += 1;
        clock.advance(2.0);
      }
      InteractionAction::PickUpItem(wx, wy) => {
        let (izx, izy) = world_to_zone(*wx, *wy);
        let (ilx, ily) = world_to_local(*wx, *wy);
        let level = gw.0.zone(izx, izy, pos.z);
        if ily < level.height && ilx < level.width {
          if let Some(item) = level.items[ily][ilx] {
            *inventory.0.entry(item).or_insert(0) += 1;
            gw.0.zone_mut(izx, izy, pos.z).set_item(*wx, *wy, None);
          }
        }
        clock.advance(1.0);
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
fn try_zone_transition(
  pos: &mut PlayerPos,
  transform: &mut Transform,
  gw: &ZoneWorld,
  fov: &mut FovGrid,
  commands: &mut Commands,
  asset_server: &AssetServer,
  tile_query: &Query<Entity, With<TileGlyph>>,
  dx: i32,
  dy: i32,
) -> bool {
  let lx = pos.x % ZONE_WIDTH as i32;
  let ly = pos.y % ZONE_HEIGHT as i32;
  let (zx, zy) = world_to_zone(pos.x, pos.y);

  let nlx = lx + dx;
  let nly = ly + dy;

  // Only handle transitions — steps within the zone are handled by normal resolve_move
  let x_exit = nlx < 0 || nlx >= ZONE_WIDTH as i32;
  let y_exit = nly < 0 || nly >= ZONE_HEIGHT as i32;
  if !x_exit && !y_exit {
    return false;
  }

  // Compute adjacent zone and wrapped local position
  let (mut new_zx, mut new_zy) = (zx as i32, zy as i32);
  let (mut new_lx, mut new_ly) = (nlx, nly);

  if nlx < 0 {
    new_zx -= 1;
    new_lx = ZONE_WIDTH as i32 - 1;
  } else if nlx >= ZONE_WIDTH as i32 {
    new_zx += 1;
    new_lx = 0;
  }

  if nly < 0 {
    new_zy -= 1;
    new_ly = ZONE_HEIGHT as i32 - 1;
  } else if nly >= ZONE_HEIGHT as i32 {
    new_zy += 1;
    new_ly = 0;
  }

  // Block at world boundary
  if !gw.in_bounds(new_zx, new_zy) {
    return true; // consumed the move, player doesn't move
  }

  // Check walkability in the destination zone
  let dest_zone = gw.zone(new_zx as usize, new_zy as usize, pos.z);
  if !dest_zone.walkable(new_lx, new_ly) {
    return true; // consumed — tile is impassable
  }

  // Perform transition: update world-space position
  pos.x = new_zx * ZONE_WIDTH as i32 + new_lx;
  pos.y = new_zy * ZONE_HEIGHT as i32 + new_ly;

  rebuild_level(commands, asset_server, tile_query, gw, new_zx as usize, new_zy as usize, pos.z);
  *fov = FovGrid::new(ZONE_WIDTH, ZONE_HEIGHT);
  compute_fov(fov, gw.zone(new_zx as usize, new_zy as usize, pos.z), new_lx, new_ly, FOV_RADIUS);
  transform.translation =
    tile_screen_pos(new_lx as usize, new_ly as usize, ZONE_WIDTH, ZONE_HEIGHT) + Vec3::Z;

  true
}

fn player_input(
  keys: Res<ButtonInput<KeyCode>>,
  time: Res<Time>,
  asset_server: Res<AssetServer>,
  gw: Res<GameWorld>,
  ui: Res<UiState>,
  mut clock: ResMut<Clock>,
  mut fov: ResMut<Fov>,
  index: Res<TileEntityIndex>,
  mut commands: Commands,
  tile_query: Query<Entity, With<TileGlyph>>,
  mut player_query: Query<(&mut PlayerPos, &mut Transform, &Stats, &mut Inventory), With<Player>>,
  mut enemy_query: Query<&mut Stats, (With<Enemy>, Without<Player>)>,
  collidable_q: Query<&Collidable>,
) {
  if !ui.any_open()
    && let Ok((mut pos, mut transform, stats, mut inventory)) = player_query.single_mut()
  {
    let player_attack = stats.attack;
    clock.move_cooldown = (clock.move_cooldown - time.delta_secs()).max(0.0);

    if keys.just_pressed(KeyCode::Period)
      && !keys.pressed(KeyCode::ShiftLeft)
      && !keys.pressed(KeyCode::ShiftRight)
    {
      clock.advance(PlayerAction::Wait.time_cost());
    } else if any_direction_pressed(&keys) && clock.move_cooldown == 0.0 {
      let (zx, zy) = world_to_zone(pos.x, pos.y);
      let (lx, ly) = (
        (pos.x as usize % ZONE_WIDTH) as i32,
        (pos.y as usize % ZONE_HEIGHT) as i32,
      );
      let level = gw.0.zone(zx, zy, pos.z);
      let dir = read_direction(&keys);
      let (raw_dx, raw_dy) = (dir.0, dir.1);

      let transitioned = try_zone_transition(
        &mut pos, &mut transform, &gw.0, &mut fov.0,
        &mut commands, &asset_server, &tile_query,
        raw_dx, raw_dy,
      );

      if transitioned {
        clock.advance(PlayerAction::Move { dx: raw_dx, dy: raw_dy }.time_cost());
        clock.move_cooldown = MOVE_COOLDOWN;
      } else {
        let (dx, dy) = resolve_move(level, lx, ly, raw_dx, raw_dy);

        if (dx, dy) != (0, 0) {
          let target_wx = pos.x + dx;
          let target_wy = pos.y + dy;

          // Check for an enemy to bump-attack
          let enemy_hit = index.0.get(&(target_wx, target_wy, pos.z))
            .and_then(|entities| entities.iter().find(|&&e| enemy_query.get(e).is_ok()).copied());

          if let Some(hostile) = enemy_hit {
            if let Ok(mut es) = enemy_query.get_mut(hostile) {
              es.hp -= player_attack;
              if es.hp <= 0 {
                commands.entity(hostile).despawn();
              }
            }
          } else {
            // Block if a collidable non-enemy entity occupies the tile
            let blocked = index.0.get(&(target_wx, target_wy, pos.z))
              .is_some_and(|entities| entities.iter().any(|&e| {
                collidable_q.get(e).is_ok_and(|c| c.0)
              }));

            if !blocked {
              pos.x += dx;
              pos.y += dy;
              let (nlx, nly) = world_to_local(pos.x, pos.y);
              transform.translation =
                tile_screen_pos(nlx, nly, ZONE_WIDTH, ZONE_HEIGHT) + Vec3::Z;
              compute_fov(&mut fov.0, level, nlx as i32, nly as i32, FOV_RADIUS);

              // Auto-pickup items underfoot
              let (izx, izy) = world_to_zone(pos.x, pos.y);
              let (ilx, ily) = world_to_local(pos.x, pos.y);
              let lvl = gw.0.zone(izx, izy, pos.z);
              if ily < lvl.height && ilx < lvl.width {
                if let Some(item) = lvl.items[ily][ilx] {
                  *inventory.0.entry(item).or_insert(0) += 1;
                }
              }
            }
          }

          clock.advance(PlayerAction::Move { dx, dy }.time_cost());
          clock.move_cooldown = MOVE_COOLDOWN;
        }
      }
    }
  }
}

/// Separate system for Space key interactions to avoid Bevy's system param limit.
fn handle_interact(
  keys: Res<ButtonInput<KeyCode>>,
  gw: Res<GameWorld>,
  mut ui: ResMut<UiState>,
  index: Res<TileEntityIndex>,
  mut commands: Commands,
  player_q: Query<&PlayerPos, With<Player>>,
  dialogue_q: Query<(&Named, &Dialogue)>,
  tree_q: Query<Entity, With<Tree>>,
) {
  if ui.any_open() || !keys.just_pressed(KeyCode::Space)
  {
    return;
  }

  let Ok(pos) = player_q.single() else { return };

  let (zx, zy) = world_to_zone(pos.x, pos.y);
  let (lx, ly) = (
    (pos.x as usize % ZONE_WIDTH) as i32,
    (pos.y as usize % ZONE_HEIGHT) as i32,
  );
  let level = gw.0.zone(zx, zy, pos.z);
  let mut options = gather_interactions(level, lx, ly, pos.z);

  // Entity-based interactions: trees, dialogue, items
  for (dx, dy) in (-1i32..=1).flat_map(|dy| (-1i32..=1).map(move |dx| (dx, dy))) {
    let wx = pos.x + dx;
    let wy = pos.y + dy;
    if let Some(entities) = index.0.get(&(wx, wy, pos.z)) {
      for &e in entities.iter() {
        if tree_q.get(e).is_ok() {
          let dir = if dx == 0 && dy == 0 { "here".to_string() } else { direction_name(dx, dy) };
          options.push(InteractionOption {
            label: format!("Chop tree ({dir})"),
            action: InteractionAction::ChopTree(e),
          });
        }
        if let Ok((named, dialogue)) = dialogue_q.get(e) {
          options.push(InteractionOption {
            label: format!("Talk to {}", named.name),
            action: InteractionAction::Talk { speaker: named.name, tree: dialogue.0 },
          });
        }
      }
    }
    // Item pickup
    let (ilx, ily) = (wx as usize % ZONE_WIDTH, wy as usize % ZONE_HEIGHT);
    if ily < level.height && ilx < level.width
      && level.items[ily][ilx].is_some()
    {
      let dir = if dx == 0 && dy == 0 { "here".to_string() } else { direction_name(dx, dy) };
      options.push(InteractionOption {
        label: format!("Pick up item ({dir})"),
        action: InteractionAction::PickUpItem(wx, wy),
      });
    }
  }

  show_interact_menu(&mut ui, &mut commands, options);
}
