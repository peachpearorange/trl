#![feature(const_trait_impl)]
#![feature(const_precise_live_drops)]
#[path = "sprites.rs"]
#[allow(dead_code)]
mod sprites;
#[path = "tiles.rs"]
pub mod tiles;
#[path = "utils.rs"]
mod utils;
#[path = "faction.rs"]
mod faction;
#[path = "level.rs"]
mod level;
#[path = "entities.rs"]
mod entities;

pub const SPRITE_TEXELS: f32 = 20.0;

use {bevy::{input::{keyboard::{Key, KeyboardInput}, mouse::AccumulatedMouseScroll},
            prelude::*,
            sprite_render::{AlphaMode2d, TileData, TilemapChunk, TilemapChunkTileData}},
     enum_assoc::Assoc,
     grid_2d::Grid,
     haalka::prelude::*,
     num_enum::TryFromPrimitive,
     std::{collections::{HashMap, VecDeque},
           num::NonZeroU32,
           path::{Path, PathBuf},
           time::{SystemTime, UNIX_EPOCH}},
     tiles::{Tile, TileRenderMode},
     wfc::{RunOwn, Wave,
           overlapping::OverlappingPatterns,
           retry::{NumTimes, RetryOwn}}};

const CELL: f32 = 20.0;
const STEP: f32 = CELL;
const INITIAL_CANVAS_W: usize = 40;
const INITIAL_CANVAS_H: usize = 40;
const CANVAS_ORIGIN_X: f32 = -(INITIAL_CANVAS_W as f32 * STEP) / 2.0;
const CANVAS_ORIGIN_Y: f32 = (INITIAL_CANVAS_H as f32 * STEP) / 2.0;
const PALETTE_COLS: usize = 4;
const PAL_CELL: f32 = 24.0;
const SAVE_DIR: &str = "editor_saves";
const EDGE_BUTTON_SIZE: f32 = 18.0;
const RESIZE_HOLD_INITIAL_DELAY: f32 = 0.25;
const RESIZE_HOLD_REPEAT: f32 = 0.05;
const DEFAULT_PATTERN_SIZE: u32 = 5;

#[derive(Assoc, Clone, Copy, PartialEq, Eq, Debug, TryFromPrimitive)]
#[repr(u8)]
#[func(pub fn label(&self) -> &'static str)]
#[func(pub fn object(&self) -> entities::Object)]
pub enum ObjectTemplate {
  #[assoc(label = "tree", object = entities::Object::TREE)]
  Tree,
  #[assoc(label = "boulder", object = entities::Object::BOULDER)]
  Boulder,
  #[assoc(label = "door", object = entities::Object::DOOR)]
  Door,
  #[assoc(label = "airlock_door", object = entities::Object::AIRLOCK_DOOR)]
  AirlockDoor,
  #[assoc(label = "bed", object = entities::Object::BED)]
  Bed,
  #[assoc(label = "table", object = entities::Object::TABLE)]
  Table,
  #[assoc(label = "chair", object = entities::Object::CHAIR)]
  Chair,
  #[assoc(label = "crafting_table", object = entities::Object::CRAFTING_TABLE)]
  CraftingTable,
  #[assoc(label = "locker", object = entities::Object::LOCKER)]
  Locker,
  #[assoc(label = "crate", object = entities::Object::CRATE_OBJ)]
  Crate,
  #[assoc(label = "loot_chest", object = entities::Object::LOOT_CHEST)]
  LootChest,
  #[assoc(label = "flight_console", object = entities::Object::FLIGHT_CONSOLE)]
  FlightConsole,
  #[assoc(label = "loadout_console", object = entities::Object::LOADOUT_CONSOLE)]
  LoadoutConsole,
  #[assoc(label = "space_cat", object = entities::Object::SPACE_CAT)]
  SpaceCat,
  #[assoc(label = "thruster", object = entities::Object::THRUSTER)]
  Thruster,
  #[assoc(label = "rat_soldier", object = entities::Object::RAT_SOLDIER)]
  RatSoldier,
  #[assoc(label = "armored_rat_soldier", object = entities::Object::ARMORED_RAT_SOLDIER)]
  ArmoredRatSoldier,
  #[assoc(label = "robot", object = entities::Object::ROBOT)]
  Robot,
  #[assoc(label = "wack_robot", object = entities::Object::WACK_ROBOT)]
  WackRobot,
  #[assoc(label = "alien_runner", object = entities::Object::ALIEN_RUNNER)]
  AlienRunner,
  #[assoc(label = "lava_crab", object = entities::Object::LAVA_CRAB)]
  LavaCrab,
  #[assoc(label = "mantis_alien", object = entities::Object::MANTIS_ALIEN)]
  MantisAlien,
  #[assoc(label = "crab_alien", object = entities::Object::CRAB_ALIEN)]
  CrabAlien,
  #[assoc(label = "mushroom_creature", object = entities::Object::MUSHROOM_CREATURE)]
  MushroomCreature,
  #[assoc(label = "grenade_thrower", object = entities::Object::GRENADE_THROWER)]
  GrenadeThrower,
  #[assoc(label = "gunman", object = entities::Object::GUNMAN)]
  Gunman,
  #[assoc(label = "laser_sword", object = entities::Object::LASER_SWORD)]
  LaserSword,
  #[assoc(label = "robot_dog", object = entities::Object::ROBOT_DOG)]
  RobotDog,
  #[assoc(label = "turret", object = entities::Object::TURRET)]
  Turret,
}

impl ObjectTemplate {
  pub fn all() -> impl Iterator<Item = ObjectTemplate> {
    (0u8..).map_while(|i| ObjectTemplate::try_from(i).ok())
  }

  fn from_save(s: &str) -> Option<Self> {
    Self::all().find(|t| format!("{t:?}") == s)
  }

  pub fn sprite_spec(self) -> (&'static str, [f32; 3], [f32; 3]) {
    let obj = self.object();
    let glyph = entities::Has::<entities::Glyph>::get(&obj).expect("ObjectTemplate missing Glyph");
    let path = glyph.texture.expect("ObjectTemplate missing texture");
    let (pri, sec) = glyph.sprite_palette.expect("ObjectTemplate missing palette");
    let to_arr = |c: Color| { let s = c.to_srgba(); [s.red, s.green, s.blue] };
    (path, to_arr(pri), to_arr(sec))
  }
}

fn to_color(c: [f32; 3]) -> Color { Color::srgb(c[0], c[1], c[2]) }

fn tile_color(t: Tile) -> Color { to_color(t.color()) }

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq)]
enum ToolMode {
  Draw,
  Bucket,
  RectOutline,
  RectFill,
  Copy,
  Move,
  Paste
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ClipboardMode {
  Copy,
  Move
}

#[derive(Clone, Copy)]
struct ClipboardSource {
  x1: i32,
  y1: i32,
  x2: i32,
  y2: i32
}

impl ClipboardSource {
  fn from_points(a: (i32, i32), b: (i32, i32)) -> Self {
    Self { x1: a.0.min(b.0), y1: a.1.min(b.1), x2: a.0.max(b.0), y2: a.1.max(b.1) }
  }

  fn offset_from(self, point: (i32, i32)) -> Option<(i32, i32)> {
    (self.x1 <= point.0 && point.0 <= self.x2 && self.y1 <= point.1 && point.1 <= self.y2)
      .then_some((point.0 - self.x1, point.1 - self.y1))
  }

  fn top_left(self) -> (i32, i32) { (self.x1, self.y1) }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum EdgeSide {
  Left,
  Right,
  Top,
  Bottom
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum EdgeAction {
  Expand,
  Contract
}

impl ToolMode {
  fn name(self) -> &'static str {
    match self {
      ToolMode::Draw => "Draw",
      ToolMode::Bucket => "Bucket",
      ToolMode::RectOutline => "Rect",
      ToolMode::RectFill => "Fill",
      ToolMode::Copy => "Copy",
      ToolMode::Move => "Move",
      ToolMode::Paste => "Paste"
    }
  }
}

#[derive(Clone)]
struct Clipboard {
  tiles: Vec<Vec<Tile>>,
  objects: Vec<Vec<Option<ObjectTemplate>>>,
  markers: Vec<Vec<Option<String>>>,
  source: ClipboardSource,
  mode: ClipboardMode
}

impl Clipboard {
  fn width(&self) -> usize { self.tiles.first().map(Vec::len).unwrap_or(0) }

  fn height(&self) -> usize { self.tiles.len() }
}

// ---------------------------------------------------------------------------
// Resources
// ---------------------------------------------------------------------------

#[derive(Resource)]
struct EditorCanvas {
  tiles: Vec<Vec<Tile>>,
  objects: Vec<Vec<Option<ObjectTemplate>>>,
  markers: Vec<Vec<Option<String>>>
}

#[derive(Resource)]
struct EditorState {
  tool: ToolMode,
  selected_tile: Tile,
  selected_object: Option<ObjectTemplate>,
  drag_start: Option<(i32, i32)>,
  paste_drag_offset: Option<(i32, i32)>,
  clipboard: Option<Clipboard>,
  pattern_size: u32,
  output_mult: u32
}

#[derive(Resource)]
struct CameraZoom(f32);

#[derive(Resource)]
struct PanState {
  active: bool,
  cursor_origin: Vec2,
  camera_origin: Vec3
}

#[derive(Resource, Clone)]
struct TileImageCache(Vec<(Handle<Image>, Color)>);

#[derive(Clone)]
struct ObjectVisualInfo {
  image: Option<Handle<Image>>,
  text: String,
  text_color: Color
}

#[derive(Resource)]
struct ObjectVisualCache(Vec<ObjectVisualInfo>);

#[derive(Resource)]
struct EditorTileset(sprites::TilesetInfo);

#[derive(Resource)]
struct UndoStack(Vec<(Vec<Vec<Tile>>, Vec<Vec<Option<ObjectTemplate>>>, Vec<Vec<Option<String>>>, CanvasGridOrigin)>);

#[derive(Resource, Clone, Copy, PartialEq, Eq)]
struct CanvasGridOrigin {
  x: i32,
  y: i32
}

#[derive(Resource)]
struct SpawnedCanvasSize {
  width: usize,
  height: usize,
  origin_x: i32,
  origin_y: i32
}

// ---------------------------------------------------------------------------
// Components
// ---------------------------------------------------------------------------

#[derive(Component)]
struct CanvasCell(usize, usize);

#[derive(Component)]
struct CanvasObjectSprite(usize, usize);

#[derive(Component)]
struct CanvasObjectText(usize, usize);

#[derive(Component)]
struct OutputChunk;

#[derive(Component)]
struct DragPreview;

#[derive(Component)]
struct TilePaletteBtn(Tile);

#[derive(Component)]
struct ObjectPaletteBtn(Option<ObjectTemplate>);

#[derive(Component)]
struct ControlsLabel;

#[derive(Component)]
struct TilePreviewImage;

#[derive(Component)]
struct TilePreviewText;

#[derive(Component)]
struct TilePreviewPopup;

#[derive(Component)]
struct ModeBarBtn(ToolMode);

#[derive(Component)]
struct ModeBarLabel;

#[derive(Component, Clone, Copy, PartialEq, Eq)]
struct EdgeResizeButton {
  side: EdgeSide,
  action: EdgeAction
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SaveUiAction {
  SaveNow,
  ToggleLoadPicker
}

#[derive(Component)]
struct SaveUiButton(SaveUiAction);

#[derive(Component)]
struct LoadPickerPanel;

#[derive(Component)]
struct LoadPickerList;

#[derive(Component)]
struct LoadPickerListItem;

#[derive(Component)]
struct LoadPickerFileButton(String);

#[derive(Resource)]
struct ResizeHoldState {
  active: Option<EdgeResizeButton>,
  held_for: f32,
  repeat_accum: f32
}

#[derive(Resource)]
struct LoadPickerState {
  open: bool,
  refresh_requested: bool
}

#[derive(Resource)]
struct SaveNameInput {
  text: String,
  focused: bool
}

#[derive(Component)]
struct SaveNameInputField;

#[derive(Resource)]
struct MarkerInput {
  text: String,
  focused: bool
}

#[derive(Component)]
struct MarkerInputField;

#[derive(Component)]
struct MarkerListPanel;

#[derive(Component)]
struct MarkerListContent;

#[derive(Component)]
struct MarkerListButton(usize, usize);

#[derive(Component)]
struct CanvasMarkerText(usize, usize);

#[derive(Resource)]
struct MarkerListState {
  open: bool,
  needs_refresh: bool
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn canvas_origin() -> (f32, f32) { (CANVAS_ORIGIN_X, CANVAS_ORIGIN_Y) }

fn abbreviate_marker(name: &str) -> String {
  name.chars().filter(|c| !matches!(c, 'a' | 'e' | 'i' | 'o' | 'u' | 'A' | 'E' | 'I' | 'O' | 'U')).collect()
}

fn marker_color(name: &str) -> Color {
  let h = name.bytes().fold(0u32, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u32));
  let hue = (h % 360) as f32;
  let (s, l): (f32, f32) = (0.75, 0.65);
  let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
  let x = c * (1.0 - ((hue / 60.0) % 2.0 - 1.0).abs());
  let m = l - c / 2.0;
  let (r, g, b) = match hue as u32 / 60 {
    0 => (c, x, 0.0),
    1 => (x, c, 0.0),
    2 => (0.0, c, x),
    3 => (0.0, x, c),
    4 => (x, 0.0, c),
    _ => (c, 0.0, x),
  };
  Color::srgb(r + m, g + m, b + m)
}

impl EditorCanvas {
  fn width(&self) -> usize { self.tiles.first().map(Vec::len).unwrap_or(0) }

