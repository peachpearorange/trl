mod level;
mod combat;

use {
  bevy::prelude::*,
  combat::{TileEntityIndex, enemy_ai, maintain_tile_index},
  level::{FovGrid, Tile, World, build_test_world, compute_fov},
  trl::entities::{Enemy, Glyph, Gravity, Location, Named, Spawnable, Stats, Wearing},
};

const TILE_SIZE: f32 = 16.0;
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

#[derive(Resource)]
struct GameClock {
  time: f32,
  mode: TimeMode
}

impl GameClock {
  fn new() -> Self { GameClock { time: 0.0, mode: TimeMode::RealTime } }

  fn advance(&mut self, cost: f32) { self.time += cost; }

  fn tick_realtime(&mut self, dt: f32) {
    if self.mode == TimeMode::RealTime {
      self.time += dt;
    }
  }
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
  OpenDoor(i32, i32)
}

#[derive(Resource, Default)]
enum InteractMenu {
  #[default]
  Closed,
  Open {
    options: Vec<InteractionOption>
  }
}

// ---------------------------------------------------------------------------
// Pause / Esc menu
// ---------------------------------------------------------------------------

#[derive(Resource, Default, PartialEq, Eq)]
enum PauseMenu {
  #[default]
  Closed,
  Main,
  Controls
}

// ---------------------------------------------------------------------------
// Resources & components
// ---------------------------------------------------------------------------

#[derive(Resource)]
struct GameWorld(World);

#[derive(Resource)]
struct CurrentZ(usize);

#[derive(Resource)]
struct MoveCooldown(f32);

#[derive(Resource)]
struct Fov(FovGrid);

#[derive(Component)]
struct Player;

#[derive(Component)]
struct PlayerPos {
  x: i32,
  y: i32
}

#[derive(Component)]
struct TileGlyph {
  x: usize,
  y: usize
}

#[derive(Component)]
struct ItemGlyph {
  x: usize,
  y: usize
}

#[derive(Component)]
struct InteractOverlay;

#[derive(Component)]
struct PauseOverlay;

#[derive(Component)]
struct HudElement;

#[derive(Component)]
struct TimeDisplay;

#[derive(Component)]
struct LevelDisplay;

#[derive(Component)]
struct TileInfoDisplay;

/// Marker for entities that have had their Text2d visual set up.
#[derive(Component)]
struct GlyphVisual;

// ---------------------------------------------------------------------------
// Glyph rendering systems
// ---------------------------------------------------------------------------

