//! Ranged attack trajectory overlay — shows a yellow path from the player to the
//! hovered tile when a ranged ability is selected, stopping at walls.

use bevy::prelude::*;
use std::f32::consts::{FRAC_PI_2, PI};
use crate::{
  CurrentZone, Player, PlayerPos, TILE_SIZE,
  abilities::{AbilityBarData, AbilityKind, TargetingState},
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
  /// For laser weapons: world-space start and end of the Euclidean aim line.
  /// When Some, `render_ranged_path` draws a straight line instead of tile sprites.
  pub laser_line: Option<(Vec3, Vec3)>,
}

// ---------------------------------------------------------------------------
// Path math
// ---------------------------------------------------------------------------

/// Enumerate every grid cell a line from (x0,y0) to (x1,y1) passes through,
/// using the Amanatides & Woo DDA algorithm. Both endpoint cells are included.
/// Coordinates are in continuous tile-space: center of cell (i,j) is (i+0.5, j+0.5).
pub fn dda_cells(x0: f32, y0: f32, x1: f32, y1: f32) -> Vec<(i32, i32)> {
  let mut cells = Vec::new();
  let mut cx = x0.floor() as i32;
  let mut cy = y0.floor() as i32;
  let end_cx = x1.floor() as i32;
  let end_cy = y1.floor() as i32;
  cells.push((cx, cy));
  if cx == end_cx && cy == end_cy { return cells; }

  let dx = x1 - x0;
  let dy = y1 - y0;
  let step_x = dx.signum() as i32;
  let step_y = dy.signum() as i32;
  let t_delta_x = if dx.abs() < 1e-9 { f32::INFINITY } else { 1.0 / dx.abs() };
  let t_delta_y = if dy.abs() < 1e-9 { f32::INFINITY } else { 1.0 / dy.abs() };
  let mut t_max_x = if dx > 1e-9 {
    ((cx + 1) as f32 - x0) / dx
  } else if dx < -1e-9 {
    (cx as f32 - x0) / dx
  } else {
    f32::INFINITY
  };
  let mut t_max_y = if dy > 1e-9 {
    ((cy + 1) as f32 - y0) / dy
  } else if dy < -1e-9 {
    (cy as f32 - y0) / dy
  } else {
    f32::INFINITY
  };

  loop {
    if t_max_x < t_max_y { cx += step_x; t_max_x += t_delta_x; }
    else                  { cy += step_y; t_max_y += t_delta_y; }
    cells.push((cx, cy));
    if cx == end_cx && cy == end_cy { break; }
  }
  cells
}