  fn height(&self) -> usize { self.tiles.len() }

  fn ensure_size(&mut self, min_width: usize, min_height: usize) {
    let width = self.width();
    let height = self.height();
    let target_width = width.max(min_width).max(1);
    let target_height = height.max(min_height).max(1);

    if target_width > width {
      for row in &mut self.tiles {
        row.resize(target_width, Tile::Grass);
      }
      for row in &mut self.objects {
        row.resize(target_width, None);
      }
      for row in &mut self.markers {
        row.resize(target_width, None);
      }
    }

    if target_height > height {
      self.tiles.resize_with(target_height, || vec![Tile::Grass; target_width]);
      self.objects.resize_with(target_height, || vec![None; target_width]);
      self.markers.resize_with(target_height, || vec![None; target_width]);
    }
  }

  fn resize_exact(&mut self, width: usize, height: usize) {
    let target_width = width.max(1);
    let target_height = height.max(1);
    self.tiles.resize_with(target_height, Vec::new);
    self.objects.resize_with(target_height, Vec::new);
    self.markers.resize_with(target_height, Vec::new);
    for row in &mut self.tiles {
      row.resize(target_width, Tile::Grass);
    }
    for row in &mut self.objects {
      row.resize(target_width, None);
    }
    for row in &mut self.markers {
      row.resize(target_width, None);
    }
  }