fn setup_glyph_visuals(
  mut commands: Commands,
  gw: Res<GameWorld>,
  query: Query<(Entity, &Glyph, &Location), (Added<Glyph>, Without<GlyphVisual>)>,
) {
  for (entity, glyph, location) in query.iter() {
    if let Location::Coords { x, y, .. } = location {
      let pos = tile_screen_pos(*x as usize, *y as usize, gw.0.width, gw.0.height)
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
  gw: Res<GameWorld>,
  mut query: Query<(&Location, &mut Transform), (With<GlyphVisual>, Changed<Location>)>,
) {
  for (location, mut transform) in query.iter_mut() {
    if let Location::Coords { x, y, .. } = location {
      transform.translation =
        tile_screen_pos(*x as usize, *y as usize, gw.0.width, gw.0.height)
          + Vec3::new(0.0, 0.0, 2.0);
    }
  }
}

// ---------------------------------------------------------------------------
// App
// ---------------------------------------------------------------------------

fn main() {
  let world = build_test_world();
  let start_z = 2;
  let fov = FovGrid::new(world.width, world.height);

  App::new()
    .add_plugins(DefaultPlugins.set(WindowPlugin {
      primary_window: Some(Window {
        title: "trl".into(),
        resolution: (1200u32, 800u32).into(),
        ..default()
      }),
      ..default()
    }))
    .insert_resource(ClearColor(Color::srgb(0.02, 0.02, 0.05)))
    .insert_resource(GameWorld(world))
    .insert_resource(CurrentZ(start_z))
    .insert_resource(GameClock::new())
    .insert_resource(MoveCooldown(0.0))
    .insert_resource(InteractMenu::default())
    .insert_resource(PauseMenu::default())
    .insert_resource(Fov(fov))
    .insert_resource(TileEntityIndex::default())
    .add_systems(Startup, setup)
    .add_systems(
      Update,
      (
        maintain_tile_index,
        setup_glyph_visuals,
        sync_entity_positions,
        update_entity_visibility,
        advance_realtime,
        handle_menus,
        player_input,
        ApplyDeferred,
        apply_gravity,
        enemy_ai,
        camera_follow,
        update_fov_visuals,
        mouse_hover_tile,
        update_hud,
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

fn screen_to_tile(world_pos: Vec2, w: usize, h: usize) -> (i32, i32) {
  let tx = (world_pos.x / TILE_SIZE + w as f32 / 2.0).floor() as i32;
  let ty = (h as f32 / 2.0 - world_pos.y / TILE_SIZE).floor() as i32;
  (tx, ty)
}

// ---------------------------------------------------------------------------
// Setup
// ---------------------------------------------------------------------------

fn setup(
  mut commands: Commands,
  gw: Res<GameWorld>,
  cz: Res<CurrentZ>,
  mut fov: ResMut<Fov>
) {
  let cam_entity = commands.spawn(Camera2d).id();

  spawn_level_tiles(&mut commands, &gw.0, cz.0);

  let level = gw.0.level(cz.0);
  let (px, py) = find_walkable(level, 35, 29);
  compute_fov(&mut fov.0, level, px, py, FOV_RADIUS);

  commands.spawn((
    Text2d::new("@"),
    TextFont { font_size: TILE_SIZE, ..default() },
    TextColor(Color::srgb(1.0, 1.0, 0.0)),
    Transform::from_translation(
      tile_screen_pos(px as usize, py as usize, gw.0.width, gw.0.height) + Vec3::Z
    ),
    Player,
    PlayerPos { x: px, y: py },
    Stats { hp: 20, max_hp: 20, attack: 5, move_speed: 3.0, attack_speed: 1.0 },
  ));

  // Spawn enemies and NPCs
  let (ex1, ey1) = find_walkable(level, px + 5, py);
  let (ex2, ey2) = find_walkable(level, px + 3, py + 4);
  let (cx1, cy1) = find_walkable(level, px - 4, py + 2);

  Spawnable::rat_soldier().spawn_at(&mut commands, ex1, ey1, cz.0);
  Spawnable::armored_rat_soldier().spawn_at(&mut commands, ex2, ey2, cz.0);
  Spawnable::catgirl().spawn_at(&mut commands, cx1, cy1, cz.0);

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

  commands.entity(cam_entity).add_children(&[time_id, level_id, tile_info_id]);
}

fn find_walkable(level: &level::Level, hint_x: i32, hint_y: i32) -> (i32, i32) {
  if level.walkable(hint_x, hint_y) {
    return (hint_x, hint_y);
  }
  for r in 1..30 {
    for dy in -r..=r {
      for dx in -r..=r {
        if level.walkable(hint_x + dx, hint_y + dy) {
          return (hint_x + dx, hint_y + dy);
        }
      }
    }
  }
  (hint_x, hint_y)
}

// ---------------------------------------------------------------------------
// Level rendering
// ---------------------------------------------------------------------------

fn spawn_level_tiles(commands: &mut Commands, world: &World, z: usize) {
  let level = world.level(z);
  for y in 0..level.height {
    for x in 0..level.width {
      let tile = level.tiles[y][x];
      if tile == Tile::Air {
        continue;
      }
      let [r, g, b] = tile.color();
      commands.spawn((
        Text2d::new(tile.glyph()),
        TextFont { font_size: TILE_SIZE, ..default() },
        TextColor(Color::srgba(r, g, b, 0.0)),
        Transform::from_translation(tile_screen_pos(x, y, world.width, world.height)),
        TileGlyph { x, y }
      ));

      if let Some(item) = level.items[y][x] {
        let [r, g, b] = item.color();
        commands.spawn((
          Text2d::new(item.glyph()),
          TextFont { font_size: TILE_SIZE, ..default() },
          TextColor(Color::srgba(r, g, b, 0.0)),
          Transform::from_translation(
            tile_screen_pos(x, y, world.width, world.height) + Vec3::new(0.0, 0.0, 1.0)
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
  tile_query: &Query<Entity, With<TileGlyph>>,
  world: &World,
  z: usize
) {
  despawn_level_tiles(commands, tile_query);
  spawn_level_tiles(commands, world, z);
}

// ---------------------------------------------------------------------------
// Gravity
// ---------------------------------------------------------------------------

/// Show entities on the current z-level, hide those on other levels.
fn update_entity_visibility(
  cz: Res<CurrentZ>,
  mut entity_q: Query<(&Location, &mut Visibility), With<GlyphVisual>>,
) {
  for (location, mut vis) in entity_q.iter_mut() {
    *vis = if let Location::Coords { z, .. } = location
      && *z == cz.0
    {
      Visibility::Visible
    } else {
      Visibility::Hidden
    };
  }
}

fn should_fall(gw: &World, x: i32, y: i32, z: usize) -> bool {
  let here = gw.level(z).tiles[y as usize][x as usize];
  let below = z.checked_sub(1).map(|z1| gw.level(z1).tiles[y as usize][x as usize]);
  here.causes_falling() || below.is_some_and(|t| t.causes_falling())
}

/// Drop entities with Gravity standing on open space or over a void.
/// Non-player entities: update their Location z. Player: rebuild level display.
fn apply_gravity(
  gw: Res<GameWorld>,
  mut cz: ResMut<CurrentZ>,
  mut fov: ResMut<Fov>,
  mut commands: Commands,
  tile_query: Query<Entity, With<TileGlyph>>,
  mut player_q: Query<(&mut PlayerPos, &mut Transform), With<Player>>,
  mut entity_q: Query<&mut Location, (With<Gravity>, Without<Player>)>,
) {
  // Non-player gravity entities: just update their z.
  for mut location in entity_q.iter_mut() {
    if let Location::Coords { x, y, z } = *location
      && z > 0
      && should_fall(&gw.0, x, y, z)
    {
      *location = Location::Coords { x, y, z: z - 1 };
    }
  }

  // Player: fall and rebuild the level display.
  if let Ok((pos, mut transform)) = player_q.single_mut()
    && cz.0 > 0
    && should_fall(&gw.0, pos.x, pos.y, cz.0)
  {
    cz.0 -= 1;
    rebuild_level(&mut commands, &tile_query, &gw.0, cz.0);
    fov.0 = FovGrid::new(gw.0.width, gw.0.height);
    compute_fov(&mut fov.0, gw.0.level(cz.0), pos.x, pos.y, FOV_RADIUS);
    transform.translation =
      tile_screen_pos(pos.x as usize, pos.y as usize, gw.0.width, gw.0.height) + Vec3::Z;
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
  cz: Res<CurrentZ>,
  mut tiles: Query<(&TileGlyph, &mut TextColor)>
) {
  let level = gw.0.level(cz.0);
  for (tg, mut color) in tiles.iter_mut() {
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
}

// ---------------------------------------------------------------------------
// Mouse hover tile info
// ---------------------------------------------------------------------------

fn mouse_hover_tile(
  windows: Query<&Window>,
  camera_q: Query<(&Camera, &GlobalTransform), With<Camera2d>>,
  gw: Res<GameWorld>,
  cz: Res<CurrentZ>,
  fov: Res<Fov>,
  index: Res<TileEntityIndex>,
  named_q: Query<(&Named, Option<&Stats>)>,
  mut info_q: Query<&mut Text2d, With<TileInfoDisplay>>,
) {
  let Ok(mut info_text) = info_q.single_mut() else { return };
  let Ok(window) = windows.single() else { return };
  let Ok((camera, cam_transform)) = camera_q.single() else { return };

  let Some(cursor_pos) = window.cursor_position() else {
    *info_text = Text2d::new("");
    return;
  };
  let Ok(world_pos) = camera.viewport_to_world_2d(cam_transform, cursor_pos) else {
    *info_text = Text2d::new("");
    return;
  };

  let (tx, ty) = screen_to_tile(world_pos, gw.0.width, gw.0.height);
  let level = gw.0.level(cz.0);

  let in_bounds = tx >= 0
    && ty >= 0
    && (tx as usize) < level.width
    && (ty as usize) < level.height;

  if !in_bounds {
    *info_text = Text2d::new("");
    return;
  }

  let visible = fov.0.is_visible(tx as usize, ty as usize);
  let revealed = fov.0.is_revealed(tx as usize, ty as usize);

  if !visible && !revealed {
    *info_text = Text2d::new("");
    return;
  }

  let tile = level.tiles[ty as usize][tx as usize];
  let tile_line = if revealed && !visible {
    format!("({tx}, {ty})\n{} (remembered)", tile.name())
  } else {
    format!("({tx}, {ty})\n{}", tile.name())
  };

  // Entity info — only show for currently visible tiles
  let entity_lines = if visible {
    index
      .0
      .get(&(tx, ty, cz.0))
      .and_then(|entities| entities.first())
      .and_then(|&e| named_q.get(e).ok())
      .map(|(named, stats)| {
        let hp_bar = stats.map(|s| {
          let filled = (((s.hp as f32 / s.max_hp as f32) * 10.0).round() as usize).min(10);
          let empty = 10usize.saturating_sub(filled);
          format!("\n[{}{}] {}/{}", "█".repeat(filled), "░".repeat(empty), s.hp, s.max_hp)
        });
        format!("\n\n{}{}\n{}", named.name, hp_bar.unwrap_or_default(), named.flavor)
      })
      .unwrap_or_default()
  } else {
    String::new()
  };

  *info_text = Text2d::new(format!("{tile_line}{entity_lines}"));
}

// ---------------------------------------------------------------------------
// Time
// ---------------------------------------------------------------------------

fn advance_realtime(time: Res<Time>, mut clock: ResMut<GameClock>) {
  clock.tick_realtime(time.delta_secs());
}

fn update_hud(
  clock: Res<GameClock>,
  cz: Res<CurrentZ>,
  mut time_q: Query<
    (&mut Text2d, &mut TextColor),
    (With<TimeDisplay>, Without<LevelDisplay>)
  >,
  mut level_q: Query<&mut Text2d, (With<LevelDisplay>, Without<TimeDisplay>)>
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

  if let Ok(mut text) = level_q.single_mut() {
    let label = match cz.0 {
      0 => "Deep Cave (z=0)",
      1 => "Shallow Cave (z=1)",
      2 => "Surface (z=2)",
      3 => "Building Upper (z=3)",
      z => return *text = Text2d::new(format!("z={z}"))
    };
    *text = Text2d::new(label);
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
  mut pause: ResMut<PauseMenu>,
  mut interact: ResMut<InteractMenu>,
  mut commands: Commands,
  pause_overlay_q: Query<Entity, With<PauseOverlay>>,
  interact_overlay_q: Query<Entity, With<InteractOverlay>>,
  mut gw: ResMut<GameWorld>,
  mut cz: ResMut<CurrentZ>,
  mut clock: ResMut<GameClock>,
  mut fov: ResMut<Fov>,
  tile_query: Query<Entity, With<TileGlyph>>,
  mut player_query: Query<(&mut PlayerPos, &mut Transform), With<Player>>,
  mut exit: MessageWriter<AppExit>
) {
  // 1. Interact menu takes priority
  if let InteractMenu::Open { options } = &*interact {
    if keys.just_pressed(KeyCode::Escape) {
      *interact = InteractMenu::Closed;
      despawn_interact_overlays(&mut commands, &interact_overlay_q);
      return;
    }

    let selection = [
      KeyCode::Digit1, KeyCode::Digit2, KeyCode::Digit3,
      KeyCode::Digit4, KeyCode::Digit5, KeyCode::Digit6,
      KeyCode::Digit7, KeyCode::Digit8, KeyCode::Digit9
    ]
    .iter()
    .position(|k| keys.just_pressed(*k));

    if let Some(idx) = selection
      && idx < options.len()
    {
      let option = options[idx].clone();
      *interact = InteractMenu::Closed;
      despawn_interact_overlays(&mut commands, &interact_overlay_q);
      execute_interaction(
        &option.action, &mut gw, &mut cz, &mut clock, &mut fov,
        &mut commands, &tile_query, &mut player_query
      );
    }
    return;
  }

  // 2. Pause menu
  match *pause {
    PauseMenu::Closed => {
      let open = keys.just_pressed(KeyCode::Escape)
        || (keys.just_pressed(KeyCode::Slash)
          && (keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight)));

      if open {
        *pause = if keys.just_pressed(KeyCode::Slash) {
          PauseMenu::Controls
        } else {
          PauseMenu::Main
        };
        spawn_pause_overlay(&mut commands, &pause);
      }
    }
    PauseMenu::Main => {
      if keys.just_pressed(KeyCode::Escape) || keys.just_pressed(KeyCode::Digit1) {
        *pause = PauseMenu::Closed;
        despawn_overlays(&mut commands, &pause_overlay_q);
      } else if keys.just_pressed(KeyCode::Digit2) {
        despawn_overlays(&mut commands, &pause_overlay_q);
        *pause = PauseMenu::Controls;
        spawn_pause_overlay(&mut commands, &pause);
      } else if keys.just_pressed(KeyCode::Digit3) {
        exit.write(AppExit::Success);
      }
    }
    PauseMenu::Controls => {
      if keys.just_pressed(KeyCode::Escape) {
        despawn_overlays(&mut commands, &pause_overlay_q);
        *pause = PauseMenu::Main;
        spawn_pause_overlay(&mut commands, &pause);
      }
    }
  }
}

fn spawn_pause_overlay(commands: &mut Commands, state: &PauseMenu) {
  let text = match state {
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
// Interaction menu
// ---------------------------------------------------------------------------

fn gather_interactions(
  level: &level::Level,
  px: i32,
  py: i32,
  z: usize,
  depth: usize
) -> Vec<InteractionOption> {
  let mut options = Vec::new();

  for dy in -1..=1 {
    for dx in -1..=1 {
      let (tx, ty) = (px + dx, py + dy);
      if let Some(tile) = level.get(tx, ty) {
        let here = dx == 0 && dy == 0;
        let dir = if here { "here".to_string() } else { direction_name(dx, dy) };

        match tile {
          Tile::StairsUp if here && z + 1 < depth => {
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
  menu: &mut ResMut<InteractMenu>,
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

  **menu = InteractMenu::Open { options };
}

fn execute_interaction(
  action: &InteractionAction,
  gw: &mut ResMut<GameWorld>,
  cz: &mut ResMut<CurrentZ>,
  clock: &mut ResMut<GameClock>,
  fov: &mut ResMut<Fov>,
  commands: &mut Commands,
  tile_query: &Query<Entity, With<TileGlyph>>,
  player_query: &mut Query<(&mut PlayerPos, &mut Transform), With<Player>>
) {
  let Ok((pos, mut transform)) = player_query.single_mut() else { return };

  match action {
    InteractionAction::Ascend => {
      if cz.0 + 1 < gw.0.depth() {
        cz.0 += 1;
        rebuild_level(commands, tile_query, &gw.0, cz.0);
        fov.0 = FovGrid::new(gw.0.width, gw.0.height);
        compute_fov(&mut fov.0, gw.0.level(cz.0), pos.x, pos.y, FOV_RADIUS);
        transform.translation =
          tile_screen_pos(pos.x as usize, pos.y as usize, gw.0.width, gw.0.height) + Vec3::Z;
        clock.advance(PlayerAction::Ascend.time_cost());
      }
    }
    InteractionAction::Descend => {
      if cz.0 > 0 {
        cz.0 -= 1;
        rebuild_level(commands, tile_query, &gw.0, cz.0);
        fov.0 = FovGrid::new(gw.0.width, gw.0.height);
        compute_fov(&mut fov.0, gw.0.level(cz.0), pos.x, pos.y, FOV_RADIUS);
        transform.translation =
          tile_screen_pos(pos.x as usize, pos.y as usize, gw.0.width, gw.0.height) + Vec3::Z;
        clock.advance(PlayerAction::Descend.time_cost());
      }
    }
    InteractionAction::OpenDoor(dx, dy) => {
      gw.0.level_mut(cz.0).set(*dx, *dy, Tile::Floor);
      rebuild_level(commands, tile_query, &gw.0, cz.0);
      compute_fov(&mut fov.0, gw.0.level(cz.0), pos.x, pos.y, FOV_RADIUS);
      clock.advance(1.0);
    }
  }
}

// ---------------------------------------------------------------------------
// Player input
// ---------------------------------------------------------------------------

fn player_input(
  keys: Res<ButtonInput<KeyCode>>,
  time: Res<Time>,
  gw: Res<GameWorld>,
  pause: Res<PauseMenu>,
  mut menu: ResMut<InteractMenu>,
  mut clock: ResMut<GameClock>,
  mut cooldown: ResMut<MoveCooldown>,
  mut fov: ResMut<Fov>,
  cz: Res<CurrentZ>,
  index: Res<TileEntityIndex>,
  mut commands: Commands,
  mut player_query: Query<(&mut PlayerPos, &mut Transform, &Stats), With<Player>>,
  mut enemy_query: Query<(&mut Stats, Option<&Wearing>), (With<Enemy>, Without<Player>)>,
) {
  if *pause != PauseMenu::Closed || matches!(*menu, InteractMenu::Open { .. }) {
    return;
  }

  // Read player attack before mutable borrow of pos
  let player_attack = player_query.single().ok().map(|(_, _, s)| s.attack).unwrap_or(5);

  let Ok((mut pos, mut transform, _)) = player_query.single_mut() else { return };

  if cooldown.0 > 0.0 {
    cooldown.0 = (cooldown.0 - time.delta_secs()).max(0.0);
  }

  // space -> interaction menu
  if keys.just_pressed(KeyCode::Space) {
    let level = gw.0.level(cz.0);
    let options = gather_interactions(level, pos.x, pos.y, cz.0, gw.0.depth());
    show_interact_menu(&mut menu, &mut commands, options);
    return;
  }

  // wait
  if keys.just_pressed(KeyCode::Period)
    && !keys.pressed(KeyCode::ShiftLeft)
    && !keys.pressed(KeyCode::ShiftRight)
  {
    clock.advance(PlayerAction::Wait.time_cost());
    return;
  }

  // movement
  let dir = read_direction(&keys);
  if dir == (0, 0) || cooldown.0 > 0.0 || !any_direction_pressed(&keys) {
    return;
  }

  let level = gw.0.level(cz.0);
  let (dx, dy) = resolve_move(level, pos.x, pos.y, dir.0, dir.1);

  // Bump attack: if target tile has a hostile entity, attack instead of moving
  let target_x = pos.x + dx;
  let target_y = pos.y + dy;
  if dx != 0 || dy != 0 {
    let hostile_entity = index
      .0
      .get(&(target_x, target_y, cz.0))
      .and_then(|entities| entities.iter().find(|&&e| enemy_query.get(e).is_ok()))
      .copied();

    if let Some(enemy_entity) = hostile_entity
      && let Ok((mut enemy_stats, enemy_wearing)) = enemy_query.get_mut(enemy_entity)
    {
      let died = combat::bump_attack(player_attack, &mut enemy_stats, enemy_wearing);
      if died {
        commands.entity(enemy_entity).despawn();
      }
      clock.advance(PlayerAction::Move { dx, dy }.time_cost());
      cooldown.0 = MOVE_COOLDOWN;
      return;
    }
  }

  if (dx, dy) != (0, 0) {
    let action = PlayerAction::Move { dx, dy };
    pos.x += dx;
    pos.y += dy;
    transform.translation =
      tile_screen_pos(pos.x as usize, pos.y as usize, gw.0.width, gw.0.height) + Vec3::Z;
    clock.advance(action.time_cost());
    cooldown.0 = MOVE_COOLDOWN;
    compute_fov(&mut fov.0, level, pos.x, pos.y, FOV_RADIUS);
  }
}
