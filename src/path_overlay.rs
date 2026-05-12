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

const LINE_NS:     &str = "textures/space_qud/lines N S.png";
const LINE_NE:     &str = "textures/space_qud/lines N NE.png";
const LINE_SE:     &str = "textures/space_qud/lines N SE.png";
const LINE_CORNER: &str = "textures/space_qud/lines N E.png";

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
  let path = bresenham_path(from_x, from_y, to_x, to_y);
  let mut last = (from_x, from_y);
  for &(x, y) in path.iter().skip(1) {
    if !level.walkable(x, y) { break; }
    last = (x, y);
  }
  last
}

// ---------------------------------------------------------------------------
// Sprite selection
// ---------------------------------------------------------------------------

/// Returns (sprite_path, z_rotation_radians) for a mid-path segment arriving
/// via `from_dir` (grid-space step dx,dy where y increases downward).
fn segment_sprite(from_dir: (i32, i32)) -> (&'static str, f32) {
  match from_dir {
    (0, _) => (LINE_NS, 0.0),               // N or S — vertical
    (_, 0) => (LINE_NS, FRAC_PI_2),          // E or W — rotate to horizontal
    (1, -1) | (-1, 1) => (LINE_NE, 0.0),    // NE or SW diagonal
    _ => (LINE_SE, 0.0),                     // SE or NW diagonal
  }
}

/// Returns (sprite_path, z_rotation_radians) for the final tile of the path.
///
/// Cardinal arrivals use the L-corner sprite rotated so the incoming arm faces
/// the direction of travel; the perpendicular arm visually caps the path.
/// Diagonal arrivals use the matching diagonal sprite.
fn endpoint_sprite(from_dir: (i32, i32)) -> (&'static str, f32) {
  match from_dir {
    // Original └: entry arm points S (down), cap arm points E (right).
    (0, -1) => (LINE_CORNER, 0.0),        // going N: entry from S ✓
    (1, 0)  => (LINE_CORNER, -FRAC_PI_2), // going E: rotate CW 90° → entry from W
    (0, 1)  => (LINE_CORNER, PI),         // going S: rotate 180° → entry from N
    (-1, 0) => (LINE_CORNER, FRAC_PI_2),  // going W: rotate CCW 90° → entry from E
    (1, -1) | (-1, 1) => (LINE_NE, 0.0),
    _ => (LINE_SE, 0.0),
  }
}

// ---------------------------------------------------------------------------
// Systems
// ---------------------------------------------------------------------------

/// Recomputes `RangedPathOverlay` each frame based on targeting state + cursor position.
pub fn update_ranged_path(
  windows: Query<&Window>,
  camera_q: Query<(&Camera, &GlobalTransform), With<Camera2d>>,
  targeting: Res<TargetingState>,
  current: Res<CurrentZone>,
  player_q: Query<&PlayerPos, With<Player>>,
  mut overlay: ResMut<RangedPathOverlay>
) {
  if targeting.selected.is_none() {
    if !overlay.tiles.is_empty() || overlay.blocked {
      *overlay = RangedPathOverlay::default();
    }
    return;
  }

  let Ok(window) = windows.single() else { return };
  let Ok((camera, cam_transform)) = camera_q.single() else { return };
  let Ok(pos) = player_q.single() else { return };
  let level = current.0.level(pos.z);

  let Some(cursor) = window.cursor_position() else {
    if !overlay.tiles.is_empty() { *overlay = RangedPathOverlay::default(); }
    return;
  };
  if !game_pane_rect(window).contains(cursor) {
    if !overlay.tiles.is_empty() { *overlay = RangedPathOverlay::default(); }
    return;
  }
  let Ok(world) = camera.viewport_to_world_2d(cam_transform, cursor) else { return };
  let (tx, ty) = world_to_level_cell(world, level.width, level.height);

  let all_tiles = bresenham_path(pos.x, pos.y, tx, ty);
  let path_tiles = &all_tiles[1..];

  let mut blocked = false;
  let mut end_idx = path_tiles.len();
  for (i, &(x, y)) in path_tiles.iter().enumerate() {
    if x < 0
      || y < 0
      || (x as usize) >= level.width
      || (y as usize) >= level.height
      || !level.walkable(x, y)
    {
      end_idx = i;
      blocked = true;
      break;
    }
  }

  let tiles: Vec<(i32, i32)> = path_tiles[..end_idx].to_vec();
  if overlay.tiles != tiles || overlay.blocked != blocked {
    overlay.tiles = tiles;
    overlay.blocked = blocked;
  }
}

/// Spawns/despawns path overlay tile entities whenever `RangedPathOverlay` changes.
pub fn render_ranged_path(
  overlay: Res<RangedPathOverlay>,
  player_q: Query<&PlayerPos, With<Player>>,
  current: Res<CurrentZone>,
  existing: Query<Entity, With<PathOverlayTile>>,
  mut commands: Commands,
  mut palette_cache: ResMut<PaletteImageCache>,
  mut images: ResMut<Assets<Image>>
) {
  for entity in existing.iter() {
    commands.entity(entity).despawn();
  }
  if overlay.tiles.is_empty() { return; }

  let Ok(pos) = player_q.single() else { return };
  let w = current.0.width;
  let h = current.0.height;
  let last_i = overlay.tiles.len() - 1;

  for (i, &(tx, ty)) in overlay.tiles.iter().enumerate() {
    let prev = if i == 0 { (pos.x, pos.y) } else { overlay.tiles[i - 1] };
    let from_dir = (tx - prev.0, ty - prev.1);
    let (sprite_path, rotation) = if i == last_i {
      endpoint_sprite(from_dir)
    } else {
      segment_sprite(from_dir)
    };

    let img = palette_sprite_handle(
      sprite_path, PATH_COLOR, PATH_COLOR, &mut palette_cache, &mut images
    );
    let screen_pos =
      tile_screen_pos(tx as f32, ty as f32, w, h) + Vec3::new(0.0, 0.0, 0.35);

    commands.spawn((
      PathOverlayTile,
      Sprite {
        image: img,
        custom_size: Some(Vec2::splat(TILE_SIZE)),
        color: Color::WHITE,
        ..default()
      },
      Transform::from_translation(screen_pos).with_rotation(Quat::from_rotation_z(rotation)),
      Visibility::Visible
    ));
  }
}