  fn resize_edge(
    &mut self,
    side: EdgeSide,
    action: EdgeAction,
    origin: &mut CanvasGridOrigin
  ) {
    let width = self.width();
    let height = self.height();
    let can_contract_x = width > 1;
    let can_contract_y = height > 1;

    match (side, action) {
      (EdgeSide::Left, EdgeAction::Expand) => {
        for row in &mut self.tiles {
          row.insert(0, Tile::Grass);
        }
        for row in &mut self.objects {
          row.insert(0, None);
        }
        for row in &mut self.markers {
          row.insert(0, None);
        }
        origin.x -= 1;
      }
      (EdgeSide::Right, EdgeAction::Expand) => {
        for row in &mut self.tiles {
          row.push(Tile::Grass);
        }
        for row in &mut self.objects {
          row.push(None);
        }
        for row in &mut self.markers {
          row.push(None);
        }
      }
      (EdgeSide::Top, EdgeAction::Expand) => {
        self.tiles.insert(0, vec![Tile::Grass; width]);
        self.objects.insert(0, vec![None; width]);
        self.markers.insert(0, vec![None; width]);
        origin.y -= 1;
      }
      (EdgeSide::Bottom, EdgeAction::Expand) => {
        self.tiles.push(vec![Tile::Grass; width]);
        self.objects.push(vec![None; width]);
        self.markers.push(vec![None; width]);
      }
      (EdgeSide::Left, EdgeAction::Contract) if can_contract_x => {
        for row in &mut self.tiles {
          row.remove(0);
        }
        for row in &mut self.objects {
          row.remove(0);
        }
        for row in &mut self.markers {
          row.remove(0);
        }
        origin.x += 1;
      }
      (EdgeSide::Right, EdgeAction::Contract) if can_contract_x => {
        for row in &mut self.tiles {
          row.pop();
        }
        for row in &mut self.objects {
          row.pop();
        }
        for row in &mut self.markers {
          row.pop();
        }
      }
      (EdgeSide::Top, EdgeAction::Contract) if can_contract_y => {
        self.tiles.remove(0);
        self.objects.remove(0);
        self.markers.remove(0);
        origin.y += 1;
      }
      (EdgeSide::Bottom, EdgeAction::Contract) if can_contract_y => {
        self.tiles.pop();
        self.objects.pop();
        self.markers.pop();
      }
      _ => {}
    }
  }
}

fn world_to_grid_unbounded(cursor: Vec2) -> (i32, i32) {
  let (ox, oy) = canvas_origin();
  let gx = ((cursor.x - ox + CELL / 2.0) / STEP).floor() as i32;
  let gy = ((oy - cursor.y + CELL / 2.0) / STEP).floor() as i32;
  (gx, gy)
}

fn world_to_grid(
  cursor: Vec2,
  canvas: &EditorCanvas,
  origin: CanvasGridOrigin
) -> Option<(usize, usize)> {
  let (gx, gy) = world_to_grid_unbounded(cursor);
  let width = canvas.width();
  let height = canvas.height();
  if gx >= origin.x
    && gy >= origin.y
    && gx < origin.x + width as i32
    && gy < origin.y + height as i32
  {
    Some(((gx - origin.x) as usize, (gy - origin.y) as usize))
  } else {
    None
  }
}

fn grid_to_world(gx: usize, gy: usize, origin: CanvasGridOrigin) -> Vec2 {
  let (ox, oy) = canvas_origin();
  let grid_x = origin.x + gx as i32;
  let grid_y = origin.y + gy as i32;
  Vec2::new(ox + grid_x as f32 * STEP, oy - grid_y as f32 * STEP)
}

fn grid_coord_to_world(gx: i32, gy: i32) -> Vec2 {
  let (ox, oy) = canvas_origin();
  Vec2::new(ox + gx as f32 * STEP, oy - gy as f32 * STEP)
}

fn grid_to_index(gx: i32, gy: i32, origin: CanvasGridOrigin) -> (usize, usize) {
  ((gx - origin.x) as usize, (gy - origin.y) as usize)
}

fn grid_coord_to_canvas_index(
  gx: i32,
  gy: i32,
  origin: CanvasGridOrigin,
  width: usize,
  height: usize
) -> Option<(usize, usize)> {
  let ix = gx - origin.x;
  let iy = gy - origin.y;
  if ix >= 0 && iy >= 0 && (ix as usize) < width && (iy as usize) < height {
    Some((ix as usize, iy as usize))
  } else {
    None
  }
}

fn edge_button_label(side: EdgeSide, action: EdgeAction) -> &'static str {
  match (side, action) {
    (EdgeSide::Left, EdgeAction::Expand) => "<",
    (EdgeSide::Left, EdgeAction::Contract) => ">",
    (EdgeSide::Right, EdgeAction::Expand) => ">",
    (EdgeSide::Right, EdgeAction::Contract) => "<",
    (EdgeSide::Top, EdgeAction::Expand) => "^",
    (EdgeSide::Top, EdgeAction::Contract) => "v",
    (EdgeSide::Bottom, EdgeAction::Expand) => "v",
    (EdgeSide::Bottom, EdgeAction::Contract) => "^"
  }
}

fn cursor_world(
  windows: &Query<&Window>,
  camera_q: &Query<(&Camera, &GlobalTransform)>
) -> Option<Vec2> {
  let window = windows.single().ok()?;
  let (camera, cam_tf) = camera_q.single().ok()?;
  window.cursor_position().and_then(|p| camera.viewport_to_world_2d(cam_tf, p).ok())
}

fn selection_rect(a: (i32, i32), b: (i32, i32)) -> (i32, i32, i32, i32) {
  (a.0.min(b.0), a.1.min(b.1), a.0.max(b.0), a.1.max(b.1))
}

fn push_undo(canvas: &EditorCanvas, origin: CanvasGridOrigin, undo: &mut UndoStack) {
  undo.0.push((canvas.tiles.clone(), canvas.objects.clone(), canvas.markers.clone(), origin));
  if undo.0.len() > 50 {
    undo.0.remove(0);
  }
}

fn flood_fill_same_tile_type(
  canvas: &mut EditorCanvas,
  sx: usize,
  sy: usize,
  target: Tile,
  paint_tile: Tile,
  paint_obj: Option<ObjectTemplate>,
  paint_marker: Option<String>
) {
  if target != paint_tile {
    let width = canvas.width();
    let height = canvas.height();
    let mut q = VecDeque::new();
    let mut seen = vec![vec![false; width]; height];
    seen[sy][sx] = true;
    q.push_back((sx, sy));
    while let Some((x, y)) = q.pop_front() {
      if canvas.tiles[y][x] == target {
        canvas.tiles[y][x] = paint_tile;
        canvas.objects[y][x] = paint_obj;
        canvas.markers[y][x] = paint_marker.clone();
        let mut consider = |nx: usize, ny: usize| {
          if canvas.tiles[ny][nx] == target && !seen[ny][nx] {
            seen[ny][nx] = true;
            q.push_back((nx, ny));
          }
        };
        if x > 0 {
          consider(x - 1, y);
        }
        if x + 1 < width {
          consider(x + 1, y);
        }
        if y > 0 {
          consider(x, y - 1);
        }
        if y + 1 < height {
          consider(x, y + 1);
        }
      }
    }
  }
}

fn build_tile_cache(
  palette_cache: &mut sprites::PaletteImageCache,
  images: &mut Assets<Image>
) -> TileImageCache {
  let entries = utils::mapv(
    |tile| {
      let extract = |rm: TileRenderMode| -> Option<(&'static str, [f32; 3], [f32; 3])> {
        match rm {
          TileRenderMode::SolidColor => None,
          TileRenderMode::Sprite(p, a, b) => Some((p, a, b)),
          TileRenderMode::SpritePackRandom(ps, a, b) => Some((ps[0], a, b)),
          TileRenderMode::ConnectedSprite(ps, a, b) => Some((ps[0], a, b)),
          TileRenderMode::ConnectedBorder(p, a, b) => Some((p, a, b))
        }
      };
      let entry = extract(tile.render_mode())
        .map(|(path, pri, sec)| {
          let h = sprites::palette_sprite_handle(
            path,
            to_color(pri),
            to_color(sec),
            palette_cache,
            images
          );
          (h, Color::WHITE)
        })
        .unwrap_or_else(|| (Handle::default(), tile_color(tile)));
      entry
    },
    Tile::all()
  );
  TileImageCache(entries)
}

fn build_object_visual_cache(
  palette_cache: &mut sprites::PaletteImageCache,
  images: &mut Assets<Image>
) -> ObjectVisualCache {
  let entries = utils::mapv(
    |tmpl| {
      let (path, primary, secondary) = tmpl.sprite_spec();
      let image = Some(sprites::palette_sprite_handle(
        path,
        to_color(primary),
        to_color(secondary),
        palette_cache,
        images
      ));
      ObjectVisualInfo {
        image,
        text: tmpl.label().chars().take(3).collect(),
        text_color: Color::srgb(1.0, 0.8, 0.2)
      }
    },
    ObjectTemplate::all()
  );
  ObjectVisualCache(entries)
}

// ---------------------------------------------------------------------------
// Setup
// ---------------------------------------------------------------------------

fn spawn_canvas_cells(
  commands: &mut Commands,
  canvas: &EditorCanvas,
  tile_cache: &TileImageCache,
  object_visuals: &ObjectVisualCache,
  origin: CanvasGridOrigin,
  x_start: usize,
  x_end: usize,
  y_start: usize,
  y_end: usize
) {
  let default_object_image =
    object_visuals.0.iter().find_map(|visual| visual.image.clone()).unwrap_or_default();
  let default_text_color = object_visuals
    .0
    .first()
    .map(|visual| visual.text_color)
    .unwrap_or(Color::srgb(1.0, 0.8, 0.2));
  for y in y_start..y_end {
    for x in x_start..x_end {
      let w = grid_to_world(x, y, origin);
      let tile =
        canvas.tiles.get(y).and_then(|row| row.get(x)).copied().unwrap_or(Tile::Grass);
      let object_visual = canvas
        .objects
        .get(y)
        .and_then(|row| row.get(x))
        .copied()
        .flatten()
        .and_then(|tmpl| object_visuals.0.get(tmpl as u8 as usize));
      let (ref img, color) = tile_cache.0[tile as u16 as usize];
      let object_image = object_visual
        .and_then(|visual| visual.image.clone())
        .unwrap_or_else(|| default_object_image.clone());
      let object_sprite_visible =
        object_visual.and_then(|visual| visual.image.as_ref()).is_some();
      let object_text = object_visual
        .filter(|visual| visual.image.is_none())
        .map(|visual| visual.text.clone())
        .unwrap_or_default();
      let object_text_color = object_visual
        .filter(|visual| visual.image.is_none())
        .map(|visual| visual.text_color)
        .unwrap_or(default_text_color);
      let object_text_visible = !object_text.is_empty();
      let marker_name = canvas
        .markers
        .get(y)
        .and_then(|row| row.get(x))
        .and_then(|m| m.as_deref());
      let marker_abbrev = marker_name.map(abbreviate_marker).unwrap_or_default();
      let marker_col = marker_name.map(marker_color).unwrap_or(Color::srgb(0.3, 0.9, 1.0));
      let has_marker = marker_name.is_some();
      commands
        .spawn((
          Sprite {
            image: img.clone(),
            color,
            custom_size: Some(Vec2::splat(CELL)),
            ..default()
          },
          Transform::from_xyz(w.x, w.y, 0.0),
          CanvasCell(x, y)
        ))
        .with_children(|parent| {
          parent.spawn((
            Sprite {
              image: object_image,
              color: Color::WHITE,
              custom_size: Some(Vec2::splat(CELL)),
              ..default()
            },
            Transform::from_xyz(0.0, 0.0, 0.5),
            if object_sprite_visible { Visibility::Visible } else { Visibility::Hidden },
            CanvasObjectSprite(x, y)
          ));
          parent.spawn((
            Text2d::new(object_text),
            TextFont { font_size: 10.0, ..default() },
            TextColor(object_text_color),
            Transform::from_xyz(0.0, 0.0, 0.5),
            if object_text_visible { Visibility::Visible } else { Visibility::Hidden },
            CanvasObjectText(x, y)
          ));
          parent.spawn((
            Text2d::new(marker_abbrev),
            TextFont { font_size: 8.0, ..default() },
            TextColor(marker_col),
            Transform::from_xyz(0.0, -4.0, 1.0),
            if has_marker { Visibility::Visible } else { Visibility::Hidden },
            CanvasMarkerText(x, y)
          ));
        });
    }
  }
}

fn static_text(t: impl Into<String>, size: f32, color: Color) -> El<Text> {
  El::<Text>::new()
    .text(Text::new(t))
    .text_font(TextFont { font_size: size, ..default() })
    .text_color(TextColor(color))
}

fn edge_btn(side: EdgeSide, action: EdgeAction) -> El<Node> {
  let color = if action == EdgeAction::Expand {
    Color::srgba(0.15, 0.22, 0.16, 0.95)
  } else {
    Color::srgba(0.22, 0.15, 0.15, 0.95)
  };
  El::<Node>::new()
    .with_node(|mut n| {
      n.width = Val::Px(EDGE_BUTTON_SIZE);
      n.height = Val::Px(EDGE_BUTTON_SIZE);
      n.justify_content = JustifyContent::Center;
      n.align_items = AlignItems::Center;
    })
    .background_color(BackgroundColor(color))
    .insert(Button)
    .insert(EdgeResizeButton { side, action })
    .child(static_text(edge_button_label(side, action), 12.0, Color::srgb(0.95, 0.95, 0.95)))
}

fn edge_row(label: &str, side: EdgeSide) -> Row<Node> {
  Row::<Node>::new()
    .with_node(|mut n| {
      n.align_items = AlignItems::Center;
      n.column_gap = Val::Px(4.0);
    })
    .item(static_text(label.to_string(), 11.0, Color::srgb(0.8, 0.8, 0.8)))
    .item(edge_btn(side, EdgeAction::Expand))
    .item(edge_btn(side, EdgeAction::Contract))
}

fn save_load_section() -> Column<Node> {
  Column::<Node>::new()
    .with_node(|mut n| {
      n.row_gap = Val::Px(2.0);
      n.margin = UiRect::bottom(Val::Px(4.0));
    })
    .item(
      El::<Node>::new()
        .with_node(|mut n| {
          n.width = Val::Px(92.0);
          n.height = Val::Px(18.0);
          n.padding = UiRect::horizontal(Val::Px(4.0));
          n.align_items = AlignItems::Center;
        })
        .background_color(BackgroundColor(Color::srgba(0.1, 0.1, 0.12, 0.95)))
        .insert(Button)
        .insert(SaveNameInputField)
        .child(static_text("", 10.0, Color::srgb(0.9, 0.9, 0.8)))
    )
    .item(
      Row::<Node>::new()
        .with_node(|mut n| { n.column_gap = Val::Px(4.0); })
        .item(
          El::<Node>::new()
            .with_node(|mut n| {
              n.width = Val::Px(44.0);
              n.height = Val::Px(20.0);
              n.justify_content = JustifyContent::Center;
              n.align_items = AlignItems::Center;
            })
            .background_color(BackgroundColor(Color::srgba(0.14, 0.2, 0.26, 0.95)))
            .insert(Button)
            .insert(SaveUiButton(SaveUiAction::SaveNow))
            .child(static_text("Save", 10.0, Color::srgb(0.95, 0.95, 0.95)))
        )
        .item(
          El::<Node>::new()
            .with_node(|mut n| {
              n.width = Val::Px(44.0);
              n.height = Val::Px(20.0);
              n.justify_content = JustifyContent::Center;
              n.align_items = AlignItems::Center;
            })
            .background_color(BackgroundColor(Color::srgba(0.2, 0.18, 0.12, 0.95)))
            .insert(Button)
            .insert(SaveUiButton(SaveUiAction::ToggleLoadPicker))
            .child(static_text("Load", 10.0, Color::srgb(0.95, 0.95, 0.95)))
        )
    )
}

fn edge_resize_panel() -> Column<Node> {
  Column::<Node>::new()
    .with_node(|mut n| {
      n.position_type = PositionType::Absolute;
      n.right = Val::Px(12.0);
      n.top = Val::Px(12.0);
      n.row_gap = Val::Px(4.0);
      n.padding = UiRect::all(Val::Px(6.0));
    })
    .background_color(BackgroundColor(Color::srgba(0.08, 0.08, 0.1, 0.92)))
    .item(save_load_section())
    .item(edge_row("L", EdgeSide::Left))
    .item(edge_row("R", EdgeSide::Right))
    .item(edge_row("T", EdgeSide::Top))
    .item(edge_row("B", EdgeSide::Bottom))
}

fn load_picker_panel() -> Column<Node> {
  Column::<Node>::new()
    .with_node(|mut n| {
      n.position_type = PositionType::Absolute;
      n.right = Val::Px(12.0);
      n.top = Val::Px(150.0);
      n.width = Val::Px(260.0);
      n.max_height = Val::Px(300.0);
      n.row_gap = Val::Px(4.0);
      n.padding = UiRect::all(Val::Px(6.0));
      n.display = Display::None;
    })
    .background_color(BackgroundColor(Color::srgba(0.06, 0.06, 0.08, 0.95)))
    .insert(LoadPickerPanel)
    .item(static_text("editor_saves", 11.0, Color::srgb(0.85, 0.85, 0.7)))
    .item(
      Column::<Node>::new()
        .with_node(|mut n| {
          n.row_gap = Val::Px(2.0);
          n.overflow = Overflow::scroll_y();
        })
        .insert(LoadPickerList)
    )
}

fn marker_list_panel_el() -> Column<Node> {
  Column::<Node>::new()
    .with_node(|mut n| {
      n.position_type = PositionType::Absolute;
      n.right = Val::Px(12.0);
      n.bottom = Val::Px(40.0);
      n.width = Val::Px(220.0);
      n.max_height = Val::Px(300.0);
      n.row_gap = Val::Px(4.0);
      n.padding = UiRect::all(Val::Px(6.0));
      n.display = Display::None;
      n.overflow = Overflow::scroll_y();
    })
    .background_color(BackgroundColor(Color::srgba(0.06, 0.08, 0.12, 0.95)))
    .insert(MarkerListPanel)
    .item(static_text("Markers [K]", 11.0, Color::srgb(0.3, 0.9, 1.0)))
    .item(
      Column::<Node>::new()
        .with_node(|mut n| { n.row_gap = Val::Px(2.0); })
        .insert(MarkerListContent)
    )
}

fn tile_palette_btn(tile: Tile, tile_cache: &TileImageCache) -> El<Node> {
  let (ref img_h, color) = tile_cache.0[tile as u16 as usize];
  let has_texture = *img_h != Handle::default();
  let el = El::<Node>::new()
    .with_node(|mut n| {
      n.width = Val::Px(PAL_CELL);
      n.height = Val::Px(PAL_CELL);
      n.border = UiRect::all(Val::Px(1.0));
    })
    .border_color(BorderColor::all(Color::srgba(0.3, 0.3, 0.3, 1.0)))
    .background_color(BackgroundColor(if has_texture { Color::BLACK } else { color }))
    .insert(Button)
    .insert(TilePaletteBtn(tile));
  if has_texture {
    el.child(
      El::<Node>::new()
        .with_node(|mut n| {
          n.width = Val::Percent(100.0);
          n.height = Val::Percent(100.0);
        })
        .insert(ImageNode::new(img_h.clone()))
    )
  } else {
    el
  }
}

fn object_palette_btn(obj: Option<ObjectTemplate>) -> El<Node> {
  let label = obj.map(|t| t.label()).unwrap_or("none");
  El::<Node>::new()
    .with_node(|mut n| {
      n.padding = UiRect::axes(Val::Px(4.0), Val::Px(1.0));
      n.border = UiRect::all(Val::Px(1.0));
    })
    .border_color(BorderColor::all(Color::srgba(0.3, 0.3, 0.3, 1.0)))
    .background_color(BackgroundColor(Color::srgba(0.15, 0.15, 0.2, 1.0)))
    .insert(Button)
    .insert(ObjectPaletteBtn(obj))
    .child(static_text(label, 11.0, Color::srgb(0.8, 0.8, 0.8)))
}

fn mode_bar_btn(mode: ToolMode, label: &str) -> El<Node> {
  El::<Node>::new()
    .with_node(|mut n| { n.padding = UiRect::axes(Val::Px(10.0), Val::Px(4.0)); })
    .background_color(BackgroundColor(Color::srgba(0.15, 0.15, 0.2, 0.9)))
    .insert(ModeBarBtn(mode))
    .child(
      static_text(label.to_string(), 14.0, Color::srgb(0.5, 0.5, 0.5))
        .insert(ModeBarLabel)
    )
}

fn sidebar_el(tile_cache: &TileImageCache) -> Column<Node> {
  let tile_btns: Vec<_> = Tile::all().map(|t| tile_palette_btn(t, tile_cache)).collect();
  let obj_entries: Vec<_> = std::iter::once(None)
    .chain(ObjectTemplate::all().map(Some))
    .collect();

  Column::<Node>::new()
    .with_node(|mut n| {
      n.width = Val::Px(PAL_CELL * PALETTE_COLS as f32 + 16.0);
      n.height = Val::Percent(100.0);
      n.padding = UiRect::all(Val::Px(4.0));
      n.overflow = Overflow::scroll_y();
    })
    .item(
      static_text("Tiles", 12.0, Color::srgb(0.9, 0.9, 0.5))
        .with_node(|mut n| { n.margin = UiRect::bottom(Val::Px(4.0)); })
    )
    .item(
      Row::<Node>::new()
        .with_node(|mut n| { n.flex_wrap = FlexWrap::Wrap; })
        .items(tile_btns)
    )
    .item(
      static_text("Objects", 12.0, Color::srgb(0.5, 0.9, 0.5))
        .with_node(|mut n| { n.margin = UiRect::vertical(Val::Px(6.0)); })
    )
    .item(
      Column::<Node>::new()
        .items(obj_entries.into_iter().map(object_palette_btn))
    )
    .item(
      static_text("Markers", 12.0, Color::srgb(0.3, 0.9, 1.0))
        .with_node(|mut n| { n.margin = UiRect::vertical(Val::Px(6.0)); })
    )
    .item(
      El::<Node>::new()
        .with_node(|mut n| {
          n.padding = UiRect::axes(Val::Px(4.0), Val::Px(2.0));
          n.border = UiRect::all(Val::Px(1.0));
          n.min_height = Val::Px(18.0);
        })
        .border_color(BorderColor::all(Color::srgba(0.3, 0.3, 0.3, 1.0)))
        .background_color(BackgroundColor(Color::srgba(0.1, 0.1, 0.12, 0.95)))
        .insert(Button)
        .insert(MarkerInputField)
        .child(static_text("(type name)", 10.0, Color::srgb(0.5, 0.5, 0.5)))
    )
}

fn tile_preview_popup_el() -> Column<Node> {
  Column::<Node>::new()
    .with_node(|mut n| {
      n.position_type = PositionType::Absolute;
      n.left = Val::Px(PAL_CELL * PALETTE_COLS as f32 + 24.0);
      n.top = Val::Px(8.0);
      n.align_items = AlignItems::Center;
      n.padding = UiRect::all(Val::Px(6.0));
      n.display = Display::None;
    })
    .background_color(BackgroundColor(Color::srgba(0.1, 0.1, 0.12, 0.95)))
    .insert(TilePreviewPopup)
    .item(
      El::<Node>::new()
        .with_node(|mut n| {
          n.width = Val::Px(160.0);
          n.height = Val::Px(160.0);
        })
        .background_color(BackgroundColor(Color::BLACK))
        .insert(ImageNode::new(Handle::default()))
        .insert(TilePreviewImage)
    )
    .item(
      static_text("", 18.0, Color::srgb(0.9, 0.9, 0.9))
        .with_node(|mut n| { n.margin = UiRect::top(Val::Px(6.0)); })
        .insert(TilePreviewText)
    )
}

fn mode_bar_el() -> Row<Node> {
  let modes = [
    (ToolMode::Draw, "[D]raw"),
    (ToolMode::Bucket, "[B]ucket"),
    (ToolMode::RectOutline, "[R]ect"),
    (ToolMode::RectFill, "[F]ill"),
    (ToolMode::Copy, "[C]opy"),
    (ToolMode::Move, "[M]ove"),
    (ToolMode::Paste, "[Paste]"),
  ];
  Row::<Node>::new()
    .with_node(|mut n| {
      n.position_type = PositionType::Absolute;
      n.bottom = Val::Px(0.0);
      n.left = Val::Px(0.0);
      n.width = Val::Percent(100.0);
      n.justify_content = JustifyContent::Center;
      n.align_items = AlignItems::Center;
      n.padding = UiRect::all(Val::Px(6.0));
      n.column_gap = Val::Px(4.0);
    })
    .items(modes.map(|(mode, label)| mode_bar_btn(mode, label)))
    .item(El::<Node>::new().with_node(|mut n| { n.width = Val::Px(20.0); }))
    .item(
      static_text("", 13.0, Color::srgb(0.6, 0.6, 0.5))
        .insert(ControlsLabel)
    )
}

fn build_editor_ui(tile_cache: &TileImageCache) -> impl Element {
  Stack::<Node>::new()
    .with_node(|mut n| {
      n.width = Val::Vw(100.0);
      n.height = Val::Vh(100.0);
      n.position_type = PositionType::Absolute;
      n.left = Val::Px(0.0);
      n.top = Val::Px(0.0);
    })
    .layer(sidebar_el(tile_cache))
    .layer(tile_preview_popup_el())
    .layer(mode_bar_el())
    .layer(edge_resize_panel())
    .layer(load_picker_panel())
    .layer(marker_list_panel_el())
}

fn setup(
  mut commands: Commands,
  canvas: Res<EditorCanvas>,
  mut palette_cache: ResMut<sprites::PaletteImageCache>,
  mut images: ResMut<Assets<Image>>
) {
  commands.spawn(Camera2d);

  let tile_cache = build_tile_cache(&mut palette_cache, &mut images);
  let object_visuals = build_object_visual_cache(&mut palette_cache, &mut images);

  spawn_canvas_cells(
    &mut commands,
    &canvas,
    &tile_cache,
    &object_visuals,
    CanvasGridOrigin { x: 0, y: 0 },
    0,
    INITIAL_CANVAS_W,
    0,
    INITIAL_CANVAS_H
  );

  let tileset_info = sprites::build_tileset(&mut images);
  commands.insert_resource(EditorTileset(tileset_info));
  commands.insert_resource(tile_cache);
  commands.insert_resource(object_visuals);
}

fn spawn_editor_ui(world: &mut World) {
  let tile_cache = world.resource::<TileImageCache>().clone();
  build_editor_ui(&tile_cache).spawn(world);
}

// ---------------------------------------------------------------------------
// Camera: RMB pan
// ---------------------------------------------------------------------------

fn camera_pan(
  mouse: Res<ButtonInput<MouseButton>>,
  window: Single<&Window>,
  mut camera_tf: Single<&mut Transform, With<Camera2d>>,
  mut pan: ResMut<PanState>
) {
  if let Some(cursor_pos) = window.cursor_position() {
    if mouse.just_pressed(MouseButton::Right) {
      pan.active = true;
      pan.cursor_origin = cursor_pos;
      pan.camera_origin = camera_tf.translation;
    }
    if mouse.just_released(MouseButton::Right) {
      pan.active = false;
    }
    if pan.active {
      let delta = cursor_pos - pan.cursor_origin;
      camera_tf.translation.x = pan.camera_origin.x - delta.x * camera_tf.scale.x;
      camera_tf.translation.y = pan.camera_origin.y + delta.y * camera_tf.scale.y;
    }
  }
}

// ---------------------------------------------------------------------------
// Camera: scroll wheel zoom
// ---------------------------------------------------------------------------

fn camera_zoom(
  scroll: Res<AccumulatedMouseScroll>,
  window: Single<&Window>,
  mut camera_q: Single<(&Camera, &GlobalTransform, &mut Transform), With<Camera2d>>,
  mut zoom: ResMut<CameraZoom>
) {
  if scroll.delta.y != 0.0 {
    let (cam, cam_gt, ref mut tf) = *camera_q;
    let cursor_world = window
      .cursor_position()
      .and_then(|p| cam.viewport_to_world_2d(cam_gt, p).ok());

    let old_zoom = zoom.0;
    let delta = scroll.delta.y * 0.1;
    zoom.0 = (zoom.0 * (1.0 - delta)).clamp(0.15, 8.0);

    if let Some(world_pt) = cursor_world {
      let factor = zoom.0 / old_zoom;
      tf.translation.x = world_pt.x + (tf.translation.x - world_pt.x) * factor;
      tf.translation.y = world_pt.y + (tf.translation.y - world_pt.y) * factor;
    }
    tf.scale = Vec3::splat(zoom.0);
  }
}

// ---------------------------------------------------------------------------
// UI palette interaction
// ---------------------------------------------------------------------------

fn ui_tile_palette(
  interaction_q: Query<(&Interaction, &TilePaletteBtn), Changed<Interaction>>,
  mut state: ResMut<EditorState>
) {
  for (interaction, btn) in &interaction_q {
    if *interaction == Interaction::Pressed {
      state.selected_tile = btn.0;
    }
  }
}

fn ui_tile_highlight(
  state: Res<EditorState>,
  mut btn_q: Query<(&TilePaletteBtn, &mut BorderColor)>
) {
  if state.is_changed() {
    for (btn, mut border) in &mut btn_q {
      *border = BorderColor::all(if btn.0 == state.selected_tile {
        Color::srgb(1.0, 1.0, 0.0)
      } else {
        Color::srgba(0.3, 0.3, 0.3, 1.0)
      });
    }
  }
}

fn ui_object_palette(
  interaction_q: Query<(&Interaction, &ObjectPaletteBtn), Changed<Interaction>>,
  mut state: ResMut<EditorState>
) {
  for (interaction, btn) in &interaction_q {
    if *interaction == Interaction::Pressed {
      state.selected_object = btn.0;
    }
  }
}

fn ui_object_highlight(
  state: Res<EditorState>,
  mut btn_q: Query<(&ObjectPaletteBtn, &mut BorderColor)>
) {
  if state.is_changed() {
    for (btn, mut border) in &mut btn_q {
      *border = BorderColor::all(if btn.0 == state.selected_object {
        Color::srgb(0.0, 1.0, 0.5)
      } else {
        Color::srgba(0.0, 0.0, 0.0, 0.0)
      });
    }
  }
}

fn update_tile_preview(
  palette_q: Query<(&Interaction, &TilePaletteBtn)>,
  tile_cache: Res<TileImageCache>,
  mut preview_img: Single<(&mut ImageNode, &mut BackgroundColor), With<TilePreviewImage>>,
  mut preview_text: Single<&mut Text, With<TilePreviewText>>,
  mut popup_node: Single<&mut Node, With<TilePreviewPopup>>
) {
  let hovered = palette_q
    .iter()
    .find(|(i, _)| **i == Interaction::Hovered || **i == Interaction::Pressed);
  if let Some((_, btn)) = hovered {
    let (ref img_h, color) = tile_cache.0[btn.0 as u16 as usize];
    let (ref mut img_node, ref mut img_bg) = *preview_img;
    let has_texture = *img_h != Handle::default();
    if has_texture {
      img_node.image = img_h.clone();
      img_bg.0 = Color::BLACK;
    } else {
      img_node.image = Handle::default();
      img_bg.0 = color;
    }
    preview_text.0 = btn.0.name().to_string();
    popup_node.display = Display::Flex;
  } else {
    popup_node.display = Display::None;
  }
}

fn update_mode_bar(
  state: Res<EditorState>,
  marker_input: Res<MarkerInput>,
  btn_q: Query<(&ModeBarBtn, &Children)>,
  mut label_q: Query<&mut TextColor, With<ModeBarLabel>>,
  mut status_text: Single<&mut Text, With<ControlsLabel>>
) {
  if state.is_changed() || marker_input.is_changed() {
    for (btn, children) in &btn_q {
      for child in children.iter() {
        if let Ok(mut color) = label_q.get_mut(child) {
          color.0 = if btn.0 == state.tool {
            Color::srgb(1.0, 1.0, 0.3)
          } else {
            Color::srgb(0.5, 0.5, 0.5)
          };
        }
      }
    }
    let obj_name =
      state.selected_object.map(|t| t.label()).unwrap_or("none");
    let marker_str = if marker_input.text.is_empty() { "none" } else { &marker_input.text };
    status_text.0 = format!(
      "tile:{}  obj:{}  marker:{}  pat:{}  |  U:undo G:gen K:markers [,./]:pat Ctrl+S/O:save/load",
      state.selected_tile.name(),
      obj_name,
      marker_str,
      state.pattern_size,
    );
  }
}

// ---------------------------------------------------------------------------
// Tool switching & object cycling
// ---------------------------------------------------------------------------

fn tool_keys(keys: Res<ButtonInput<KeyCode>>, mut state: ResMut<EditorState>, save_name: Res<SaveNameInput>, marker_input: Res<MarkerInput>) {
  if !save_name.focused && !marker_input.focused {
    if keys.just_pressed(KeyCode::KeyD) {
      state.tool = ToolMode::Draw;
      state.drag_start = None;
      state.paste_drag_offset = None;
    }
    if keys.just_pressed(KeyCode::KeyB) {
      state.tool = ToolMode::Bucket;
      state.drag_start = None;
      state.paste_drag_offset = None;
    }
    if keys.just_pressed(KeyCode::KeyR) {
      state.tool = ToolMode::RectOutline;
      state.drag_start = None;
      state.paste_drag_offset = None;
    }
    if keys.just_pressed(KeyCode::KeyF) {
      state.tool = ToolMode::RectFill;
      state.drag_start = None;
      state.paste_drag_offset = None;
    }
    if keys.just_pressed(KeyCode::KeyC)
      && !keys.pressed(KeyCode::ControlLeft)
      && !keys.pressed(KeyCode::ControlRight)
    {
      state.tool = ToolMode::Copy;
      state.drag_start = None;
      state.paste_drag_offset = None;
    }
    if keys.just_pressed(KeyCode::KeyM) {
      state.tool = ToolMode::Move;
      state.drag_start = None;
      state.paste_drag_offset = None;
    }
    if keys.just_pressed(KeyCode::Escape) {
      state.tool = ToolMode::Draw;
      state.drag_start = None;
      state.paste_drag_offset = None;
    }
    if keys.just_pressed(KeyCode::Comma) {
      state.pattern_size = (state.pattern_size - 1).max(2);
    }
    if keys.just_pressed(KeyCode::Period) || keys.just_pressed(KeyCode::Slash) {
      state.pattern_size = state.pattern_size.saturating_add(1);
    }
  }
}

// ---------------------------------------------------------------------------
// Canvas interaction
// ---------------------------------------------------------------------------

fn canvas_interact(
  mouse: Res<ButtonInput<MouseButton>>,
  windows: Query<&Window>,
  camera_q: Query<(&Camera, &GlobalTransform)>,
  mut state: ResMut<EditorState>,
  mut canvas: ResMut<EditorCanvas>,
  origin: Res<CanvasGridOrigin>,
  mut undo: ResMut<UndoStack>,
  pan: Res<PanState>,
  ui_buttons: Query<&Interaction, With<Button>>,
  marker_input: Res<MarkerInput>
) {
  if !pan.active
    && !utils::any(|i: &Interaction| *i == Interaction::Pressed, ui_buttons.iter())
    && let Some(cursor) = cursor_world(&windows, &camera_q)
  {
    let grid_pos = world_to_grid(cursor, &canvas, *origin);
    let grid_coord = grid_pos.map(|(x, y)| (origin.x + x as i32, origin.y + y as i32));
    let width = canvas.width();
    let height = canvas.height();
    let paint_marker = (!marker_input.text.is_empty()).then(|| marker_input.text.clone());

    match state.tool {
      ToolMode::Draw => {
        if mouse.pressed(MouseButton::Left)
          && let Some((gx, gy)) = grid_pos
        {
          if mouse.just_pressed(MouseButton::Left) {
            push_undo(&canvas, *origin, &mut undo);
          }
          canvas.tiles[gy][gx] = state.selected_tile;
          canvas.objects[gy][gx] = state.selected_object;
          canvas.markers[gy][gx] = paint_marker.clone();
        }
      }
      ToolMode::Bucket => {
        if mouse.just_pressed(MouseButton::Left)
          && let Some((gx, gy)) = grid_pos
        {
          let target = canvas.tiles[gy][gx];
          if target != state.selected_tile {
            push_undo(&canvas, *origin, &mut undo);
            flood_fill_same_tile_type(
              &mut canvas,
              gx,
              gy,
              target,
              state.selected_tile,
              state.selected_object,
              paint_marker
            );
          }
        }
      }
      ToolMode::RectOutline | ToolMode::RectFill => {
        if mouse.just_pressed(MouseButton::Left) {
          state.drag_start = grid_coord;
        }
        if mouse.just_released(MouseButton::Left)
          && let (Some(start), Some(end)) = (state.drag_start, grid_coord)
        {
          push_undo(&canvas, *origin, &mut undo);
          let (x1, y1, x2, y2) = selection_rect(start, end);
          let filled = state.tool == ToolMode::RectFill;
          for gy in y1..=y2 {
            for gx in x1..=x2 {
              if filled || gx == x1 || gx == x2 || gy == y1 || gy == y2 {
                let (ix, iy) = grid_to_index(gx, gy, *origin);
                canvas.tiles[iy][ix] = state.selected_tile;
                canvas.objects[iy][ix] = state.selected_object;
                canvas.markers[iy][ix] = paint_marker.clone();
              }
            }
          }
          state.drag_start = None;
        }
      }
      ToolMode::Copy | ToolMode::Move => {
        if mouse.just_pressed(MouseButton::Left)
          && let Some((gx, gy)) = grid_pos
        {
          state.drag_start = Some((origin.x + gx as i32, origin.y + gy as i32));
          state.paste_drag_offset = None;
        }
        if mouse.just_released(MouseButton::Left)
          && let (Some(start), Some((end_x, end_y))) = (
            state.drag_start,
            grid_pos.map(|(gx, gy)| (origin.x + gx as i32, origin.y + gy as i32))
          )
        {
          let source = ClipboardSource::from_points(start, (end_x, end_y));
          let mode = if state.tool == ToolMode::Move {
            ClipboardMode::Move
          } else {
            ClipboardMode::Copy
          };
          let mut clip_tiles = Vec::new();
          let mut clip_objects = Vec::new();
          let mut clip_markers = Vec::new();
          for gy in source.y1..=source.y2 {
            let mut row_t = Vec::new();
            let mut row_o = Vec::new();
            let mut row_m = Vec::new();
            for gx in source.x1..=source.x2 {
              let (ix, iy) = grid_to_index(gx, gy, *origin);
              row_t.push(canvas.tiles[iy][ix]);
              row_o.push(canvas.objects[iy][ix]);
              row_m.push(canvas.markers[iy][ix].clone());
            }
            clip_tiles.push(row_t);
            clip_objects.push(row_o);
            clip_markers.push(row_m);
          }
          state.clipboard =
            Some(Clipboard { tiles: clip_tiles, objects: clip_objects, markers: clip_markers, source, mode });
          state.tool = ToolMode::Paste;
          state.drag_start = None;
          state.paste_drag_offset = None;
        }
      }
      ToolMode::Paste => {
        if mouse.just_pressed(MouseButton::Left)
          && let (Some(point), Some(clip)) = (grid_coord, state.clipboard.clone())
        {
          state.paste_drag_offset =
            Some(clip.source.offset_from(point).unwrap_or((0, 0)));
        }
        if mouse.just_released(MouseButton::Left)
          && let (Some(point), Some(offset), Some(clip)) =
            (grid_coord, state.paste_drag_offset, state.clipboard.clone())
        {
          let top_left = (point.0 - offset.0, point.1 - offset.1);
          push_undo(&canvas, *origin, &mut undo);
          if clip.mode == ClipboardMode::Move {
            for gy in clip.source.y1..=clip.source.y2 {
              for gx in clip.source.x1..=clip.source.x2 {
                if let Some((ix, iy)) =
                  grid_coord_to_canvas_index(gx, gy, *origin, width, height)
                {
                  canvas.tiles[iy][ix] = Tile::Grass;
                  canvas.objects[iy][ix] = None;
                  canvas.markers[iy][ix] = None;
                }
              }
            }
          }
          for (dy, row) in clip.tiles.iter().enumerate() {
            for (dx, &tile) in row.iter().enumerate() {
              let gx = top_left.0 + dx as i32;
              let gy = top_left.1 + dy as i32;
              if let Some((ix, iy)) =
                grid_coord_to_canvas_index(gx, gy, *origin, width, height)
              {
                canvas.tiles[iy][ix] = tile;
                canvas.objects[iy][ix] = clip.objects[dy][dx];
                canvas.markers[iy][ix] = clip.markers[dy][dx].clone();
              }
            }
          }
          state.tool = ToolMode::Draw;
          state.drag_start = None;
          state.paste_drag_offset = None;
        }
        if mouse.just_released(MouseButton::Left) {
          state.paste_drag_offset = None;
        }
      }
    }
  }
}

// ---------------------------------------------------------------------------
// Eyedropper (Alt+click)
// ---------------------------------------------------------------------------

fn eyedropper(
  mouse: Res<ButtonInput<MouseButton>>,
  keys: Res<ButtonInput<KeyCode>>,
  windows: Query<&Window>,
  camera_q: Query<(&Camera, &GlobalTransform)>,
  canvas: Res<EditorCanvas>,
  origin: Res<CanvasGridOrigin>,
  mut state: ResMut<EditorState>,
  mut marker_input: ResMut<MarkerInput>
) {
  let pick = (keys.pressed(KeyCode::AltLeft) && mouse.just_pressed(MouseButton::Left))
    || mouse.just_pressed(MouseButton::Middle);
  if pick {
    if let Some((gx, gy)) = cursor_world(&windows, &camera_q)
      .and_then(|cursor| world_to_grid(cursor, &canvas, *origin))
    {
      state.selected_tile = canvas.tiles[gy][gx];
      state.selected_object = canvas.objects[gy][gx];
      marker_input.text = canvas.markers[gy][gx].clone().unwrap_or_default();
    }
  }
}

// ---------------------------------------------------------------------------
// Copy / Cut / Paste / Undo
// ---------------------------------------------------------------------------

fn undo_key(
  keys: Res<ButtonInput<KeyCode>>,
  mut canvas: ResMut<EditorCanvas>,
  mut origin: ResMut<CanvasGridOrigin>,
  mut undo: ResMut<UndoStack>,
  spawned: Res<SpawnedCanvasSize>,
  save_name: Res<SaveNameInput>,
  marker_input: Res<MarkerInput>
) {
  let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
  if !save_name.focused && !marker_input.focused && (keys.just_pressed(KeyCode::KeyU) || (ctrl && keys.just_pressed(KeyCode::KeyZ))) {
    if let Some((tiles, objects, markers, undo_origin)) = undo.0.pop() {
      canvas.tiles = tiles;
      canvas.objects = objects;
      canvas.markers = markers;
      canvas.ensure_size(spawned.width, spawned.height);
      *origin = undo_origin;
    }
  }
}

// ---------------------------------------------------------------------------
// Sync canvas tile sprites
// ---------------------------------------------------------------------------

fn ensure_canvas_entities(
  mut commands: Commands,
  canvas: Res<EditorCanvas>,
  origin: Res<CanvasGridOrigin>,
  tile_cache: Res<TileImageCache>,
  object_visuals: Res<ObjectVisualCache>,
  mut spawned: ResMut<SpawnedCanvasSize>,
  existing_cells: Query<(Entity, Option<&Children>), With<CanvasCell>>
) {
  if canvas.is_changed() || origin.is_changed() {
    let width = canvas.width();
    let height = canvas.height();

    let shifted = origin.x != spawned.origin_x || origin.y != spawned.origin_y;
    let shrunk = width < spawned.width || height < spawned.height;
    if shifted || shrunk {
      for (entity, children) in &existing_cells {
        if let Some(children) = children {
          for child in children.iter() {
            commands.entity(child).despawn();
          }
        }
        commands.entity(entity).despawn();
      }
      spawn_canvas_cells(
        &mut commands,
        &canvas,
        &tile_cache,
        &object_visuals,
        *origin,
        0,
        width,
        0,
        height
      );
    } else {
      if width > spawned.width {
        spawn_canvas_cells(
          &mut commands,
          &canvas,
          &tile_cache,
          &object_visuals,
          *origin,
          spawned.width,
          width,
          0,
          height
        );
      }
      if height > spawned.height {
        spawn_canvas_cells(
          &mut commands,
          &canvas,
          &tile_cache,
          &object_visuals,
          *origin,
          0,
          spawned.width.min(width),
          spawned.height,
          height
        );
      }
    }

    spawned.width = width;
    spawned.height = height;
    spawned.origin_x = origin.x;
    spawned.origin_y = origin.y;
  }
}

fn clear_wfc_preview(
  commands: &mut Commands,
  existing: &Query<Entity, Or<(With<OutputChunk>, With<OutputLabel>)>>
) {
  for entity in existing {
    commands.entity(entity).despawn();
  }
}

fn resize_canvas_with_edge_buttons(
  time: Res<Time>,
  interaction_q: Query<(&Interaction, &EdgeResizeButton), With<Button>>,
  mut commands: Commands,
  existing_preview: Query<Entity, Or<(With<OutputChunk>, With<OutputLabel>)>>,
  mut canvas: ResMut<EditorCanvas>,
  mut origin: ResMut<CanvasGridOrigin>,
  mut undo: ResMut<UndoStack>,
  mut hold: ResMut<ResizeHoldState>
) {
  let pressed = interaction_q.iter().find_map(|(interaction, button)| {
    (*interaction == Interaction::Pressed).then_some(*button)
  });

  if let Some(button) = pressed {
    if hold.active != Some(button) {
      hold.active = Some(button);
      hold.held_for = 0.0;
      hold.repeat_accum = 0.0;
      push_undo(&canvas, *origin, &mut undo);
      if button.action == EdgeAction::Expand {
        clear_wfc_preview(&mut commands, &existing_preview);
      }
      canvas.resize_edge(button.side, button.action, &mut origin);
    } else {
      let dt = time.delta_secs();
      hold.held_for += dt;
      if hold.held_for >= RESIZE_HOLD_INITIAL_DELAY {
        hold.repeat_accum += dt;
        while hold.repeat_accum >= RESIZE_HOLD_REPEAT {
          push_undo(&canvas, *origin, &mut undo);
          if button.action == EdgeAction::Expand {
            clear_wfc_preview(&mut commands, &existing_preview);
          }
          canvas.resize_edge(button.side, button.action, &mut origin);
          hold.repeat_accum -= RESIZE_HOLD_REPEAT;
        }
      }
    }
  } else {
    hold.active = None;
    hold.held_for = 0.0;
    hold.repeat_accum = 0.0;
  }
}

// ---------------------------------------------------------------------------
// Sync canvas tile sprites
// ---------------------------------------------------------------------------

fn sync_canvas_positions(
  origin: Res<CanvasGridOrigin>,
  mut query: Query<(&CanvasCell, &mut Transform)>
) {
  if origin.is_changed() {
    for (cell, mut transform) in &mut query {
      let world = grid_to_world(cell.0, cell.1, *origin);
      transform.translation.x = world.x;
      transform.translation.y = world.y;
    }
  }
}

fn sync_canvas_sprites(
  canvas: Res<EditorCanvas>,
  tile_cache: Res<TileImageCache>,
  mut query: Query<(&CanvasCell, &mut Sprite)>
) {
  if canvas.is_changed() {
    for (cell, mut sprite) in &mut query {
      if let Some(tile) =
        canvas.tiles.get(cell.1).and_then(|row| row.get(cell.0)).copied()
      {
        let (ref img, color) = tile_cache.0[tile as u16 as usize];
        sprite.image = img.clone();
        sprite.color = color;
      }
    }
  }
}

// ---------------------------------------------------------------------------
// Sync object visuals
// ---------------------------------------------------------------------------

fn sync_object_visuals(
  canvas: Res<EditorCanvas>,
  object_visuals: Res<ObjectVisualCache>,
  mut queries: ParamSet<(
    Query<(&CanvasObjectSprite, &mut Sprite, &mut Visibility)>,
    Query<(&CanvasObjectText, &mut Text2d, &mut TextColor, &mut Visibility)>
  )>
) {
  if canvas.is_changed() {
    for (label, mut sprite, mut visibility) in &mut queries.p0() {
      if let Some(obj_idx) = canvas
        .objects
        .get(label.1)
        .and_then(|row| row.get(label.0))
        .copied()
        .flatten()
        .map(|idx| idx as usize)
        && let Some(image) =
          object_visuals.0.get(obj_idx).and_then(|visual| visual.image.as_ref())
      {
        sprite.image = image.clone();
        *visibility = Visibility::Visible;
      } else {
        *visibility = Visibility::Hidden;
      }
    }
    for (label, mut text, mut text_color, mut visibility) in &mut queries.p1() {
      if let Some(obj_idx) = canvas
        .objects
        .get(label.1)
        .and_then(|row| row.get(label.0))
        .copied()
        .flatten()
        .map(|idx| idx as usize)
        && let Some(visual) = object_visuals.0.get(obj_idx)
        && visual.image.is_none()
      {
        text.0 = visual.text.clone();
        text_color.0 = visual.text_color;
        *visibility = Visibility::Visible;
      } else {
        text.0.clear();
        *visibility = Visibility::Hidden;
      }
    }
  }
}

fn sync_marker_visuals(
  canvas: Res<EditorCanvas>,
  mut query: Query<(&CanvasMarkerText, &mut Text2d, &mut TextColor, &mut Visibility)>
) {
  if canvas.is_changed() {
    for (label, mut text, mut color, mut visibility) in &mut query {
      if let Some(name) = canvas
        .markers
        .get(label.1)
        .and_then(|row| row.get(label.0))
        .and_then(|m| m.as_deref())
      {
        text.0 = abbreviate_marker(name);
        color.0 = marker_color(name);
        *visibility = Visibility::Visible;
      } else {
        text.0.clear();
        *visibility = Visibility::Hidden;
      }
    }
  }
}

// ---------------------------------------------------------------------------
// Selection & drag preview overlays
// ---------------------------------------------------------------------------

fn spawn_drag_preview(
  commands: &mut Commands,
  top_left: (i32, i32),
  width: usize,
  height: usize,
  color: Color
) {
  if width > 0 && height > 0 {
    let x2 = top_left.0 + width as i32 - 1;
    let y2 = top_left.1 + height as i32 - 1;
    let tl = grid_coord_to_world(top_left.0, top_left.1);
    let br = grid_coord_to_world(x2, y2);
    let center = (tl + br) / 2.0;
    let size = Vec2::new(width as f32 * STEP, height as f32 * STEP);
    commands.spawn((
      Sprite { color, custom_size: Some(size), ..default() },
      Transform::from_xyz(center.x, center.y, 2.0),
      DragPreview
    ));
  }
}

fn update_overlays(
  mut commands: Commands,
  state: Res<EditorState>,
  canvas: Res<EditorCanvas>,
  origin: Res<CanvasGridOrigin>,
  windows: Query<&Window>,
  camera_q: Query<(&Camera, &GlobalTransform)>,
  existing_drag: Query<Entity, With<DragPreview>>
) {
  for e in &existing_drag {
    commands.entity(e).despawn();
  }

  if state.tool == ToolMode::Paste {
    if let Some(clip) = state.clipboard.as_ref() {
      let cursor_point = cursor_world(&windows, &camera_q)
        .and_then(|cursor| world_to_grid(cursor, &canvas, *origin))
        .map(|(x, y)| (origin.x + x as i32, origin.y + y as i32));
      let top_left =
        if let (Some(point), Some(offset)) = (cursor_point, state.paste_drag_offset) {
          (point.0 - offset.0, point.1 - offset.1)
        } else {
          clip.source.top_left()
        };
      let color = if clip.mode == ClipboardMode::Move {
        Color::srgba(1.0, 0.5, 0.1, 0.3)
      } else {
        Color::srgba(0.2, 0.6, 1.0, 0.3)
      };
      spawn_drag_preview(&mut commands, top_left, clip.width(), clip.height(), color);
    }
  } else if state.drag_start.is_some() {
    if let Some(cursor) = cursor_world(&windows, &camera_q) {
      let end = world_to_grid(cursor, &canvas, *origin)
        .map(|(x, y)| (origin.x + x as i32, origin.y + y as i32));
      if let (Some(start), Some(end)) = (state.drag_start, end) {
        let (x1, y1, x2, y2) = selection_rect(start, end);
        let color = match state.tool {
          ToolMode::RectOutline | ToolMode::RectFill => Color::srgba(1.0, 1.0, 0.0, 0.3),
          _ => Color::srgba(0.2, 0.6, 1.0, 0.3)
        };
        spawn_drag_preview(
          &mut commands,
          (x1, y1),
          (x2 - x1 + 1) as usize,
          (y2 - y1 + 1) as usize,
          color
        );
      }
    }
  }
}

// ---------------------------------------------------------------------------
// WFC generation
// ---------------------------------------------------------------------------

fn encode_cell(tile: Tile, obj: Option<ObjectTemplate>) -> u16 {
  (tile as u16) | ((obj.map(|o| o as u16 + 1).unwrap_or(0)) << 8)
}

fn decode_cell(val: u16) -> (Option<Tile>, Option<ObjectTemplate>) {
  let tile = Tile::try_from(val & 0xFF).ok();
  let obj = match val >> 8 {
    0 => None,
    n => ObjectTemplate::try_from((n - 1) as u8).ok()
  };
  (tile, obj)
}

#[derive(Component)]
struct OutputLabel;

fn generate_wfc(
  keys: Res<ButtonInput<KeyCode>>,
  canvas: Res<EditorCanvas>,
  origin: Res<CanvasGridOrigin>,
  state: Res<EditorState>,
  tileset: Res<EditorTileset>,
  object_visuals: Res<ObjectVisualCache>,
  mut commands: Commands,
  (existing, save_name, marker_input): (Query<Entity, Or<(With<OutputChunk>, With<OutputLabel>)>>, Res<SaveNameInput>, Res<MarkerInput>)
) {
  if keys.just_pressed(KeyCode::KeyG) && !save_name.focused && !marker_input.focused {

  clear_wfc_preview(&mut commands, &existing);

  let canvas_width = canvas.width();
  let canvas_height = canvas.height();
  let ow = INITIAL_CANVAS_W as u32 * state.output_mult;
  let oh = INITIAL_CANVAS_H as u32 * state.output_mult;

  let input_grid = Grid::new_fn(
    coord_2d::Size::new(canvas_width as u32, canvas_height as u32),
    |coord| {
      encode_cell(
        canvas.tiles[coord.y as usize][coord.x as usize],
        canvas.objects[coord.y as usize][coord.x as usize]
      )
    }
  );

  let pattern_size = NonZeroU32::new(state.pattern_size).unwrap();
  let patterns = OverlappingPatterns::new(input_grid, pattern_size, &[
    wfc::orientation::Orientation::Original,
    wfc::orientation::Orientation::Clockwise90,
    wfc::orientation::Orientation::Clockwise180,
    wfc::orientation::Orientation::Clockwise270,
    wfc::orientation::Orientation::DiagonallyFlipped,
    wfc::orientation::Orientation::DiagonallyFlippedClockwise90,
    wfc::orientation::Orientation::DiagonallyFlippedClockwise180,
    wfc::orientation::Orientation::DiagonallyFlippedClockwise270
  ]);

  let global_stats = patterns.global_stats();
  let mut rng = rand::thread_rng();
  let output_size = coord_2d::Size::new(ow, oh);

  let run = RunOwn::new(output_size, &global_stats, &mut rng);
  let result: Result<Wave, _> = NumTimes(20).retry(run, &mut rng);

  match result {
    Ok(wave) => {
      let right_cell_grid_x = origin.x + canvas_width as i32 - 1;
      let top_cell_grid_y = origin.y;
      let bottom_cell_grid_y = origin.y + canvas_height as i32 - 1;
      let canvas_right_edge = grid_coord_to_world(right_cell_grid_x, top_cell_grid_y).x;
      let output_gap = 40.0;
      let output_half_w = ow as f32 * CELL / 2.0;
      let output_half_h = oh as f32 * CELL / 2.0;
      // `TilemapChunk` mesh is centered on the entity transform (see bevy `calculate_tile_transform`).
      let output_center_x = canvas_right_edge + output_gap + output_half_w;
      let top_y = grid_coord_to_world(origin.x, top_cell_grid_y).y;
      let bottom_y = grid_coord_to_world(origin.x, bottom_cell_grid_y).y;
      let canvas_center_y = (top_y + bottom_y) / 2.0;
      let output_center_y = canvas_center_y;

      let mut tile_data: Vec<Option<TileData>> = vec![None; (ow * oh) as usize];
      for coord_y in 0..oh {
        for coord_x in 0..ow {
          let cell = wave
            .grid()
            .get(coord_2d::Coord::new(coord_x as i32, coord_y as i32))
            .unwrap();
          let val =
            cell.chosen_pattern_id().ok().map(|id| *patterns.pattern_top_left_value(id));
          let (tile, obj) = val.map(decode_cell).unwrap_or((None, None));
          if let Some(tile) = tile {
            let info = tileset.0.layer_range[tile as usize];
            let tileset_index = match info.select {
              sprites::TileSelect::Single => info.base,
              sprites::TileSelect::RandomHash => {
                let h: u64 = (coord_x as u64) | ((coord_y as u64) << 32);
                let h = (h ^ (h >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
                let h = (h ^ (h >> 27)).wrapping_mul(0x94d049bb133111eb);
                let h = h ^ (h >> 31);
                info.base + (h as u16) % info.count
              }
              sprites::TileSelect::Connected => info.base
            };
            let chunk_idx = ((oh - 1 - coord_y) * ow + coord_x) as usize;
            tile_data[chunk_idx] =
              Some(TileData { tileset_index, color: Color::WHITE, visible: true });
          }
          if let Some(tmpl) = obj
            && let Some(visual) = object_visuals.0.get(tmpl as u8 as usize)
          {
            let ux = coord_x as f32;
            let uy = (oh - 1 - coord_y) as f32;
            let lx = ux * CELL + CELL / 2.0 - output_half_w;
            let ly = uy * CELL + CELL / 2.0 - output_half_h;
            let tx = output_center_x + lx;
            let ty = output_center_y + ly;
            if let Some(image) = &visual.image {
              commands.spawn((
                Sprite {
                  image: image.clone(),
                  color: Color::WHITE,
                  custom_size: Some(Vec2::splat(CELL)),
                  ..default()
                },
                Transform::from_xyz(tx, ty, 1.0),
                OutputLabel
              ));
            } else {
              commands.spawn((
                Text2d::new(visual.text.clone()),
                TextFont { font_size: 8.0, ..default() },
                TextColor(visual.text_color),
                Transform::from_xyz(tx, ty, 1.0),
                OutputLabel
              ));
            }
          }
        }
      }

      commands.spawn((
        TilemapChunk {
          chunk_size: UVec2::new(ow, oh),
          tile_display_size: UVec2::splat(CELL as u32),
          tileset: tileset.0.handle.clone(),
          alpha_mode: AlphaMode2d::Blend
        },
        TilemapChunkTileData(tile_data),
        Transform::from_xyz(output_center_x, output_center_y, 0.01),
        OutputChunk
      ));
      eprintln!("WFC generated {ow}x{oh} output (pattern_size={})", state.pattern_size);
    }
    Err(_) => {
      eprintln!("WFC generation failed after retries");
    }
  }
  }
}

// ---------------------------------------------------------------------------
// Save / Load
// ---------------------------------------------------------------------------

fn save_timestamp() -> u128 {
  SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis()).unwrap_or(0)
}

fn save_files() -> Vec<PathBuf> {
  let mut files = std::fs::read_dir(SAVE_DIR)
    .ok()
    .into_iter()
    .flat_map(|entries| entries.filter_map(Result::ok))
    .map(|entry| entry.path())
    .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("txt"))
    .collect::<Vec<_>>();
  files.sort();
  files.reverse();
  files
}

fn save_canvas(canvas: &EditorCanvas, origin: CanvasGridOrigin, name: &str) -> Option<PathBuf> {
  let _ = std::fs::create_dir_all(SAVE_DIR);
  let width = canvas.width();
  let height = canvas.height();
  let filename = if name.is_empty() {
    format!("editor_save_{}.txt", save_timestamp())
  } else {
    format!("{name}.txt")
  };
  let path = PathBuf::from(SAVE_DIR).join(filename);
  let mut out = format!("{width} {height} {} {}\n", origin.x, origin.y);
  for y in 0..height {
    for x in 0..width {
      let t = canvas.tiles[y][x];
      let o = canvas.objects[y][x].map(|v| format!("{v:?}")).unwrap_or_else(|| "-".into());
      out.push_str(&format!("{t:?} {o} "));
    }
    out.push('\n');
  }
  out.push_str("MARKERS\n");
  for y in 0..height {
    for x in 0..width {
      if let Some(name) = &canvas.markers[y][x] {
        out.push_str(&format!("{x} {y} {name}\n"));
      }
    }
  }
  std::fs::write(&path, &out).ok().map(|_| path)
}

fn load_canvas_from_file(
  path: &Path,
  canvas: &mut EditorCanvas,
  origin: &mut CanvasGridOrigin,
  undo: &mut UndoStack
) -> bool {
  std::fs::read_to_string(path)
    .ok()
    .map(|text| {
      let (grid_section, marker_section) = text
        .split_once("MARKERS\n")
        .map(|(a, b)| (a, Some(b)))
        .unwrap_or((&text, None));
      let mut nums = grid_section.split_whitespace();
      let w: usize = nums.next().and_then(|s| s.parse().ok()).unwrap_or(INITIAL_CANVAS_W);
      let h: usize = nums.next().and_then(|s| s.parse().ok()).unwrap_or(INITIAL_CANVAS_H);
      let remaining: Vec<&str> = nums.collect();
      let cell_tokens = w.saturating_mul(h).saturating_mul(2);
      let (saved_origin_x, saved_origin_y, data_start) =
        if remaining.len() >= cell_tokens + 2
          && remaining[0].parse::<i32>().is_ok()
          && remaining[1].parse::<i32>().is_ok()
        {
          (
            remaining[0].parse().unwrap_or(0),
            remaining[1].parse().unwrap_or(0),
            2
          )
        } else {
          (0, 0, 0)
        };
      let mut data_idx = data_start;
      push_undo(canvas, *origin, undo);
      canvas.resize_exact(w, h);
      origin.x = saved_origin_x;
      origin.y = saved_origin_y;
      let width = canvas.width();
      let height = canvas.height();
      for y in 0..height {
        for x in 0..width {
          canvas.tiles[y][x] = Tile::Grass;
          canvas.objects[y][x] = None;
          canvas.markers[y][x] = None;
        }
      }
      for y in 0..h.min(height) {
        for x in 0..w.min(width) {
          let tile_tok = remaining.get(data_idx).copied().unwrap_or("Grass");
          data_idx += 1;
          let obj_tok = remaining.get(data_idx).copied().unwrap_or("-");
          data_idx += 1;
          canvas.tiles[y][x] = Tile::from_save(tile_tok).unwrap_or(Tile::Grass);
          canvas.objects[y][x] = ObjectTemplate::from_save(obj_tok);
        }
      }
      if let Some(markers_text) = marker_section {
        for line in markers_text.lines() {
          let mut parts = line.split_whitespace();
          if let (Some(x), Some(y), Some(name)) =
            (parts.next().and_then(|s| s.parse::<usize>().ok()),
             parts.next().and_then(|s| s.parse::<usize>().ok()),
             parts.next())
          {
            if x < width && y < height {
              canvas.markers[y][x] = Some(name.to_string());
            }
          }
        }
      }
    })
    .is_some()
}

fn save_name_input_focus(
  interaction_q: Query<&Interaction, (Changed<Interaction>, With<SaveNameInputField>)>,
  mut save_name: ResMut<SaveNameInput>
) {
  for interaction in &interaction_q {
    if *interaction == Interaction::Pressed {
      save_name.focused = true;
    }
  }
}

fn save_name_input_unfocus(
  mouse: Res<ButtonInput<MouseButton>>,
  interaction_q: Query<&Interaction, With<SaveNameInputField>>,
  mut save_name: ResMut<SaveNameInput>
) {
  if mouse.just_pressed(MouseButton::Left)
    && save_name.focused
    && interaction_q.iter().all(|i| *i != Interaction::Pressed)
  {
    save_name.focused = false;
  }
}

fn save_name_input_typing(
  mut events: MessageReader<KeyboardInput>,
  keys: Res<ButtonInput<KeyCode>>,
  mut save_name: ResMut<SaveNameInput>
) {
  if !save_name.focused {
    events.clear();
  } else {
    for event in events.read() {
      if event.state.is_pressed() {
        match (&event.logical_key, &event.text) {
          (Key::Backspace, _) => { save_name.text.pop(); }
          (Key::Escape | Key::Enter, _) => { save_name.focused = false; }
          _ if keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight) => {}
          (_, Some(ch)) => {
            if ch.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-') {
              save_name.text.push_str(ch);
            }
          }
          _ => {}
        }
      }
    }
  }
}