/// Find a point on tile (tx, ty) that has a clear Euclidean line-of-sight from (px, py)
/// (continuous tile-space coordinates). Tries center, corners, and edge midpoints in that order.
/// All cells between start and end (exclusive of the player cell, the target tile is allowed)
/// must be walkable. Returns `None` if no point on the tile is visible.
pub fn euclidean_los_point(
  px: f32,
  py: f32,
  tx: i32,
  ty: i32,
  level: &crate::level::Level
) -> Option<(f32, f32)> {
  let candidates = [
    (tx as f32 + 0.5, ty as f32 + 0.5),  // center first
    (tx as f32 + 0.1, ty as f32 + 0.1),
    (tx as f32 + 0.9, ty as f32 + 0.1),
    (tx as f32 + 0.1, ty as f32 + 0.9),
    (tx as f32 + 0.9, ty as f32 + 0.9),
    (tx as f32 + 0.5, ty as f32 + 0.1),
    (tx as f32 + 0.5, ty as f32 + 0.9),
    (tx as f32 + 0.1, ty as f32 + 0.5),
    (tx as f32 + 0.9, ty as f32 + 0.5),
  ];
  candidates.into_iter().find(|&(cx, cy)| {
    dda_cells(px, py, cx, cy)
      .into_iter()
      .skip(1)  // skip player's cell
      .all(|(gx, gy)| (gx, gy) == (tx, ty) || level.walkable(gx, gy))
  })
}

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
  camera_q: Query<(&Camera, &GlobalTransform), With<crate::post_process::GameCamera>>,
  targeting: Res<TargetingState>,
  bar: Res<AbilityBarData>,
  current: Res<CurrentZone>,
  player_q: Query<&PlayerPos, With<Player>>,
  mut overlay: ResMut<RangedPathOverlay>
) {
  let new = if targeting.selected.is_none() {
    Some(RangedPathOverlay::default())
  } else if let Ok(window) = windows.single()
    && let Ok((camera, cam_transform)) = camera_q.single()
    && let Ok(pos) = player_q.single()
  {
    let is_laser = targeting.selected
      .and_then(|i| bar.slots.get(i))
      .is_some_and(|s| s.kind == AbilityKind::FireLaser);

    let level = current.0.level(pos.z);
    let w = level.width;
    let h = level.height;

    let inner = if let Some(cursor) = window.cursor_position()
      && game_pane_rect(window).contains(cursor)
      && let Ok(world) = camera.viewport_to_world_2d(cam_transform, cursor)
    {
      let (tx, ty) = world_to_level_cell(world, w, h);

      if is_laser {
        let px = pos.x as f32 + 0.5;
        let py = pos.y as f32 + 0.5;
        // Tile-space → world-space for overlay z-layer
        let tile_world = |x: f32, y: f32| Vec3::new(
          (x - 0.5 - w as f32 / 2.0) * TILE_SIZE,
          (h as f32 / 2.0 - y + 0.5) * TILE_SIZE,
          0.35,
        );
        let laser_line = euclidean_los_point(px, py, tx, ty, level)
          .map(|(lx, ly)| (tile_world(px, py), tile_world(lx, ly)));
        RangedPathOverlay { laser_line, ..default() }
      } else {
        let all_tiles = bresenham_path(pos.x, pos.y, tx, ty);
        let path_tiles = &all_tiles[1..];
        let block_idx = path_tiles.iter().position(|&(x, y)| {
          x < 0
            || y < 0
            || (x as usize) >= w
            || (y as usize) >= h
            || !level.walkable(x, y)
        });
        RangedPathOverlay {
          tiles: path_tiles[..block_idx.unwrap_or(path_tiles.len())].to_vec(),
          blocked: block_idx.is_some(),
          laser_line: None,
        }
      }
    } else {
      RangedPathOverlay::default()
    };
    Some(inner)
  } else {
    None
  };

  if let Some(new) = new
    && (overlay.tiles != new.tiles || overlay.blocked != new.blocked
        || overlay.laser_line != new.laser_line)
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

const LASER_LINE_WIDTH: f32 = TILE_SIZE * 0.12;
const LASER_LINE_COLOR: Color = Color::srgb(0.0, 0.88, 1.0);

/// Spawns/despawns path overlay tile entities whenever `RangedPathOverlay` changes.
pub fn render_ranged_path(
  overlay: Res<RangedPathOverlay>,
  pos: Single<&PlayerPos, With<Player>>,
  current: Res<CurrentZone>,
  existing: Query<Entity, With<PathOverlayTile>>,
  mut commands: Commands,
  mut meshes: ResMut<Assets<Mesh>>,
  mut materials: ResMut<Assets<ColorMaterial>>,
  mut palette_cache: ResMut<PaletteImageCache>,
  mut images: ResMut<Assets<Image>>
) {
  for entity in &existing {
    commands.entity(entity).despawn();
  }

  // Laser targeting: draw a single straight Euclidean line.
  if let Some((start, end)) = overlay.laser_line {
    let diff = end - start;
    let length = diff.truncate().length();
    if length > 0.1 {
      let angle = diff.y.atan2(diff.x);
      let mid = (start + end) * 0.5;
      commands.spawn((
        PathOverlayTile,
        Mesh2d(meshes.add(Rectangle::new(length, LASER_LINE_WIDTH))),
        MeshMaterial2d(materials.add(ColorMaterial::from_color(LASER_LINE_COLOR))),
        Transform::from_translation(mid.with_z(0.35))
          .with_rotation(Quat::from_rotation_z(angle)),
      ));
    }
    return;
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
