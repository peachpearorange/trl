//! Ranged attack trajectory overlay — shows a yellow path from the player to the
//! hovered tile when a ranged ability is selected, stopping at walls.

use bevy::prelude::*;
use std::f32::consts::{FRAC_PI_2, PI};
use crate::{
  CurrentZone, Player, PlayerPos, TILE_SIZE,
  abilities::TargetingState,
  sprites::{PaletteImageCache, palette_sprite_handle},
  game_pane_rect, tile_screen_pos, world_to_level_cell,
};

const LINE_NS:      &str = "textures/space_qud/lines N S.png";
const LINE_N_NE:    &str = "textures/space_qud/lines N NE.png";
const LINE_N_SE:    &str = "textures/space_qud/lines N SE.png";
const LINE_CORNER:  &str = "textures/space_qud/lines N E.png";
const LINE_DIAG_NW: &str = "textures/space_qud/lines NW SE.png";
const LINE_DIAG_NE: &str = "textures/space_qud/lines NE SW.png";
const LINE_HALF_N:  &str = "textures/space_qud/lines N.png";
const LINE_HALF_NE: &str = "textures/space_qud/lines NE.png";

/// Yellow — used as both primary AND secondary so all non-transparent pixels bake yellow.
const PATH_COLOR: Color = Color::srgb(1.0, 0.88, 0.0);

/// Marks entities that are part of the path overlay so they can be batch-despawned.
#[derive(Component)]
pub struct PathOverlayTile;

/// Current computed projectile path. Updated each frame when targeting is active.
#[derive(Resource, Default)]
pub struct RangedPathOverlay {
  /// Tiles from just after player to destination (inclusive), empty when not targeting.
  pub tiles: Vec<(i32, i32)>,
  /// True if the path was cut short by a wall before reaching the cursor.
  pub blocked: bool,
}

// ---------------------------------------------------------------------------
// Path math
// ---------------------------------------------------------------------------

/// All grid tiles on the Bresenham line from (x0,y0) to (x1,y1), inclusive of both ends.
pub fn bresenham_path(x0: i32, y0: i32, x1: i32, y1: i32) -> Vec<(i32, i32)> {
  let mut tiles = vec![(x0, y0)];
  if x0 == x1 && y0 == y1 { return tiles; }
  let dx = (x1 - x0).abs();
  let dy = -(y1 - y0).abs();
  let sx = if x0 < x1 { 1 } else { -1 };
  let sy = if y0 < y1 { 1 } else { -1 };
  let mut err = dx + dy;
  let mut x = x0;
  let mut y = y0;
  loop {
    if x == x1 && y == y1 { break; }
    let e2 = 2 * err;
    if e2 >= dy { err += dy; x += sx; }
    if e2 <= dx { err += dx; y += sy; }
    tiles.push((x, y));
  }
  tiles
}

/// Trace from (from_x, from_y) toward (to_x, to_y) and return the last walkable tile
/// before any wall. Returns `(from_x, from_y)` if the first step is already blocked.
pub fn ray_cast_target(
  from_x: i32,
  from_y: i32,
  to_x: i32,
  to_y: i32,
  level: &crate::level::Level
) -> (i32, i32) {
  bresenham_path(from_x, from_y, to_x, to_y)
    .into_iter()
    .skip(1)
    .take_while(|&(x, y)| level.walkable(x, y))
    .last()
    .unwrap_or((from_x, from_y))
}

// ---------------------------------------------------------------------------
// Sprite selection
// ---------------------------------------------------------------------------
//
// Grid directions (y increases downward):
//   N=(0,-1)  NE=(1,-1)  E=(1,0)  SE=(1,1)
//   S=(0,1)   SW=(-1,1)  W=(-1,0) NW=(-1,-1)
//
// Each path tile has two "arms" pointing toward its neighbors:
//   back_arm  = prev - current  (toward previous tile)
//   fwd_arm   = next - current  (toward next tile)
//
// Sprite images (20×20 black-on-transparent, palette-baked to yellow):
//   LINE_NS      |   arms N + S
//   LINE_CORNER  └   arms N + E
//   LINE_N_NE    ╲   arms N + NE  (45° bend, cardinal → adjacent diagonal)
//   LINE_N_SE    ⟋   arms N + SE  (135° bend, cardinal → far diagonal)
//   LINE_DIAG_NW ╲   pure NW–SE diagonal
//   LINE_DIAG_NE ╱   pure NE–SW diagonal
//
// Bevy rotation: Quat::from_rotation_z(θ) rotates CCW.
//   90° CCW maps: E→N, N→W, W→S, S→E, NE→NW, NW→SW, SW→SE, SE→NE
// Sprite flip_x mirrors the texture horizontally before the transform rotation.