fn save_name_input_display(
  save_name: Res<SaveNameInput>,
  field_q: Query<(Entity, &Children), With<SaveNameInputField>>,
  mut bg_q: Query<&mut BackgroundColor, With<SaveNameInputField>>,
  mut text_q: Query<&mut Text>
) {
  if save_name.is_changed() {
    for (_entity, children) in &field_q {
      for child in children.iter() {
        if let Ok(mut text) = text_q.get_mut(child) {
          let display = if save_name.text.is_empty() && !save_name.focused {
            "(unnamed)".into()
          } else if save_name.focused {
            format!("{}|", save_name.text)
          } else {
            save_name.text.clone()
          };
          **text = display;
        }
      }
    }
    for mut bg in &mut bg_q {
      *bg = if save_name.focused {
        BackgroundColor(Color::srgba(0.15, 0.15, 0.2, 0.95))
      } else {
        BackgroundColor(Color::srgba(0.1, 0.1, 0.12, 0.95))
      };
    }
  }
}

fn save_load_hotkeys(
  keys: Res<ButtonInput<KeyCode>>,
  canvas: Res<EditorCanvas>,
  origin: Res<CanvasGridOrigin>,
  mut picker: ResMut<LoadPickerState>,
  save_name: Res<SaveNameInput>
) {
  let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
  if ctrl && keys.just_pressed(KeyCode::KeyS) {
    if let Some(path) = save_canvas(&canvas, *origin, &save_name.text) {
      eprintln!("Saved to {}", path.display());
      picker.refresh_requested = picker.open;
    }
  }
  if ctrl && keys.just_pressed(KeyCode::KeyO) {
    picker.open = !picker.open;
    picker.refresh_requested = picker.open;
  }
}

fn save_load_ui_actions(
  interaction_q: Query<(&Interaction, &SaveUiButton), Changed<Interaction>>,
  canvas: Res<EditorCanvas>,
  origin: Res<CanvasGridOrigin>,
  mut picker: ResMut<LoadPickerState>,
  save_name: Res<SaveNameInput>
) {
  for (interaction, button) in &interaction_q {
    if *interaction == Interaction::Pressed {
      match button.0 {
        SaveUiAction::SaveNow => {
          if let Some(path) = save_canvas(&canvas, *origin, &save_name.text) {
            eprintln!("Saved to {}", path.display());
            picker.refresh_requested = picker.open;
          }
        }
        SaveUiAction::ToggleLoadPicker => {
          picker.open = !picker.open;
          picker.refresh_requested = picker.open;
        }
      }
    }
  }
}

fn load_picker_visibility(
  picker: Res<LoadPickerState>,
  mut node: Single<&mut Node, With<LoadPickerPanel>>
) {
  if picker.is_changed() {
    node.display = if picker.open { Display::Flex } else { Display::None };
  }
}

fn refresh_load_picker_list(
  mut commands: Commands,
  mut picker: ResMut<LoadPickerState>,
  list_entity: Single<Entity, With<LoadPickerList>>,
  existing_q: Query<Entity, With<LoadPickerListItem>>
) {
  if picker.refresh_requested && picker.open {
    picker.refresh_requested = false;
    for entity in &existing_q {
      commands.entity(entity).despawn();
    }
    commands.entity(*list_entity).with_children(|parent| {
      for path in save_files() {
        let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("unknown");
        parent
          .spawn((
            Button,
            Node {
              width: Val::Percent(100.0),
              padding: UiRect::axes(Val::Px(4.0), Val::Px(2.0)),
              ..default()
            },
            BackgroundColor(Color::srgba(0.14, 0.14, 0.18, 0.95)),
            LoadPickerListItem,
            LoadPickerFileButton(path.to_string_lossy().to_string())
          ))
          .with_child((
            Text::new(name),
            TextFont { font_size: 10.0, ..default() },
            TextColor(Color::srgb(0.9, 0.9, 0.9))
          ));
      }
    });
  }
}