/// Returns (sprite_path, rotation_radians, flip_x) for a path segment
/// connecting two arm directions.
///
/// `arm_a` and `arm_b` are grid-space direction vectors pointing from this
/// tile toward each neighbor (previous and next).
fn connection_sprite(arm_a: (i32, i32), arm_b: (i32, i32)) -> (&'static str, f32, bool) {
  // Normalize to canonical order so (arm_a, arm_b) and (arm_b, arm_a) hit same branch.
  let (a, b) = if arm_a <= arm_b { (arm_a, arm_b) } else { (arm_b, arm_a) };
  // Tuple order: (-1,-1)<(-1,0)<(-1,1)<(0,-1)<(0,1)<(1,-1)<(1,0)<(1,1)
  //              NW      W      SW     N      S     NE     E     SE
  match (a, b) {
    // --- Straight cardinal ---
    ((0, -1), (0, 1))   => (LINE_NS, 0.0, false),          // {N,S}
    ((-1, 0), (1, 0))   => (LINE_NS, FRAC_PI_2, false),    // {W,E}

    // --- Straight diagonal ---
    ((-1, -1), (1, 1))  => (LINE_DIAG_NW, 0.0, false),     // {NW,SE}
    ((-1, 1), (1, -1))  => (LINE_DIAG_NE, 0.0, false),     // {NE,SW}

    // --- 45° bends (N-NE family) ---
    ((0, -1), (1, -1))  => (LINE_N_NE, 0.0, false),        // {N,NE}
    ((-1, -1), (0, -1)) => (LINE_N_NE, 0.0, true),         // {NW,N} flip
    ((-1, -1), (-1, 0)) => (LINE_N_NE, FRAC_PI_2, false),  // {NW,W}
    ((-1, 0), (-1, 1))  => (LINE_N_NE, FRAC_PI_2, true),   // {W,SW} flip
    ((-1, 1), (0, 1))   => (LINE_N_NE, PI, false),          // {SW,S}
    ((0, 1), (1, 1))    => (LINE_N_NE, PI, true),           // {S,SE} flip
    ((1, 0), (1, 1))    => (LINE_N_NE, -FRAC_PI_2, false),  // {E,SE}
    ((1, -1), (1, 0))   => (LINE_N_NE, -FRAC_PI_2, true),  // {NE,E} flip

    // --- 135° bends (N-SE family) ---
    ((0, -1), (1, 1))   => (LINE_N_SE, 0.0, false),         // {N,SE}
    ((-1, 1), (0, -1))  => (LINE_N_SE, 0.0, true),          // {N,SW} flip
    ((-1, 0), (1, -1))  => (LINE_N_SE, FRAC_PI_2, false),   // {W,NE}
    ((-1, 0), (1, 1))   => (LINE_N_SE, FRAC_PI_2, true),    // {W,SE} flip
    ((-1, -1), (0, 1))  => (LINE_N_SE, PI, false),           // {NW,S}
    ((0, 1), (1, -1))   => (LINE_N_SE, PI, true),            // {S,NE} flip
    ((-1, 1), (1, 0))   => (LINE_N_SE, -FRAC_PI_2, false),  // {SW,E}
    ((-1, -1), (1, 0))  => (LINE_N_SE, -FRAC_PI_2, true),   // {NW,E} flip

    // Fallback (shouldn't happen in Bresenham paths)
    _ => (LINE_NS, 0.0, false),
  }
}

/// Returns (sprite_path, rotation, flip_x) for a single-arm endpoint or start tile.
/// Cardinal arms use `lines N` rotated; diagonal arms use `lines NE` rotated/flipped.
fn half_sprite(arm_dir: (i32, i32)) -> (&'static str, f32, bool) {
  match arm_dir {
    (0, -1)  => (LINE_HALF_N, 0.0, false),          // N
    (1, 0)   => (LINE_HALF_N, -FRAC_PI_2, false),   // E
    (0, 1)   => (LINE_HALF_N, PI, false),            // S
    (-1, 0)  => (LINE_HALF_N, FRAC_PI_2, false),    // W
    (1, -1)  => (LINE_HALF_NE, 0.0, false),          // NE
    (-1, -1) => (LINE_HALF_NE, 0.0, true),           // NW (flip)
    (1, 1)   => (LINE_HALF_NE, PI, true),             // SE (flip + 180°)
    (-1, 1)  => (LINE_HALF_NE, PI, false),            // SW (180°)
    _ => (LINE_HALF_N, 0.0, false),
  }
}

// ---------------------------------------------------------------------------
// Systems
// ---------------------------------------------------------------------------