fn load_picker_file_clicks(
  interaction_q: Query<(&Interaction, &LoadPickerFileButton), Changed<Interaction>>,
  mut canvas: ResMut<EditorCanvas>,
  mut origin: ResMut<CanvasGridOrigin>,
  mut undo: ResMut<UndoStack>,
  mut picker: ResMut<LoadPickerState>,
  mut save_name: ResMut<SaveNameInput>
) {
  for (interaction, file_btn) in &interaction_q {
    if *interaction == Interaction::Pressed {
      let path = PathBuf::from(&file_btn.0);
      if load_canvas_from_file(&path, &mut canvas, &mut origin, &mut undo) {
        save_name.text = path.file_stem()
          .and_then(|s| s.to_str())
          .unwrap_or("")
          .to_string();
        eprintln!("Loaded from {}", path.display());
        picker.open = false;
        picker.refresh_requested = false;
      }
    }
  }
}

// ---------------------------------------------------------------------------
// Export prefab
// ---------------------------------------------------------------------------

fn export_prefab(keys: Res<ButtonInput<KeyCode>>, canvas: Res<EditorCanvas>, save_name: Res<SaveNameInput>, marker_input: Res<MarkerInput>) {
  let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
  if !save_name.focused && !marker_input.focused && keys.just_pressed(KeyCode::KeyE) && !ctrl {
    let mut chars_used = HashMap::<u16, char>::new();
    let mut next_char = b'a';

    for row in &canvas.tiles {
      for &tile in row {
        chars_used.entry(tile as u16).or_insert_with(|| {
          let c = next_char as char;
          next_char += 1;
          if next_char == b'{' {
            next_char = b'A';
          }
          c
        });
      }
    }

    let mut layout = String::new();
    for row in &canvas.tiles {
      for &tile in row {
        layout.push(chars_used[&(tile as u16)]);
      }
      layout.push('\n');
    }

    let mut assocs = String::from("// Associations:\n");
    let mut sorted: Vec<_> = chars_used.iter().collect();
    sorted.sort_by_key(|(_, c)| **c);
    for (disc, ch) in &sorted {
      if let Ok(t) = Tile::try_from(**disc) {
        assocs.push_str(&format!("// '{}' => Tile::{:?}\n", ch, t));
      }
    }

    let mut markers_out = String::new();
    let height = canvas.height();
    let width = canvas.width();
    for y in 0..height {
      for x in 0..width {
        if let Some(name) = &canvas.markers[y][x] {
          markers_out.push_str(&format!("// marker({x}, {y}) = \"{name}\"\n"));
        }
      }
    }

    let out = format!("{}\n{}{}", layout, assocs, markers_out);
    let path = "editor_export.txt";
    std::fs::write(path, &out).unwrap();
    eprintln!("Exported to {path}");
  }
}