/// Recomputes `RangedPathOverlay` each frame based on targeting state + cursor position.
/// Returns `None` when system queries fail (overlay unchanged); `Some(x)` to write `x`.
pub fn update_ranged_path(
  windows: Query<&Window>,
  camera_q: Query<(&Camera, &GlobalTransform), With<Camera2d>>,
  targeting: Res<TargetingState>,
  current: Res<CurrentZone>,
  player_q: Query<&PlayerPos, With<Player>>,
  mut overlay: ResMut<RangedPathOverlay>
) {
  // None = leave as-is (query failures); Some(x) = write x.
  let new = if targeting.selected.is_none() {
    Some(RangedPathOverlay::default())
  } else if let Ok(window) = windows.single()
    && let Ok((camera, cam_transform)) = camera_q.single()
    && let Ok(pos) = player_q.single()
  {
    let level = current.0.level(pos.z);
    // No cursor or outside game pane → clear; failed projection → leave as-is.
    let inner = if let Some(cursor) = window.cursor_position()
      && game_pane_rect(window).contains(cursor)
      && let Ok(world) = camera.viewport_to_world_2d(cam_transform, cursor)
    {
      let (tx, ty) = world_to_level_cell(world, level.width, level.height);
      let all_tiles = bresenham_path(pos.x, pos.y, tx, ty);
      let path_tiles = &all_tiles[1..];
      let block_idx = path_tiles.iter().position(|&(x, y)| {
        x < 0
          || y < 0
          || (x as usize) >= level.width
          || (y as usize) >= level.height
          || !level.walkable(x, y)
      });
      RangedPathOverlay {
        tiles: path_tiles[..block_idx.unwrap_or(path_tiles.len())].to_vec(),
        blocked: block_idx.is_some(),
      }
    } else {
      RangedPathOverlay::default()
    };
    Some(inner)
  } else {
    None
  };

  if let Some(new) = new
    && (overlay.tiles != new.tiles || overlay.blocked != new.blocked)
  {
    *overlay = new;
  }
}

fn spawn_path_tile(
  commands: &mut Commands,
  sprite_path: &'static str,
  rotation: f32,
  flip_x: bool,
  screen_pos: Vec3,
  palette_cache: &mut PaletteImageCache,
  images: &mut Assets<Image>
) {
  let img = palette_sprite_handle(
    sprite_path, PATH_COLOR, PATH_COLOR, palette_cache, images
  );
  commands.spawn((
    PathOverlayTile,
    Sprite {
      image: img,
      custom_size: Some(Vec2::splat(TILE_SIZE)),
      flip_x,
      color: Color::WHITE,
      ..default()
    },
    Transform::from_translation(screen_pos).with_rotation(Quat::from_rotation_z(rotation)),
    Visibility::Visible
  ));
}

/// Spawns/despawns path overlay tile entities whenever `RangedPathOverlay` changes.
pub fn render_ranged_path(
  overlay: Res<RangedPathOverlay>,
  pos: Single<&PlayerPos, With<Player>>,
  current: Res<CurrentZone>,
  existing: Query<Entity, With<PathOverlayTile>>,
  mut commands: Commands,
  mut palette_cache: ResMut<PaletteImageCache>,
  mut images: ResMut<Assets<Image>>
) {
  for entity in &existing {
    commands.entity(entity).despawn();
  }
  if overlay.tiles.is_empty() { return; }

  let w = current.0.width;
  let h = current.0.height;
  let last_i = overlay.tiles.len() - 1;

  // Start tile on the player position — half-line pointing toward first path tile
  let fwd_dir = (overlay.tiles[0].0 - pos.x, overlay.tiles[0].1 - pos.y);
  let (sp, rot, flip) = half_sprite(fwd_dir);
  let start_pos = tile_screen_pos(pos.x as f32, pos.y as f32, w, h)
    + Vec3::new(0.0, 0.0, 0.35);
  spawn_path_tile(
    &mut commands, sp, rot, flip, start_pos, &mut palette_cache, &mut images
  );

  for (i, &(tx, ty)) in overlay.tiles.iter().enumerate() {
    let prev = if i == 0 { (pos.x, pos.y) } else { overlay.tiles[i - 1] };
    let back_arm = (prev.0 - tx, prev.1 - ty);
    let screen_pos =
      tile_screen_pos(tx as f32, ty as f32, w, h) + Vec3::new(0.0, 0.0, 0.35);

    let (sp, rot, flip) = if i == last_i {
      half_sprite(back_arm)
    } else {
      let next = overlay.tiles[i + 1];
      let fwd_arm = (next.0 - tx, next.1 - ty);
      connection_sprite(back_arm, fwd_arm)
    };

    spawn_path_tile(
      &mut commands, sp, rot, flip, screen_pos, &mut palette_cache, &mut images
    );
  }
}