// ---------------------------------------------------------------------------
// Marker input
// ---------------------------------------------------------------------------

fn marker_input_focus(
  interaction_q: Query<&Interaction, (Changed<Interaction>, With<MarkerInputField>)>,
  mut marker_input: ResMut<MarkerInput>
) {
  for interaction in &interaction_q {
    if *interaction == Interaction::Pressed {
      marker_input.focused = true;
    }
  }
}

fn marker_input_unfocus(
  mouse: Res<ButtonInput<MouseButton>>,
  interaction_q: Query<&Interaction, With<MarkerInputField>>,
  mut marker_input: ResMut<MarkerInput>
) {
  if mouse.just_pressed(MouseButton::Left)
    && marker_input.focused
    && interaction_q.iter().all(|i| *i != Interaction::Pressed)
  {
    marker_input.focused = false;
  }
}

fn marker_input_typing(
  mut events: MessageReader<KeyboardInput>,
  keys: Res<ButtonInput<KeyCode>>,
  mut marker_input: ResMut<MarkerInput>
) {
  if !marker_input.focused {
    events.read().last();
  } else {
    for ev in events.read() {
      if !ev.state.is_pressed() || keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight) {
      } else {
        match (&ev.logical_key, ev.key_code) {
          (Key::Backspace, _) => { marker_input.text.pop(); }
          (Key::Escape | Key::Enter, _) => { marker_input.focused = false; }
          (Key::Character(ch), _) if !ch.is_empty() => {
            let filtered: String = ch.chars().filter(|c| !c.is_whitespace()).collect();
            marker_input.text.push_str(&filtered);
          }
          _ => {}
        }
      }
    }
  }
}

fn marker_input_display(
  marker_input: Res<MarkerInput>,
  field_q: Query<(&Children, Entity), With<MarkerInputField>>,
  mut text_q: Query<(&mut Text, &mut TextColor)>,
  mut bg_q: Query<&mut BackgroundColor>
) {
  if marker_input.is_changed() {
    for (children, entity) in &field_q {
      for child in children.iter() {
        if let Ok((mut text, mut color)) = text_q.get_mut(child) {
          let display = if marker_input.text.is_empty() && !marker_input.focused {
            "(type name)".to_string()
          } else if marker_input.focused {
            format!("{}|", marker_input.text)
          } else {
            marker_input.text.clone()
          };
          text.0 = display;
          color.0 = if marker_input.text.is_empty() && !marker_input.focused {
            Color::srgb(0.5, 0.5, 0.5)
          } else {
            Color::srgb(0.3, 0.9, 1.0)
          };
        }
      }
      if let Ok(mut bg) = bg_q.get_mut(entity) {
        *bg = if marker_input.focused {
          BackgroundColor(Color::srgba(0.12, 0.15, 0.2, 0.95))
        } else {
          BackgroundColor(Color::srgba(0.1, 0.1, 0.12, 0.95))
        };
      }
    }
  }
}

// ---------------------------------------------------------------------------
// Marker list panel
// ---------------------------------------------------------------------------

fn marker_list_toggle(
  keys: Res<ButtonInput<KeyCode>>,
  save_name: Res<SaveNameInput>,
  marker_input: Res<MarkerInput>,
  mut state: ResMut<MarkerListState>
) {
  if !save_name.focused && !marker_input.focused && keys.just_pressed(KeyCode::KeyK) {
    state.open = !state.open;
    state.needs_refresh = state.open;
  }
}

fn marker_list_visibility(
  state: Res<MarkerListState>,
  mut node: Single<&mut Node, With<MarkerListPanel>>
) {
  if state.is_changed() {
    node.display = if state.open { Display::Flex } else { Display::None };
  }
}

fn refresh_marker_list(
  mut commands: Commands,
  canvas: Res<EditorCanvas>,
  mut state: ResMut<MarkerListState>,
  origin: Res<CanvasGridOrigin>,
  content_entity: Single<Entity, With<MarkerListContent>>,
  existing_items: Query<Entity, With<MarkerListButton>>
) {
  if !state.open {
    state.needs_refresh = false;
  }
  if state.needs_refresh || (state.open && canvas.is_changed()) {
    state.needs_refresh = false;
    for e in &existing_items {
      commands.entity(e).despawn();
    }

    let mut entries: Vec<(usize, usize, String)> = Vec::new();
    for (y, row) in canvas.markers.iter().enumerate() {
      for (x, marker) in row.iter().enumerate() {
        if let Some(name) = marker {
          entries.push((x, y, name.clone()));
        }
      }
    }
    entries.sort_by(|a, b| a.2.cmp(&b.2).then(a.1.cmp(&b.1)).then(a.0.cmp(&b.0)));

    commands.entity(*content_entity).with_children(|parent| {
      for (x, y, name) in &entries {
        let gx = origin.x + *x as i32;
        let gy = origin.y + *y as i32;
        parent
          .spawn((
            Button,
            Node {
              padding: UiRect::axes(Val::Px(4.0), Val::Px(1.0)),
              ..default()
            },
            BackgroundColor(Color::srgba(0.1, 0.14, 0.18, 0.95)),
            MarkerListButton(*x, *y)
          ))
          .with_child((
            Text::new(format!("{name} ({gx},{gy})")),
            TextFont { font_size: 10.0, ..default() },
            TextColor(Color::srgb(0.3, 0.9, 1.0))
          ));
      }
    });
  }
}

fn marker_list_clicks(
  interaction_q: Query<(&Interaction, &MarkerListButton), Changed<Interaction>>,
  origin: Res<CanvasGridOrigin>,
  mut camera_tf: Single<&mut Transform, With<Camera2d>>
) {
  for (interaction, btn) in &interaction_q {
    if *interaction == Interaction::Pressed {
      let world = grid_to_world(btn.0, btn.1, *origin);
      camera_tf.translation.x = world.x;
      camera_tf.translation.y = world.y;
    }
  }
}

// ---------------------------------------------------------------------------
// Window title (shows mode/tool/tile/object info)
// ---------------------------------------------------------------------------

fn update_title(state: Res<EditorState>, mut win: Single<&mut Window>) {
  if state.is_changed() {
    win.title = format!("Level Editor | {}", state.tool.name());
  }
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

fn main() {
  App::new()
    .add_plugins(DefaultPlugins.set(ImagePlugin::default_nearest()).set(WindowPlugin {
      primary_window: Some(Window {
        title: "Level Editor".into(),
        resolution: (1600, 900).into(),
        ..default()
      }),
      ..default()
    }))
    .insert_resource(ClearColor(Color::srgb(0.08, 0.08, 0.1)))
    .insert_resource(EditorCanvas {
      tiles: vec![vec![Tile::Grass; INITIAL_CANVAS_W]; INITIAL_CANVAS_H],
      objects: vec![vec![None; INITIAL_CANVAS_W]; INITIAL_CANVAS_H],
      markers: vec![vec![None; INITIAL_CANVAS_W]; INITIAL_CANVAS_H]
    })
    .insert_resource(SpawnedCanvasSize {
      width: INITIAL_CANVAS_W,
      height: INITIAL_CANVAS_H,
      origin_x: 0,
      origin_y: 0
    })
    .insert_resource(CanvasGridOrigin { x: 0, y: 0 })
    .insert_resource(EditorState {
      tool: ToolMode::Draw,
      selected_tile: Tile::Wall,
      selected_object: None,
      drag_start: None,
      paste_drag_offset: None,
      clipboard: None,
      pattern_size: DEFAULT_PATTERN_SIZE,
      output_mult: 1
    })
    .insert_resource(CameraZoom(1.0))
    .insert_resource(PanState {
      active: false,
      cursor_origin: Vec2::ZERO,
      camera_origin: Vec3::ZERO
    })
    .insert_resource(ResizeHoldState { active: None, held_for: 0.0, repeat_accum: 0.0 })
    .insert_resource(LoadPickerState { open: false, refresh_requested: false })
    .insert_resource(SaveNameInput { text: String::new(), focused: false })
    .insert_resource(MarkerInput { text: String::new(), focused: false })
    .insert_resource(MarkerListState { open: false, needs_refresh: false })
    .insert_resource(UndoStack(Vec::new()))
    .init_resource::<sprites::PaletteImageCache>()
    .add_plugins(haalka::HaalkaPlugin::default())
    .add_systems(Startup, (setup, spawn_editor_ui).chain())
    .add_systems(Update, (camera_pan, camera_zoom, tool_keys, ui_tile_palette))
    .add_systems(Update, (ui_tile_highlight, ui_object_palette, ui_object_highlight))
    .add_systems(
      Update,
      (
        update_tile_preview,
        update_mode_bar,
        save_name_input_focus,
        save_name_input_unfocus,
        save_name_input_typing,
        save_name_input_display,
        save_load_ui_actions,
        load_picker_visibility
      )
    )
    .add_systems(Update, (refresh_load_picker_list, load_picker_file_clicks))
    .add_systems(
      Update,
      (
        save_load_hotkeys,
        eyedropper,
        resize_canvas_with_edge_buttons,
        canvas_interact,
        undo_key,
        ensure_canvas_entities,
        sync_canvas_positions,
        sync_canvas_sprites
      )
    )
    .add_systems(Update, (sync_object_visuals, sync_marker_visuals, update_overlays, generate_wfc))
    .add_systems(
      Update,
      (
        marker_input_focus,
        marker_input_unfocus,
        marker_input_typing,
        marker_input_display,
        marker_list_toggle,
        marker_list_visibility,
        refresh_marker_list,
        marker_list_clicks
      )
    )
    .add_systems(Update, (export_prefab, update_title))
    .run();
}
