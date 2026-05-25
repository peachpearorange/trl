#![warn(dead_code)]
#![warn(unused_imports)]
mod abilities;
pub mod active_zone;
pub mod atmosphere;
mod combat;
mod crafting;
pub mod crew;
pub mod docking;
pub mod entities;
pub mod faction;
pub mod galaxy;
pub mod level;
mod locations;
mod loot;
pub mod navigation;
mod npcs;
mod outline;
mod particles;
mod path_overlay;
mod post_process;
pub mod prefabs;
pub mod ship;
pub mod sprites;
pub mod tiles;
mod ui;
mod utils;

use {crate::entities::{AirlockDoor, Bed, BlocksSight, Collidable, CraftingTable,
                       Dialogue, DialogueNode, DialogueTree, Door, Elevator, Enemy,
                       FixedChestLoot, FlightConsole, FollowerData, FollowerState,
                       Glyph, Loadout, LoadoutConsole, Location, LootChest, Named,
                       Stats, Tree, Visuals, WalkAnim},
     active_zone::ActiveZone,
     bevy::{camera::visibility::RenderLayers,
            prelude::*,
            sprite_render::{AlphaMode2d, TileData, TilemapChunk, TilemapChunkTileData}},
     combat::{FlowField, TileEntityIndex, compute_flow_field, damage_cloud_tick,
              enemy_ai, enemy_stealth_ai, follower_ai, grenade_thrower_ai,
              gun_attacker_ai, maintain_tile_index, mushroom_spore_attack, npc_wander,
              tick_grabbed, tick_grenade_in_flight, tick_invisible, tick_phasing},
     level::{FovGrid, Item, LocationType, Tile, compute_fov},
     sprites::{PaletteImageCache, palette_sprite_handle},
     std::collections::{HashMap, HashSet},
     ui::{LogEntries, LogSpan, MenuClickPending, log_message, log_spans}};

/// Tile art is authored at this resolution (e.g. space_qud masks).
pub const SPRITE_TEXELS: f32 = 20.0;
/// Each source pixel is drawn as this many screen pixels (integer scale).
pub const SCREEN_PIXELS_PER_TEXEL: f32 = 2.0;
/// World-space size of one grid cell (`Sprite` quad). Pixel-perfect when camera maps 1 world unit ≈ 1 screen pixel.
pub const TILE_SIZE: f32 = SPRITE_TEXELS * SCREEN_PIXELS_PER_TEXEL;

pub const fn id<T>(x: T) -> T { x }
pub const fn compose<A, B, C, F, G>(f: F, g: G) -> impl Fn(A) -> C
where
  F: Fn(B) -> C,
  G: Fn(A) -> B
{
  move |x| f(g(x))
}

const fn add_one_i32(x: i32) -> i32 { x + 1 }

const fn double_i32(x: i32) -> i32 { x * 2 }

pub const COMPOSED_CLOSURE: &dyn Fn(i32) -> i32 = &compose(add_one_i32, double_i32);
pub const COMPOSED_DYN: &dyn Fn(i32) -> i32 = COMPOSED_CLOSURE;
/// Palette-mask doors (`door closed (1).png` / `door open (2).png`).
const DOOR_CLOSED_PRI: Color = Color::srgb(0.34, 0.37, 0.41);
const DOOR_CLOSED_SEC: Color = Color::srgb(0.52, 0.55, 0.58);
const DOOR_OPEN_PRI: Color = Color::srgb(0.48, 0.55, 0.58);
const DOOR_OPEN_SEC: Color = Color::srgb(0.72, 0.78, 0.82);
/// Palette-mask airlocks (`airlock closed.png` / `airlock open.png`).
pub(crate) const AIRLOCK_PRI: Color = Color::srgb(0.58, 0.61, 0.64);
pub(crate) const AIRLOCK_SEC: Color = Color::srgb(0.52, 0.55, 0.58);
/// Primary color used for the player sprite and "You:" log labels.
pub const PLAYER_PRIMARY: Color = Color::srgb(0.72, 0.72, 0.72);
/// Simulated 60Hz display: one grid step / one input gate spans this many render updates.
pub const RENDER_FRAMES_PER_SIM_STEP: u32 = 8;
/// How many sim steps run per real-time second (= assumed display Hz / render frames per step).
pub const SIM_STEPS_PER_SEC: f32 = 60.0 / RENDER_FRAMES_PER_SIM_STEP as f32;
const FOV_RADIUS: i32 = 18;
/// Haalka layout: game view is left of the sidebar (`GAME_VIEWPORT_WIDTH_FRAC`); sidebar is
/// `SIDEBAR_WIDTH_FRAC`. Status bar is `STATUS_BAR_HEIGHT` along the bottom.
pub const GAME_VIEWPORT_WIDTH_FRAC: f32 = 0.70;
pub const SIDEBAR_WIDTH_FRAC: f32 = 1.0 - GAME_VIEWPORT_WIDTH_FRAC;
pub const STATUS_BAR_HEIGHT: f32 = 32.0;

// ---------------------------------------------------------------------------
// Player actions
// ---------------------------------------------------------------------------

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
  Talk { speaker: &'static str, tree: &'static DialogueTree, speaker_color: Color },
  ChopTree(Entity),
  PickUpItem(i32, i32),
  OpenChest(Entity),
  Navigate { dest: galaxy::LocationId },
  OpenCraftingTable,
  Salvage(Item),
  Craft(usize),
  EquipItem(Item),
  UnequipItem(Item),
  ShowLoadoutStatus,
  TakeElevator { dest_z: usize, dest_x: i32, dest_y: i32 },
  RecruitFollower { entity: Entity, name: &'static str },
  DismissFollower { entity: Entity, name: &'static str },
  SaveAtBed
}

/// Tracks which menu row index a [`Button`] entity belongs to; queried by [`detect_menu_option_clicks`].
#[derive(Component)]
pub struct MenuOptionIndex(pub usize);

#[derive(Default)]
pub enum InteractMenu {
  #[default]
  Closed,
  Open {
    options: Vec<InteractionOption>,
    selected: usize,
    highlighted: Vec<bool>,
    disabled: Vec<bool>
  }
}

#[derive(Default)]
pub enum CraftingMenu {
  #[default]
  Closed,
  Open {
    tab: usize,
    selected: usize,
    scroll: usize,
    salvage_actions: Vec<InteractionAction>,
    craft_actions: Vec<InteractionAction>,
    salvage_entries: Vec<ui::CraftingEntry>,
    craft_entries: Vec<ui::CraftingEntry>
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
    node_name: &'static str,
    speaker_color: Color
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
  crafting: CraftingMenu,
  dialogue: DialogueState,
  /// Set by `handle_dialogue`/`handle_menus` when Space closes a menu; read+cleared by
  /// `handle_interact` so the same keypress doesn't also open an interaction.
  space_consumed: bool,
  /// Set by `handle_menus` when a direction key (W/S/A/D/Enter) is consumed by menu navigation
  /// or confirmation; cleared + checked by `accumulate_dir`/`player_input` to prevent that
  /// keypress from also moving the player.
  dir_consumed: bool,
  /// Key-repeat state for W/S popup menu scrolling: (-1/1/0) direction + frames countdown.
  menu_nav_dir: i8,
  menu_nav_frames: u32
}

impl UiState {
  fn any_open(&self) -> bool {
    self.pause != PauseMenu::Closed
      || matches!(self.interact, InteractMenu::Open { .. })
      || matches!(self.crafting, CraftingMenu::Open { .. })
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
  pub mode: TimeMode
}

/// When `true`, [`update_time_mode`] may switch to turn-based when enemies are near.
/// `T` sets a manual mode and sets this to `false`; `Shift+T` restores auto.
#[derive(Resource)]
pub struct TimeModeAuto(pub bool);

/// Latches direction key presses between move ticks so a tap that lands between ticks isn't lost.
#[derive(Resource, Default)]
pub struct AccumulatedDir {
  pub up: bool,
  pub down: bool,
  pub left: bool,
  pub right: bool
}

impl Clock {
  fn new() -> Self {
    Clock { time: 0, mode: TimeMode::TurnBased }
  }

  fn spend_turn(&mut self, tb: &mut TurnBasedWorldState) {
    self.time = self.time.saturating_add(1);
    if self.mode == TimeMode::TurnBased {
      tb.world_tick_pending = true;
    }
  }
}

/// In turn-based mode, the world only advances in [`SimStep`] after a player spends a turn.
/// Cleared at the end of [`combat::enemy_ai`] once all world systems have run.
#[derive(Resource, Default)]
pub struct TurnBasedWorldState {
  pub world_tick_pending: bool
}

/// Filled by [`player_input`] when a move is blocked; [`resolve_bump_interact`] reads it the same frame.
#[derive(Resource, Default)]
struct PendingBumpInteract(pub Option<(i32, i32, usize)>, pub Option<(Entity, Vec2)>);

/// Deferred actions set by menu selection, applied next frame by dedicated systems.
#[derive(Resource, Default)]
struct DeferredActions {
  pub loot_chest: Option<Entity>,
  pub navigate: Option<galaxy::LocationId>,
  /// Position to teleport the player to after navigation completes (used by death respawn).
  pub post_navigate_pos: Option<(i32, i32, usize)>,
  pub save_at_bed: bool
}

/// Snapshot of player state taken when sleeping in a bed.
#[derive(Resource, Default)]
struct BedSave(Option<SaveData>);

struct SaveData {
  docked_at: Option<galaxy::LocationId>,
  pos: (i32, i32, usize),
  inventory: HashMap<Item, u32>,
  loadout: Loadout
}

/// Single interaction chosen after bumping a blocked tile ([`resolve_bump_interact`] → [`apply_bump_auto_interact`]).
#[derive(Resource, Default)]
struct BumpInteractFlash(pub Option<InteractionOption>);

fn bump_render_frame(mut frame: ResMut<RenderFrame>, mut clock: ResMut<Clock>) {
  frame.0 = frame.0.saturating_add(1);
  if clock.mode == TimeMode::RealTime
    && frame.0 > 0
    && frame.0 % u64::from(RENDER_FRAMES_PER_SIM_STEP) == 0
  {
    clock.time = clock.time.saturating_add(1);
  }
}

fn is_sim_frame(frame: Res<RenderFrame>) -> bool {
  frame.0 > 0 && frame.0 % u64::from(RENDER_FRAMES_PER_SIM_STEP) == 0
}

fn should_run_sim_step(
  frame: Res<RenderFrame>,
  clock: Res<Clock>,
  tb: Res<TurnBasedWorldState>
) -> bool {
  frame.0 > 0
    && frame.0 % u64::from(RENDER_FRAMES_PER_SIM_STEP) == 0
    && (clock.mode == TimeMode::RealTime || tb.world_tick_pending)
}


// ---------------------------------------------------------------------------
#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
struct EveryFrame;

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
struct EveryFrameUi;

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
struct SimFrame;

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
struct WorldStep;

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
struct Render;

// ---------------------------------------------------------------------------
// Resources & components
// ---------------------------------------------------------------------------

#[derive(Resource)]
pub struct CurrentZone(pub active_zone::ActiveZone);

#[derive(Resource)]
pub struct Fov(pub FovGrid);

#[derive(Component)]
pub struct Player;

/// Marks which z-level a tilemap chunk belongs to.
#[derive(Component)]
struct TilemapLayer(usize);

/// Stores the baked tileset and its per-tile layer-range index.
#[derive(Resource)]
struct Tileset(sprites::TilesetInfo);

#[derive(Component)]
struct ItemGlyph {
  x: usize,
  y: usize,
  z: usize,
  item: level::Item
}

#[derive(Component)]
struct LootDrop {
  from: Vec2,
  to: Vec2,
  start_frame: u64,
  duration_frames: u64
}

#[derive(Component)]
struct DeathShrink {
  start_frame: u64,
  frames_per_texel: u64,
  total_texels: u64,
  prev_texels: u64,
  source: image::RgbaImage
}

#[derive(Component, Default)]
pub struct Inventory(pub HashMap<Item, u32>);

/// Marker for entities that use [`Glyph`] visuals (tile sprite or [`Text2d`]).
#[derive(Component)]
struct GlyphVisual;

/// Semi-transparent cell highlight following the cursor over the current zone.
#[derive(Component)]
struct TileHoverHighlight;

#[derive(Component)]
pub struct BumpLunge {
  dir: Vec2,
  start_frame: u64
}

// ---------------------------------------------------------------------------
// Glyph rendering systems
// ---------------------------------------------------------------------------

fn apply_visuals_move(vis: &mut Visuals, f: u64, local: Vec2) {
  if (local - vis.last_pos).length_squared() > 0.5 {
    vis.prev = vis.display;
    vis.last_move_start_frame = Some(f);
    vis.last_pos = local;
  }
}

/// After movement systems run, snapshot position changes into Visuals.
/// When an entity's Location changes, `prev` snaps to the current display pos
/// (so direction changes pivot smoothly) and the move timer resets.
fn track_movement(
  frame: Res<RenderFrame>,
  mut query: Query<(&Location, &mut Visuals)>
) {
  let f = frame.0;
  for (loc, mut vis) in query.iter_mut() {
    if let Some(world_pos) = loc.as_vec2() {
      let local = Vec2::new(world_pos.x, world_pos.y);
      apply_visuals_move(&mut vis, f, local);
    }
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
  mut query: Query<(&Location, &mut Visuals)>
) {
  let f = frame.0;
  for (loc, mut vis) in query.iter_mut() {
    if let Some(world_pos) = loc.as_vec2() {
      let local = Vec2::new(world_pos.x, world_pos.y);
      interpolate_visual_one(&mut vis, f, local);
    }
  }
}

fn apply_bump_lunge(
  frame: Res<RenderFrame>,
  mut commands: Commands,
  mut query: Query<(Entity, &BumpLunge, &mut Visuals)>
) {
  let n = u64::from(RENDER_FRAMES_PER_SIM_STEP);
  for (entity, lunge, mut vis) in query.iter_mut() {
    let elapsed = frame.0.saturating_sub(lunge.start_frame);
    if elapsed >= n {
      commands.entity(entity).remove::<BumpLunge>();
    } else {
      let half = n as f32 / 2.0;
      let peak = 0.5;
      let offset = if (elapsed as f32) < half {
        peak * (elapsed + 1) as f32 / half
      } else {
        peak * (n - elapsed) as f32 / half
      };
      vis.display += lunge.dir * offset;
    }
  }
}

fn setup_glyph_visuals(
  mut commands: Commands,
  asset_server: Res<AssetServer>,
  mut images: ResMut<Assets<Image>>,
  mut palette_cache: ResMut<PaletteImageCache>,
  current: Res<CurrentZone>,
  query: Query<(Entity, &Glyph, &Location), (Added<Glyph>, Without<GlyphVisual>)>
) {
  for (entity, glyph, location) in query.iter() {
    if let Location::Coords { x, y, .. } = location {
      let lx = *x;
      let ly = *y;
      let local = Vec2::new(lx as f32, ly as f32);
      let pos = tile_screen_pos(lx as f32, ly as f32, current.0.width, current.0.height)
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
          RenderLayers::layer(post_process::LAYER_ENTITIES),
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
          RenderLayers::layer(post_process::LAYER_ENTITIES),
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
  current: Res<CurrentZone>,
  mut query: Query<(&Visuals, &mut Transform), With<GlyphVisual>>
) {
  let (w, h) = (current.0.width, current.0.height);
  for (vis, mut transform) in query.iter_mut() {
    transform.translation =
      tile_screen_pos(vis.display.x, vis.display.y, w, h) + Vec3::new(0.0, 0.0, 2.0);
  }
}

fn animate_walk_sprites(
  frame: Res<RenderFrame>,
  keys: Res<ButtonInput<KeyCode>>,
  mut palette_cache: ResMut<PaletteImageCache>,
  mut images: ResMut<Assets<Image>>,
  mut query: Query<(&WalkAnim, &Glyph, &mut Sprite)>
) {
  let move_held = keys.any_pressed([
    KeyCode::KeyW,
    KeyCode::KeyA,
    KeyCode::KeyS,
    KeyCode::KeyD,
    KeyCode::ArrowUp,
    KeyCode::ArrowDown,
    KeyCode::ArrowLeft,
    KeyCode::ArrowRight
  ]);
  for (anim, glyph, mut sprite) in query.iter_mut() {
    let (frames, interval) = if move_held {
      (anim.walk_frames, anim.interval)
    } else {
      (anim.idle_frames, anim.idle_interval)
    };
    let path = if frames.is_empty() {
      anim.idle
    } else {
      let step = (frame.0 / interval) as usize;
      let n = frames.len() * 2;
      let phase = step % n;
      if phase % 2 == 0 { anim.idle } else { frames[phase / 2] }
    };
    if let Some((primary, secondary)) = glyph.sprite_palette {
      sprite.image =
        palette_sprite_handle(path, primary, secondary, &mut palette_cache, &mut images);
    }
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
  let origin: galaxy::LocationId = locations::starter_planet::ID;
  let starter_planet = locations::starter_planet::generate();
  galaxy.insert(origin, starter_planet.clone());
  galaxy.insert(locations::asteroid_field::ID, locations::asteroid_field::generate());
  galaxy.insert(locations::meridian_station::ID, locations::meridian_station::generate());
  galaxy.insert(locations::lava_planet::ID, locations::lava_planet::generate());
  galaxy.insert(locations::mushroom_planet::ID, locations::mushroom_planet::generate());
  galaxy.insert(locations::gamma_station::ID, locations::gamma_station::generate());
  for (id, name) in locations::planet_gen::all_ids() {
    galaxy.register_deferred(id, name, |id| locations::planet_gen::generate_by_id(id));
  }
  for (id, loc) in locations::station_gen::all() {
    galaxy.insert(id, loc);
  }

  // Ship starts docked at the starter planet
  let active = active_zone::ActiveZone::docked(&ship_location, &starter_planet)
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
    .insert_resource(TimeModeAuto(false))
    .init_resource::<DeferredActions>()
    .init_resource::<BedSave>()
    .init_resource::<BumpInteractFlash>()
    .init_resource::<PendingBumpInteract>()
    .init_resource::<PaletteImageCache>()
    .init_resource::<AccumulatedDir>()
    .insert_resource(UiState::default())
    .insert_resource(Fov(fov))
    .insert_resource(TileEntityIndex::default())
    .init_resource::<FlowField>()
    .init_resource::<abilities::AbilityBarData>()
    .init_resource::<abilities::TargetingState>()
    .init_resource::<path_overlay::RangedPathOverlay>()
    .add_plugins(ui::UiPlugin)
    .add_plugins(particles::ParticlesPlugin)
    .add_plugins(post_process::PostProcessPlugin)
    .add_plugins(outline::OutlinePlugin)
    .add_systems(Startup, (setup, ui::spawn_haalka_root).chain())
    .add_systems(PostStartup, (update_fov, init_follower_homes).chain())
    // --- every frame: world bookkeeping ---
    .add_systems(
      Update,
      (
        apply_pending_navigation,
        bump_render_frame,
        setup_glyph_visuals,
        materialize_ground_items,
        update_time_mode
      )
        .chain()
        .in_set(EveryFrame)
    )
    // --- every frame: input / UI ---
    .add_systems(
      Update,
      (
        handle_dialogue,
        detect_menu_option_clicks,
        handle_menus,
        handle_crafting_menu,
        flush_pending_loot,
        apply_bed_save,
        handle_interact,
        handle_utility_menus,
        abilities::handle_ability_keys,
        abilities::handle_ability_click,
        abilities::detect_ability_bar_clicks,
        abilities::handle_ability_scroll,
        path_overlay::update_ranged_path,
        path_overlay::render_ranged_path
      )
        .chain()
        .in_set(EveryFrameUi)
    )
    // --- sim frames only: player turn ---
    .add_systems(
      Update,
      (
        maintain_tile_index,
        accumulate_dir,
        player_input,
        resolve_bump_interact,
        apply_bump_auto_interact,
        abilities::sync_ability_bar,
        auto_close_airlocks,
        update_fov
      )
        .chain()
        .run_if(is_sim_frame)
        .in_set(SimFrame)
    )
    // --- sim frames only: world response (turn-based: only when player acted) ---
    .add_systems(
      Update,
      (
        ApplyDeferred,
        enemy_death_check,
        compute_flow_field,
        enemy_ai,
        mushroom_spore_attack,
        grenade_thrower_ai,
        gun_attacker_ai,
        enemy_stealth_ai,
        tick_grabbed,
        tick_invisible,
        tick_phasing,
        tick_grenade_in_flight,
        damage_cloud_tick,
        player_death_check,
        npc_wander,
        follower_ai,
        abilities::tick_cooldowns,
        abilities::advance_pending_fire
      )
        .chain()
        .run_if(should_run_sim_step)
        .in_set(WorldStep)
    )
    // --- every frame: visuals ---
    .add_systems(
      Update,
      (
        particles::liquid_splash_on_move,
        track_movement,
        interpolate_visual_positions,
        apply_bump_lunge,
        sync_entity_positions,
        animate_walk_sprites,
        animate_loot_drops,
        animate_death_shrink,
        apply_invisible_alpha,
        camera_follow,
        update_fov_visuals,
        update_tile_hover_highlight,
        update_interactable_highlights
      )
        .chain()
        .in_set(Render)
    )
    .configure_sets(Update, (EveryFrame, EveryFrameUi, SimFrame, WorldStep, Render).chain())
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
  let tx = (world.x / TILE_SIZE + w as f32 * 0.5 + 0.5).floor() as i32;
  let ty = (h as f32 * 0.5 - world.y / TILE_SIZE + 0.5).floor() as i32;
  (tx, ty)
}

// ---------------------------------------------------------------------------
// Gravity
// ---------------------------------------------------------------------------

fn scatter_loot_tiles(
  ex: i32,
  ey: i32,
  level: &level::Level,
  count: usize
) -> Vec<(i32, i32)> {
  use rand::seq::SliceRandom;
  let mut tiles = Vec::with_capacity(count);
  let mut candidates = [
    (ex - 1, ey),
    (ex + 1, ey),
    (ex, ey - 1),
    (ex, ey + 1),
    (ex - 1, ey - 1),
    (ex + 1, ey - 1),
    (ex - 1, ey + 1),
    (ex + 1, ey + 1),
    (ex, ey)
  ];
  candidates.shuffle(&mut rand::thread_rng());
  for &(cx, cy) in &candidates {
    if tiles.len() >= count {
      break;
    }
    if level.walkable(cx, cy)
      && cx >= 0
      && cy >= 0
      && (cx as usize) < level.width
      && (cy as usize) < level.height
      && !tiles.contains(&(cx, cy))
    {
      tiles.push((cx, cy));
    }
  }
  tiles
}

fn enemy_death_check(
  mut commands: Commands,
  mut current: ResMut<CurrentZone>,
  frame: Res<RenderFrame>,
  mut palette_cache: ResMut<PaletteImageCache>,
  mut images: ResMut<Assets<Image>>,
  enemy_q: Query<
    (Entity, &Stats, &Loadout, &Location, Option<&Named>, &Glyph),
    With<Enemy>
  >
) {
  for (entity, stats, loadout, location, _, glyph) in enemy_q.iter() {
    if stats.hp <= 0 {
      let loot = loadout.lootable_items();
      let (ex, ey, ez) = if let Location::Coords { x, y, z, .. } = *location {
        (x, y, z)
      } else {
        commands.entity(entity).despawn();
        continue;
      };

      let (w, h) = (current.0.width, current.0.height);
      let level = current.0.level_mut(ez);
      let drop_tiles = scatter_loot_tiles(ex, ey, level, loot.len());

      for (i, &(item, _qty)) in loot.iter().enumerate() {
        if let Some(&(tx, ty)) = drop_tiles.get(i) {
          let (primary, secondary) = item.loot_colors();
          let img = palette_sprite_handle(
            item.loot_texture(),
            primary,
            secondary,
            &mut palette_cache,
            &mut images
          );
          commands.spawn((
            Sprite { image: img, custom_size: Some(Vec2::splat(TILE_SIZE)), ..default() },
            Transform::from_translation(
              tile_screen_pos(ex as f32, ey as f32, w, h) + Vec3::new(0.0, 0.0, 5.0)
            ),
            LootDrop {
              from: Vec2::new(ex as f32, ey as f32),
              to: Vec2::new(tx as f32, ty as f32),
              start_frame: frame.0,
              duration_frames: 12
            },
            ItemGlyph { x: tx as usize, y: ty as usize, z: ez, item },
            RenderLayers::layer(post_process::LAYER_ENTITIES)
          ));
        }
      }

      let source = glyph.texture.map(|path| {
        let (primary, secondary) =
          glyph.sprite_palette.unwrap_or((glyph.color, glyph.color));
        sprites::bake_palette_rgba(path, primary, secondary)
      });
      if let Some(source) = source {
        commands
          .entity(entity)
          .remove::<(
            Enemy,
            Stats,
            Loadout,
            entities::Character,
            entities::FactionComp,
            entities::Gravity,
            entities::TimeSinceAction,
            entities::Path,
            entities::DriftChance,
            entities::Invisible,
            Glyph,
            GlyphVisual,
            WalkAnim
          )>()
          .insert((Collidable(false), DeathShrink {
            start_frame: frame.0,
            frames_per_texel: 6,
            total_texels: SPRITE_TEXELS as u64,
            prev_texels: SPRITE_TEXELS as u64,
            source
          }));
      } else {
        commands.entity(entity).despawn();
      }
    }
  }
}

fn animate_loot_drops(
  mut commands: Commands,
  frame: Res<RenderFrame>,
  current: Res<CurrentZone>,
  mut query: Query<(Entity, &LootDrop, &mut Transform, &mut Sprite)>
) {
  let (w, h) = (current.0.width, current.0.height);
  for (entity, drop, mut tf, mut sprite) in query.iter_mut() {
    let elapsed = frame.0.saturating_sub(drop.start_frame);
    if elapsed >= drop.duration_frames {
      tf.translation =
        tile_screen_pos(drop.to.x, drop.to.y, w, h) + Vec3::new(0.0, 0.0, 1.5);
      sprite.color = Color::WHITE;
      commands.entity(entity).remove::<LootDrop>();
    } else {
      let t = (elapsed + 1) as f32 / drop.duration_frames as f32;
      let pos = drop.from.lerp(drop.to, t);
      let arc = 1.5 * (t * std::f32::consts::PI).sin();
      tf.translation =
        tile_screen_pos(pos.x, pos.y, w, h) + Vec3::new(0.0, arc * TILE_SIZE, 5.0);
      sprite.color = Color::srgba(1.0, 1.0, 1.0, (t * 2.0).min(1.0));
    }
  }
}

fn animate_death_shrink(
  mut commands: Commands,
  frame: Res<RenderFrame>,
  mut images: ResMut<Assets<Image>>,
  mut query: Query<(Entity, &mut DeathShrink, &mut Sprite)>
) {
  use bevy::{asset::RenderAssetUsages,
             render::render_resource::{Extent3d, TextureDimension, TextureFormat}};
  for (entity, mut shrink, mut sprite) in query.iter_mut() {
    let elapsed = frame.0.saturating_sub(shrink.start_frame);
    let texels_lost = elapsed / shrink.frames_per_texel;
    if texels_lost >= shrink.total_texels {
      commands.entity(entity).despawn();
    } else {
      let remaining = shrink.total_texels - texels_lost;
      if remaining != shrink.prev_texels {
        shrink.prev_texels = remaining;
        let n = remaining as u32;
        let resized = image::imageops::resize(
          &shrink.source,
          n,
          n,
          image::imageops::FilterType::Nearest
        );
        let handle = images.add(Image::new(
          Extent3d { width: n, height: n, depth_or_array_layers: 1 },
          TextureDimension::D2,
          resized.into_raw(),
          TextureFormat::Rgba8UnormSrgb,
          RenderAssetUsages::RENDER_WORLD
        ));
        sprite.image = handle;
        sprite.custom_size = Some(Vec2::splat(n as f32 * SCREEN_PIXELS_PER_TEXEL));
      }
    }
  }
}

fn apply_invisible_alpha(
  mut query: Query<(&mut Sprite, Option<&entities::Invisible>), With<GlyphVisual>>
) {
  for (mut sprite, invis) in query.iter_mut() {
    let target = if invis.is_some() { 0.25 } else { 1.0 };
    if (sprite.color.alpha() - target).abs() > 0.01 {
      sprite.color.set_alpha(target);
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
  window: Single<&Window>,
  camera: Single<(&Camera, &GlobalTransform), With<post_process::GameCamera>>,
  current: Res<CurrentZone>,
  fov: Res<Fov>,
  player_pos: Single<&Location, With<Player>>,
  mut q: Query<(&mut Transform, &mut Visibility), With<TileHoverHighlight>>
) {
  if let Ok((mut transform, mut vis)) = q.single_mut()
    && let &Location::Coords { z, .. } = &**player_pos
  {
    *vis = Visibility::Hidden;
    let (camera, cam_transform) = *camera;
    {
      let level = current.0.level(z);
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
        pick(&*window, camera, cam_transform, level.width, level.height)
      {
        let visible = fov.0.is_visible(tx as usize, ty as usize);
        if visible {
          *vis = Visibility::Visible;
          transform.translation =
            tile_screen_pos(tx as f32, ty as f32, current.0.width, current.0.height)
              + Vec3::new(0.0, 0.0, 0.25);
        }
      }
    }
  }
}

fn has_interaction(
  entity: Entity,
  interact_q: &Query<(
    Option<&Tree>,
    Option<&Dialogue>,
    Option<&Door>,
    Option<&Elevator>,
    Option<&FlightConsole>,
    Option<&LoadoutConsole>,
    Option<&CraftingTable>,
    Option<&FollowerState>,
    Option<&Bed>
  )>,
  loot_q: &Query<&LootChest>
) -> bool {
  interact_q.get(entity).is_ok_and(
    |(tree, dialogue, door, elevator, flight, loadout, craft, follower, bed)| {
      tree.is_some()
        || dialogue.is_some()
        || door.is_some()
        || elevator.is_some()
        || flight.is_some()
        || loadout.is_some()
        || craft.is_some()
        || follower.is_some()
        || bed.is_some()
    }
  ) || loot_q.get(entity).is_ok_and(|c| !c.opened)
}

fn update_interactable_highlights(
  mut commands: Commands,
  frame: Res<RenderFrame>,
  current: Res<CurrentZone>,
  index: Res<TileEntityIndex>,
  fov: Res<Fov>,
  player_pos: Single<&Location, With<Player>>,
  ui: Res<UiState>,
  interact_q: Query<(
    Option<&Tree>,
    Option<&Dialogue>,
    Option<&Door>,
    Option<&Elevator>,
    Option<&FlightConsole>,
    Option<&LoadoutConsole>,
    Option<&CraftingTable>,
    Option<&FollowerState>,
    Option<&Bed>
  )>,
  loot_q: Query<&LootChest>,
  sprite_q: Query<(&Sprite, &Transform, &Visibility), Without<outline::InteractOutline>>,
  pool: Res<outline::OutlinePool>,
  mut outline_q: Query<(&mut Transform, &mut Visibility), With<outline::InteractOutline>>,
  mut outline_mats: ResMut<Assets<outline::OutlineMaterial>>
) {
  let any_menu_open = ui.any_open();
  let t = frame.0 as f32 * 0.08;
  let pulse = (t.sin() * 0.5 + 0.5) * 0.55 + 0.45;
  let outline_color = LinearRgba::new(0.95, 0.85, 0.2, pulse);

  let mut targets: Vec<(Vec3, Handle<Image>)> = Vec::new();

  if !any_menu_open
    && let &Location::Coords { x: px, y: py, z, .. } = &**player_pos
  {
    for dy in -1..=1i32 {
      for dx in -1..=1i32 {
        let wx = px + dx;
        let wy = py + dy;
        if wx < 0
          || wy < 0
          || (wx as usize) >= current.0.width
          || (wy as usize) >= current.0.height
        {
          continue;
        }
        if !fov.0.is_visible(wx as usize, wy as usize) {
          continue;
        }
        if let Some(ents) = index.0.get(&(wx, wy, z)) {
          for &e in ents {
            if has_interaction(e, &interact_q, &loot_q)
              && let Ok((sprite, entity_tf, vis)) = sprite_q.get(e)
              && *vis != Visibility::Hidden
              && sprite.image != Handle::default()
            {
              let pos =
                entity_tf.translation.truncate().extend(entity_tf.translation.z - 0.1);
              targets.push((pos, sprite.image.clone()));
            }
          }
        }
      }
    }
  }

  for (i, &ent) in pool.entities.iter().enumerate() {
    if let Ok((mut tf, mut vis)) = outline_q.get_mut(ent) {
      if let Some((pos, tex)) = targets.get(i) {
        *vis = Visibility::Visible;
        tf.translation = *pos;
        let mat = outline_mats
          .add(outline::OutlineMaterial { texture: tex.clone(), color: outline_color });
        commands.entity(ent).insert(MeshMaterial2d(mat));
      } else {
        *vis = Visibility::Hidden;
      }
    }
  }
}

fn update_fov_visuals(
  fov: Res<Fov>,
  current: Res<CurrentZone>,
  frame: Res<RenderFrame>,
  tileset: Res<Tileset>,
  index: Res<TileEntityIndex>,
  player: Single<(Entity, &Location, &mut Visibility), With<Player>>,
  mut chunk_q: Query<
    (&TilemapLayer, &mut TilemapChunkTileData, &mut Visibility),
    Without<Player>
  >,
  mut item_q: Query<
    (Entity, &ItemGlyph, &mut Visibility),
    (Without<Player>, Without<GlyphVisual>, Without<TilemapLayer>)
  >,
  mut entity_q: Query<
    (Entity, &Location, &mut Visibility),
    (With<GlyphVisual>, Without<Player>, Without<TilemapLayer>)
  >
) {
  let (player_ent, pos, mut player_vis) = player.into_inner();
  let Location::Coords { x: pos_x, y: pos_y, z, .. } = *pos else { unreachable!() };
  let level = current.0.level(z);
  let width = current.0.width;
  let height = current.0.height;

  for (layer, mut tile_data, mut vis) in chunk_q.iter_mut() {
    *vis = if layer.0 == z { Visibility::Visible } else { Visibility::Hidden };
    let chunk_fresh = tile_data.0.iter().all(|t| t.is_none());
    if layer.0 == z && (fov.is_changed() || chunk_fresh) {
      for y in 0..height {
        for x in 0..width {
          let chunk_idx = (height - 1 - y) * width + x;
          let tile = level.get(x as i32, y as i32).unwrap_or(Tile::Air);
          tile_data.0[chunk_idx] = if tile == Tile::Air || tile == Tile::Blank {
            None
          } else {
            let info = tileset.0.layer_range[tile as usize];
            let tileset_index = match info.select {
              sprites::TileSelect::Single => info.base,
              sprites::TileSelect::RandomHash => {
                // SplitMix64 finalizer on (x, y) packed into 64 bits — no correlation between positions.
                let h: u64 = (x as u64) | ((y as u64) << 32);
                let h = (h ^ (h >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
                let h = (h ^ (h >> 27)).wrapping_mul(0x94d049bb133111eb);
                let h = h ^ (h >> 31);
                info.base + (h as u16) % info.count
              }
              sprites::TileSelect::Connected => {
                let same = |dx: i32, dy: i32| {
                  level.get(x as i32 + dx, y as i32 + dy) == Some(tile)
                };
                let mask = (same(0, -1) as u16)
                  | ((same(1, 0) as u16) << 1)
                  | ((same(0, 1) as u16) << 2)
                  | ((same(-1, 0) as u16) << 3);
                info.base + mask
              }
            };
            if fov.0.is_visible(x, y) {
              let color = if tile.is_liquid() {
                Color::srgba(1.0, 1.0, 1.0, 254.0 / 255.0)
              } else {
                Color::WHITE
              };
              Some(TileData { tileset_index, color, visible: true })
            } else {
              None
            }
          };
        }
      }
    }
  }

  let t = frame.0 as f32 * 0.052;
  let tau = std::f32::consts::TAU;
  let mut stacks: HashMap<(i32, i32), Vec<Entity>> = HashMap::new();
  stacks.entry((pos_x, pos_y)).or_default().push(player_ent);
  for (&(x, y, zz), ents) in index.0.iter() {
    if zz != z {
      continue;
    }
    stacks.entry((x, y)).or_default().extend(ents.iter().copied());
  }
  for (entity, item, _) in item_q.iter_mut() {
    if item.z == z {
      stacks.entry((item.x as i32, item.y as i32)).or_default().push(entity);
    }
  }
  for ents in stacks.values_mut() {
    ents.sort_by_key(|e| e.index());
  }

  for (entity, location, mut vis) in entity_q.iter_mut() {
    let visible_in_fov = if let Location::Coords { x, y, z: lz, .. } = location
      && *lz == z
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
  for (entity, item, mut vis) in item_q.iter_mut() {
    let visible_in_fov = item.z == z && fov.0.is_visible(item.x, item.y);
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
  let key = (pos_x, pos_y);
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

// ---------------------------------------------------------------------------
// Time
// ---------------------------------------------------------------------------

/// In real-time mode, one abstract time tick every [`RENDER_FRAMES_PER_SIM_STEP`] display frames.
const ENEMY_ALERT_RADIUS: i32 = 8;

fn update_time_mode(
  mut clock: ResMut<Clock>,
  time_mode_auto: Res<TimeModeAuto>,
  player_q: Query<&Location, With<Player>>,
  enemy_q: Query<&Location, (With<Enemy>, Without<Player>)>
) {
  if time_mode_auto.0 {
    let enemy_near = player_q.single().is_ok_and(|ploc| {
      if let Location::Coords { x: px, y: py, z: pz, .. } = *ploc {
        enemy_q.iter().any(|loc| {
          if let Location::Coords { x, y, z, .. } = *loc {
            z == pz
              && (x - px).abs() <= ENEMY_ALERT_RADIUS
              && (y - py).abs() <= ENEMY_ALERT_RADIUS
          } else {
            false
          }
        })
      } else {
        false
      }
    });
    clock.mode = if enemy_near { TimeMode::TurnBased } else { TimeMode::RealTime };
  }
}

// ---------------------------------------------------------------------------
// Input helpers
// ---------------------------------------------------------------------------

/// Runs every frame before [`player_input`]; latches any newly-pressed direction keys so a tap
/// that falls between move ticks is not silently dropped.
fn accumulate_dir(
  keys: Res<ButtonInput<KeyCode>>,
  ui: Res<UiState>,
  mut acc: ResMut<AccumulatedDir>
) {
  // If handle_menus consumed a direction key this frame for menu navigation/confirmation,
  // do not latch it — it must not bleed into player movement.
  if !ui.dir_consumed {
    if keys.just_pressed(KeyCode::KeyW) || keys.just_pressed(KeyCode::ArrowUp) {
      acc.up = true;
    }
    if keys.just_pressed(KeyCode::KeyS) || keys.just_pressed(KeyCode::ArrowDown) {
      acc.down = true;
    }
    if keys.just_pressed(KeyCode::KeyA) || keys.just_pressed(KeyCode::ArrowLeft) {
      acc.left = true;
    }
    if keys.just_pressed(KeyCode::KeyD) || keys.just_pressed(KeyCode::ArrowRight) {
      acc.right = true;
    }
  }
}

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

fn resolve_move(
  level: &level::Level,
  px: i32,
  py: i32,
  dx: i32,
  dy: i32,
  phasing: bool,
  entity_blocked: &impl Fn(i32, i32) -> bool
) -> (i32, i32) {
  let passable = |x, y| (phasing || level.walkable(x, y)) && !entity_blocked(x, y);
  if dx != 0 && dy != 0 {
    // Diagonal blocked only when both cardinal neighbours are impassable
    if passable(px + dx, py + dy) && (passable(px + dx, py) || passable(px, py + dy)) {
      (dx, dy)
    } else if passable(px + dx, py) {
      (dx, 0)
    } else if passable(px, py + dy) {
      (0, dy)
    } else {
      (0, 0)
    }
  } else if passable(px + dx, py + dy) {
    (dx, dy)
  } else {
    (0, 0)
  }
}

// ---------------------------------------------------------------------------
// Pause / Esc menu
// ---------------------------------------------------------------------------

fn door_glyph(open: bool, is_airlock: bool) -> Glyph {
  if is_airlock {
    if open {
      Glyph::palette_sprite(
        "textures/space_qud/airlock open.png",
        '/',
        AIRLOCK_PRI,
        AIRLOCK_SEC
      )
    } else {
      Glyph::palette_sprite(
        "textures/space_qud/airlock closed.png",
        '+',
        AIRLOCK_PRI,
        AIRLOCK_SEC
      )
    }
  } else if open {
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
  asset_server: &AssetServer,
  zone_w: usize,
  zone_h: usize
) {
  door.open = open;
  if open {
    commands.entity(entity).remove::<Collidable>();
    commands.entity(entity).remove::<BlocksSight>();
  } else {
    commands.entity(entity).insert((Collidable(true), BlocksSight));
  }
  let is_airlock = airlock.is_some();
  if let Some(airlock) = airlock {
    airlock.opened_at_sim_time = open.then_some(clock_time);
  }
  *glyph = door_glyph(open, is_airlock);
  if let Location::Coords { x, y, .. } = *location {
    let lx = x as f32;
    let ly = y as f32;
    let local = Vec2::new(lx, ly);
    let pos = tile_screen_pos(lx, ly, zone_w, zone_h) + Vec3::new(0.0, 0.0, 2.0);
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
        RenderLayers::layer(post_process::LAYER_ENTITIES),
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
        RenderLayers::layer(post_process::LAYER_ENTITIES),
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

fn detect_menu_option_clicks(
  button_q: Query<(&Interaction, &MenuOptionIndex), Changed<Interaction>>,
  mut pending: ResMut<MenuClickPending>,
  mut ui: ResMut<UiState>
) {
  for (interaction, idx) in &button_q {
    match *interaction {
      Interaction::Pressed => {
        pending.0 = Some(idx.0);
      }
      Interaction::Hovered => {
        if let InteractMenu::Open { ref mut selected, .. } = ui.interact {
          *selected = idx.0;
        }
      }
      Interaction::None => {}
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
  mut player_query: Query<(&mut Location, &mut Inventory, &mut Loadout), With<Player>>,
  asset_server: Res<AssetServer>,
  mut palette_cache: ResMut<PaletteImageCache>,
  mut images: ResMut<Assets<Image>>,
  mut door_q: Query<(
    &mut Door,
    &mut Glyph,
    Option<&mut Collidable>,
    &Location,
    Option<&mut AirlockDoor>
  ), Without<Player>>,
  mut deferred: ResMut<DeferredActions>,
  mut exit: MessageWriter<AppExit>,
  mut menu_click: ResMut<MenuClickPending>,
  item_glyph_q: Query<(Entity, &ItemGlyph)>
) {
  // Extract what we need before any mutation so the borrow checker is happy.
  let n_opts = if let InteractMenu::Open { ref options, .. } = ui.interact {
    options.len()
  } else {
    0
  };
  let cur_sel =
    if let InteractMenu::Open { selected, .. } = ui.interact { selected } else { 0 };

  // Key-repeat constants: ~0.3 s initial delay, ~0.1 s repeat rate at 60 fps
  const NAV_INITIAL_DELAY: u32 = 8;
  const NAV_REPEAT_RATE: u32 = 1;

  ui.dir_consumed = false; // cleared each frame; set below when a direction key feeds the menu
  if matches!(ui.interact, InteractMenu::Open { .. }) {
    let up_just = keys.just_pressed(KeyCode::KeyW) || keys.just_pressed(KeyCode::ArrowUp);
    let down_just =
      keys.just_pressed(KeyCode::KeyS) || keys.just_pressed(KeyCode::ArrowDown);
    let up_held = keys.pressed(KeyCode::KeyW) || keys.pressed(KeyCode::ArrowUp);
    let down_held = keys.pressed(KeyCode::KeyS) || keys.pressed(KeyCode::ArrowDown);

    // Advance or reset the repeat counter
    let do_up = if up_just {
      ui.menu_nav_dir = -1;
      ui.menu_nav_frames = NAV_INITIAL_DELAY;
      true
    } else if up_held && ui.menu_nav_dir == -1 {
      if ui.menu_nav_frames == 0 {
        ui.menu_nav_frames = NAV_REPEAT_RATE;
        true
      } else {
        ui.menu_nav_frames -= 1;
        false
      }
    } else {
      false
    };

    let do_down = if down_just {
      ui.menu_nav_dir = 1;
      ui.menu_nav_frames = NAV_INITIAL_DELAY;
      true
    } else if down_held && ui.menu_nav_dir == 1 {
      if ui.menu_nav_frames == 0 {
        ui.menu_nav_frames = NAV_REPEAT_RATE;
        true
      } else {
        ui.menu_nav_frames -= 1;
        false
      }
    } else {
      false
    };

    if !up_held && !down_held {
      ui.menu_nav_dir = 0;
      ui.menu_nav_frames = 0;
    }

    if keys.just_pressed(KeyCode::Space) {
      ui.interact = InteractMenu::Closed;
      ui.space_consumed = true;
    } else if do_up {
      if let InteractMenu::Open { ref mut selected, .. } = ui.interact {
        *selected = cur_sel.saturating_sub(1);
      }
      ui.dir_consumed = true;
    } else if do_down {
      if let InteractMenu::Open { ref mut selected, .. } = ui.interact {
        *selected = (cur_sel + 1).min(n_opts.saturating_sub(1));
      }
      ui.dir_consumed = true;
    } else {
      let exec_idx: Option<usize> = if keys.just_pressed(KeyCode::KeyA)
        || keys.just_pressed(KeyCode::KeyD)
        || keys.just_pressed(KeyCode::Enter)
      {
        ui.dir_consumed = true;
        Some(cur_sel)
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
        && idx < n_opts
      {
        Some(idx)
      } else {
        menu_click.0.take().filter(|&i| i < n_opts)
      };

      if let Some(idx) = exec_idx
        && let InteractMenu::Open { ref options, ref disabled, .. } = ui.interact
        && !disabled.get(idx).copied().unwrap_or(false)
        && let Some(option) = options.get(idx).cloned()
      {
        let is_loadout = matches!(
          option.action,
          InteractionAction::EquipItem(_) | InteractionAction::UnequipItem(_)
        );
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
          &mut deferred,
          &mut door_q,
          &asset_server,
          &mut palette_cache,
          &mut images,
          &item_glyph_q
        );
        if is_loadout && let Ok((_, inventory, equipped)) = player_query.single() {
          let opts = loadout_options(&inventory, &equipped);
          let highlighted = utils::mapv(|o: &_| is_equipped(&o.action, &equipped), &opts);
          let disabled = utils::mapv(|o: &_| is_disabled(&o.action, &equipped), &opts);
          let new_sel = cur_sel.min(opts.len().saturating_sub(1));
          if !opts.is_empty() {
            ui.interact = InteractMenu::Open {
              options: opts,
              selected: new_sel,
              highlighted,
              disabled
            };
          }
        }
      }
    }
  } else {
    menu_click.0 = None; // discard stale clicks when no menu is open
    match ui.pause {
      PauseMenu::Closed => {
        let open = keys.just_pressed(KeyCode::Tab)
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
        if keys.just_pressed(KeyCode::Space) || keys.just_pressed(KeyCode::Digit1) {
          ui.pause = PauseMenu::Closed;
          ui.space_consumed = true;
        } else if keys.just_pressed(KeyCode::Digit2) {
          ui.pause = PauseMenu::Controls;
        } else if keys.just_pressed(KeyCode::Digit3) {
          exit.write(AppExit::Success);
        }
      }
      PauseMenu::Controls => {
        if keys.just_pressed(KeyCode::Space) {
          ui.pause = PauseMenu::Main;
          ui.space_consumed = true;
        }
      }
    }
  }
}

fn log_dialogue_node_block(
  log: &mut LogEntries,
  speaker: &str,
  speaker_color: Color,
  node: &DialogueNode
) {
  log_spans(log, vec![
    LogSpan::colored(format!("{speaker}:"), speaker_color),
    LogSpan::plain(format!(" {}", node.text)),
  ]);
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

  if let DialogueState::Open { speaker, tree, node_name, speaker_color } = &ui.dialogue {
    let (speaker, tree, node_name, speaker_color) =
      (*speaker, *tree, *node_name, *speaker_color);
    let node = tree.find(node_name);
    if keys.just_pressed(KeyCode::Space) {
      ui.dialogue = DialogueState::Closed;
      ui.space_consumed = true;
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
      log_spans(&mut *log, vec![
        LogSpan::colored("You:", PLAYER_PRIMARY),
        LogSpan::plain(format!(" {}", choice.text)),
      ]);
      if let Some(next_name) = choice.next {
        ui.dialogue =
          DialogueState::Open { speaker, tree, node_name: next_name, speaker_color };
        let next_node = tree.find(next_name);
        log_dialogue_node_block(&mut *log, speaker, speaker_color, next_node);
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
  player_query: &mut Query<(&mut Location, &mut Inventory, &mut Loadout), With<Player>>,
  loot_chest_q: &mut Query<(&mut LootChest, &mut Glyph, &Location), Without<Player>>,
  fixed_q: &Query<&FixedChestLoot>,
  log: &mut LogEntries,
  clock: &mut Clock,
  tb: &mut TurnBasedWorldState
) {
  if let Ok((mut chest, mut glyph, loc)) = loot_chest_q.get_mut(entity)
    && !chest.opened
    && let Ok((_, mut inventory, _)) = player_query.single_mut()
    && let &Location::Coords { x: cx, y: cy, z: cz, .. } = loc
  {
    let bundles: Vec<(Item, u32)> = fixed_q
      .get(entity)
      .map(|f| f.0.to_vec())
      .unwrap_or_else(|_| loot::roll_chest_loot(42u64, cx, cy, cz));
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
    clock.spend_turn(tb);
  }
}

fn build_salvage_entries(
  inv: &HashMap<Item, u32>
) -> (Vec<ui::CraftingEntry>, Vec<InteractionAction>) {
  let mut items =
    utils::mapv(|(&k, _)| k, utils::filter(|(k, n)| **n > 0 && k.can_salvage(), inv));
  items.sort_by(|a, b| a.name().cmp(b.name()));
  let entries: Vec<ui::CraftingEntry> = items
    .iter()
    .map(|&item| {
      let y = item.scrap_yield();
      let detail = y
        .iter()
        .map(|&(i, q)| format!("{}x {}", q, i.name()))
        .collect::<Vec<_>>()
        .join(", ");
      let have = inv.get(&item).copied().unwrap_or(0);
      ui::CraftingEntry {
        label: format!("{} (have {})", item.name(), have),
        detail: format!("→ {detail}"),
        craftable: true
      }
    })
    .collect();
  let actions = items.into_iter().map(InteractionAction::Salvage).collect();
  (entries, actions)
}

fn build_craft_entries(
  inv: &HashMap<Item, u32>
) -> (Vec<ui::CraftingEntry>, Vec<InteractionAction>) {
  let entries: Vec<ui::CraftingEntry> = crafting::RECIPES
    .iter()
    .enumerate()
    .map(|(_, r)| {
      let can = crafting::can_craft(inv, r);
      let ingredients = r
        .ingredients
        .iter()
        .map(|&(item, need)| {
          let have = inv.get(&item).copied().unwrap_or(0);
          if have >= need {
            format!("{}x {}", need, item.name())
          } else {
            format!("{}x {} ({}/{})", need, item.name(), have, need)
          }
        })
        .collect::<Vec<_>>()
        .join(", ");
      ui::CraftingEntry {
        label: format!("{}x {}", r.output_qty, r.output.name()),
        detail: ingredients,
        craftable: can
      }
    })
    .collect();
  let actions = crafting::RECIPES
    .iter()
    .enumerate()
    .map(|(i, _)| InteractionAction::Craft(i))
    .collect();
  (entries, actions)
}

fn open_crafting_menu(ui: &mut UiState, inv: &HashMap<Item, u32>) {
  let (salvage_entries, salvage_actions) = build_salvage_entries(inv);
  let (craft_entries, craft_actions) = build_craft_entries(inv);
  ui.crafting = CraftingMenu::Open {
    tab: 0,
    selected: 0,
    scroll: 0,
    salvage_actions,
    craft_actions,
    salvage_entries,
    craft_entries
  };
}

pub const CRAFT_VISIBLE_ROWS: usize = 12;

fn craft_scroll_for(selected: usize, old_scroll: usize) -> usize {
  if selected < old_scroll {
    selected
  } else if selected >= old_scroll + CRAFT_VISIBLE_ROWS {
    selected + 1 - CRAFT_VISIBLE_ROWS
  } else {
    old_scroll
  }
}

fn handle_crafting_menu(
  keys: Res<ButtonInput<KeyCode>>,
  mut ui: ResMut<UiState>,
  mut clock: ResMut<Clock>,
  mut tb: ResMut<TurnBasedWorldState>,
  mut log: ResMut<LogEntries>,
  mut player_query: Query<(&mut Location, &mut Inventory, &mut Loadout), With<Player>>
) {
  if !matches!(ui.crafting, CraftingMenu::Open { .. }) {
    return;
  }

  if keys.just_pressed(KeyCode::Space) {
    ui.crafting = CraftingMenu::Closed;
    ui.space_consumed = true;
    return;
  }

  // Key repeat (same pattern as interact menu)
  const NAV_INITIAL_DELAY: u32 = 8;
  const NAV_REPEAT_RATE: u32 = 1;

  let up_just = keys.just_pressed(KeyCode::KeyW) || keys.just_pressed(KeyCode::ArrowUp);
  let down_just =
    keys.just_pressed(KeyCode::KeyS) || keys.just_pressed(KeyCode::ArrowDown);
  let up_held = keys.pressed(KeyCode::KeyW) || keys.pressed(KeyCode::ArrowUp);
  let down_held = keys.pressed(KeyCode::KeyS) || keys.pressed(KeyCode::ArrowDown);

  let do_up = if up_just {
    ui.menu_nav_dir = -1;
    ui.menu_nav_frames = NAV_INITIAL_DELAY;
    true
  } else if up_held && ui.menu_nav_dir == -1 {
    if ui.menu_nav_frames == 0 {
      ui.menu_nav_frames = NAV_REPEAT_RATE;
      true
    } else {
      ui.menu_nav_frames -= 1;
      false
    }
  } else {
    false
  };

  let do_down = if down_just {
    ui.menu_nav_dir = 1;
    ui.menu_nav_frames = NAV_INITIAL_DELAY;
    true
  } else if down_held && ui.menu_nav_dir == 1 {
    if ui.menu_nav_frames == 0 {
      ui.menu_nav_frames = NAV_REPEAT_RATE;
      true
    } else {
      ui.menu_nav_frames -= 1;
      false
    }
  } else {
    false
  };

  if !up_held && !down_held {
    ui.menu_nav_dir = 0;
    ui.menu_nav_frames = 0;
  }

  let (tab, selected, scroll, action) = {
    let CraftingMenu::Open {
      tab,
      selected,
      scroll,
      ref salvage_actions,
      ref craft_actions,
      ref salvage_entries,
      ref craft_entries,
      ..
    } = ui.crafting
    else {
      unreachable!()
    };
    let entries = if tab == 0 { salvage_entries } else { craft_entries };
    let actions = if tab == 0 { salvage_actions } else { craft_actions };
    let n = entries.len();

    if keys.just_pressed(KeyCode::KeyA)
      || keys.just_pressed(KeyCode::ArrowLeft)
      || keys.just_pressed(KeyCode::KeyD)
      || keys.just_pressed(KeyCode::ArrowRight)
    {
      let new_tab = if tab == 0 { 1 } else { 0 };
      (new_tab, 0usize, 0usize, None)
    } else if do_up {
      let new_sel = selected.saturating_sub(1);
      (tab, new_sel, craft_scroll_for(new_sel, scroll), None)
    } else if do_down {
      let new_sel = (selected + 1).min(n.saturating_sub(1));
      (tab, new_sel, craft_scroll_for(new_sel, scroll), None)
    } else if keys.just_pressed(KeyCode::Enter) && n > 0 && selected < actions.len() {
      let craftable = entries.get(selected).is_some_and(|e| e.craftable);
      if craftable {
        (tab, selected, scroll, Some(actions[selected].clone()))
      } else {
        return;
      }
    } else {
      return;
    }
  };

  ui.dir_consumed = do_up
    || do_down
    || keys.just_pressed(KeyCode::KeyA)
    || keys.just_pressed(KeyCode::KeyD)
    || keys.just_pressed(KeyCode::ArrowLeft)
    || keys.just_pressed(KeyCode::ArrowRight);

  if let Some(ref act) = action {
    if let Ok((_, mut inventory, _)) = player_query.single_mut() {
      match act {
        InteractionAction::Salvage(item) => {
          if let Some(count) = inventory.0.get_mut(item) {
            if *count > 0 {
              *count -= 1;
              if *count == 0 {
                inventory.0.remove(item);
              }
              for &(comp, q) in item.scrap_yield() {
                *inventory.0.entry(comp).or_insert(0) += q;
              }
              log_message(&mut log, format!("Salvaged {}.", item.name()));
              clock.spend_turn(&mut tb);
            }
          }
        }
        InteractionAction::Craft(recipe_idx) => {
          if let Some(recipe) = crafting::RECIPES.get(*recipe_idx)
            && crafting::can_craft(&inventory.0, recipe)
          {
            crafting::apply_craft(&mut inventory.0, recipe);
            log_message(&mut log, format!("Crafted {}.", recipe.output.name()));
            clock.spend_turn(&mut tb);
          }
        }
        _ => {}
      }
      let inv = &inventory.0;
      let (salvage_entries, salvage_actions) = build_salvage_entries(inv);
      let (craft_entries, craft_actions) = build_craft_entries(inv);
      let new_entries = if tab == 0 { &salvage_entries } else { &craft_entries };
      let new_sel = selected.min(new_entries.len().saturating_sub(1));
      let new_scroll = craft_scroll_for(new_sel, scroll);
      ui.crafting = CraftingMenu::Open {
        tab,
        selected: new_sel,
        scroll: new_scroll,
        salvage_actions,
        craft_actions,
        salvage_entries,
        craft_entries
      };
    }
  } else {
    if let CraftingMenu::Open {
      tab: ref mut t,
      selected: ref mut s,
      scroll: ref mut sc,
      ..
    } = ui.crafting
    {
      *t = tab;
      *s = selected;
      *sc = scroll;
    }
  }
}

fn handle_utility_menus(_keys: Res<ButtonInput<KeyCode>>, _ui: ResMut<UiState>) {}

fn execute_interaction(
  action: &InteractionAction,
  _zone: &mut ActiveZone,
  clock: &mut Clock,
  tb: &mut TurnBasedWorldState,
  ui: &mut UiState,
  log: &mut LogEntries,
  commands: &mut Commands,
  player_query: &mut Query<(&mut Location, &mut Inventory, &mut Loadout), With<Player>>,
  item_glyph_q: &Query<(Entity, &ItemGlyph)>
) {
  // No player/position needed; must not sit behind `player_query` or logging can be skipped.
  if let InteractionAction::Talk { speaker, tree, speaker_color } = action {
    let node = tree.find(tree.nodes[0].name);
    ui.dialogue = DialogueState::Open {
      speaker,
      tree,
      node_name: tree.nodes[0].name,
      speaker_color: *speaker_color
    };
    log_dialogue_node_block(log, speaker, *speaker_color, node);
  } else if let InteractionAction::Navigate { .. } = action {
  } else if let Ok((mut pos, mut inventory, mut equipped)) = player_query.single_mut() {
    let Location::Coords { z: cur_z, .. } = *pos else { unreachable!() };
    match action {
      InteractionAction::Talk { .. } => unreachable!(),
      InteractionAction::Navigate { .. } => {}
      InteractionAction::ToggleDoor(_) => {}
      InteractionAction::OpenCraftingTable => {}
      InteractionAction::ChopTree(entity) => {
        commands.entity(*entity).despawn();
        *inventory.0.entry(Item::Wood).or_insert(0) += 1;
        clock.spend_turn(tb);
      }
      InteractionAction::PickUpItem(wx, wy) => {
        for (ig_ent, ig) in item_glyph_q.iter() {
          if ig.x == *wx as usize && ig.y == *wy as usize && ig.z == cur_z {
            *inventory.0.entry(ig.item).or_insert(0) += 1;
            commands.entity(ig_ent).despawn();
          }
        }
        clock.spend_turn(tb);
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
        clock.spend_turn(tb);
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
        clock.spend_turn(tb);
      }
      InteractionAction::EquipItem(item) => {
        if item.is_weapon() {
          if let Some(reason) = equipped.rejection_reason(entities::Gear::Weapon(*item)) {
            log_message(log, format!("Can't equip {} — {}.", item.name(), reason));
          } else {
            equipped.equip_weapon(*item);
            log_message(log, format!("Equipped {} as weapon.", item.name()));
            clock.spend_turn(tb);
          }
        } else if item.is_armor() {
          if let Some(reason) = equipped.rejection_reason(entities::Gear::Armor(*item)) {
            log_message(log, format!("Can't equip {} — {}.", item.name(), reason));
          } else {
            equipped.equip_armor(*item);
            log_message(log, format!("Equipped {} as armor.", item.name()));
            clock.spend_turn(tb);
          }
        } else if item.is_grenade() {
          if let Some(reason) = equipped.rejection_reason(entities::Gear::Grenade(*item))
          {
            log_message(log, format!("Can't equip {} — {}.", item.name(), reason));
          } else {
            equipped.equip_grenade(*item);
            log_message(log, format!("Equipped {}.", item.name()));
            clock.spend_turn(tb);
          }
        } else if item.is_device() {
          equipped.equip_device(*item);
          log_message(log, format!("Equipped {}.", item.name()));
          clock.spend_turn(tb);
        }
      }
      InteractionAction::UnequipItem(item) => {
        if item.is_weapon() {
          if equipped.weapon() == Some(*item)
            && let Some(w) = equipped.unequip_weapon()
          {
            log_message(log, format!("Unequipped {}.", w.name()));
          }
          clock.spend_turn(tb);
        } else if item.is_armor() {
          if equipped.armor_item() == Some(*item)
            && let Some(a) = equipped.unequip_armor()
          {
            log_message(log, format!("Unequipped {}.", a.name()));
          }
          clock.spend_turn(tb);
        } else if item.is_grenade() {
          if equipped.grenade_slots().iter().any(|(_, g)| *g == *item) {
            equipped.remove_grenade_by_item(*item);
            log_message(log, format!("Unequipped {}.", item.name()));
          }
          clock.spend_turn(tb);
        } else if item.is_device() {
          if equipped.device_slots().iter().any(|(_, d)| *d == *item) {
            equipped.remove_device_by_item(*item);
            log_message(log, format!("Unequipped {}.", item.name()));
          }
          clock.spend_turn(tb);
        }
      }
      InteractionAction::ShowLoadoutStatus => {
        let wpn = equipped.weapon().map(|w| w.name()).unwrap_or("none");
        let arm = equipped.armor_item().map(|a| a.name()).unwrap_or("none");
        log_message(log, format!("Loadout — weapon: {wpn}, armor: {arm}."));
      }
      InteractionAction::TakeElevator { dest_z, dest_x, dest_y } => {
        *pos = Location::xyz(*dest_x, *dest_y, *dest_z);
        clock.spend_turn(tb);
      }
      InteractionAction::RecruitFollower { .. }
      | InteractionAction::DismissFollower { .. }
      | InteractionAction::SaveAtBed => {}
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
  player_query: &mut Query<(&mut Location, &mut Inventory, &mut Loadout), With<Player>>,
  deferred: &mut DeferredActions,
  door_q: &mut Query<(
    &mut Door,
    &mut Glyph,
    Option<&mut Collidable>,
    &Location,
    Option<&mut AirlockDoor>
  ), Without<Player>>,
  asset_server: &AssetServer,
  palette_cache: &mut PaletteImageCache,
  images: &mut Assets<Image>,
  item_glyph_q: &Query<(Entity, &ItemGlyph)>
) {
  match &option.action {
    InteractionAction::OpenChest(ent) => {
      deferred.loot_chest = Some(*ent);
    }
    InteractionAction::Navigate { dest } => {
      deferred.navigate = Some(*dest);
    }
    InteractionAction::RecruitFollower { entity, name } => {
      commands.entity(*entity).insert(FollowerState::Following);
      log_message(log, format!("{name} is now following you."));
      clock.spend_turn(tb);
    }
    InteractionAction::DismissFollower { entity, name } => {
      commands.entity(*entity).insert(FollowerState::Dismissed);
      log_message(log, format!("{name} heads home."));
      clock.spend_turn(tb);
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
          asset_server,
          zone.width,
          zone.height
        );
      }
      clock.spend_turn(tb);
    }
    InteractionAction::SaveAtBed => {
      deferred.save_at_bed = true;
    }
    InteractionAction::OpenCraftingTable => {
      if let Ok((_, inventory, _)) = player_query.single() {
        open_crafting_menu(ui, &inventory.0);
      }
    }
    other => {
      execute_interaction(
        other,
        zone,
        clock,
        tb,
        ui,
        log,
        commands,
        player_query,
        item_glyph_q
      );
    }
  }
}

fn auto_close_airlocks(
  mut commands: Commands,
  clock: Res<Clock>,
  current: Res<CurrentZone>,
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
        &asset_server,
        current.0.width,
        current.0.height
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

fn is_equipped(action: &InteractionAction, loadout: &Loadout) -> bool {
  match action {
    InteractionAction::EquipItem(item) | InteractionAction::UnequipItem(item) => {
      item.is_weapon() && loadout.weapon() == Some(*item)
        || item.is_armor() && loadout.armor_item() == Some(*item)
        || item.is_grenade() && loadout.grenade_slots().iter().any(|(_, g)| *g == *item)
        || item.is_device() && loadout.device_slots().iter().any(|(_, d)| *d == *item)
    }
    _ => false
  }
}

fn loadout_options(inventory: &Inventory, loadout: &Loadout) -> Vec<InteractionOption> {
  let sorted = |pred: fn(Item) -> bool| -> Vec<Item> {
    let mut v: Vec<_> = inventory.0.keys().copied().filter(|&i| pred(i)).collect();
    v.sort_by_key(|i| i.name());
    v
  };
  sorted(Item::is_weapon)
    .into_iter()
    .map(|item| {
      let action = if loadout.weapon() == Some(item) {
        InteractionAction::UnequipItem(item)
      } else {
        InteractionAction::EquipItem(item)
      };
      InteractionOption { label: item.name().to_string(), action }
    })
    .chain(sorted(Item::is_armor).into_iter().map(|item| {
      let action = if loadout.armor_item() == Some(item) {
        InteractionAction::UnequipItem(item)
      } else {
        InteractionAction::EquipItem(item)
      };
      InteractionOption { label: item.name().to_string(), action }
    }))
    .chain(sorted(Item::is_grenade).into_iter().map(|item| {
      let in_loadout = loadout.grenade_slots().iter().any(|(_, g)| *g == item);
      let action = if in_loadout {
        InteractionAction::UnequipItem(item)
      } else {
        InteractionAction::EquipItem(item)
      };
      InteractionOption { label: item.name().to_string(), action }
    }))
    .chain(sorted(Item::is_device).into_iter().map(|item| {
      let in_loadout = loadout.device_slots().iter().any(|(_, d)| *d == item);
      let action = if in_loadout {
        InteractionAction::UnequipItem(item)
      } else {
        InteractionAction::EquipItem(item)
      };
      InteractionOption { label: item.name().to_string(), action }
    }))
    .collect()
}

fn is_disabled(action: &InteractionAction, loadout: &Loadout) -> bool {
  match action {
    InteractionAction::EquipItem(item) => {
      if item.is_weapon() {
        !loadout.can_add(entities::Gear::Weapon(*item))
      } else if item.is_armor() {
        !loadout.can_add(entities::Gear::Armor(*item))
      } else if item.is_grenade() {
        !loadout.can_add(entities::Gear::Grenade(*item))
      } else {
        false
      }
    }
    _ => false
  }
}

#[derive(bevy::ecs::system::SystemParam)]
struct InteractQueries<'w, 's> {
  tree_q: Query<'w, 's, Entity, With<Tree>>,
  dialogue_q: Query<'w, 's, (&'static Named, &'static Dialogue)>,
  glyph_q: Query<'w, 's, &'static Glyph, Without<LootChest>>,
  loot_chest_q:
    Query<'w, 's, (&'static mut LootChest, &'static mut Glyph, &'static Location)>,
  door_q: Query<'w, 's, &'static Door>,
  elevator_q: Query<'w, 's, &'static Elevator>,
  named_q: Query<
    'w,
    's,
    (&'static Named, Option<&'static entities::Corpse>, Option<&'static Bed>)
  >,
  console_q: Query<
    'w,
    's,
    (
      Option<&'static FlightConsole>,
      Option<&'static LoadoutConsole>,
      Option<&'static CraftingTable>
    )
  >,
  follower_q: Query<'w, 's, &'static FollowerState>,
  item_glyph_q: Query<'w, 's, (Entity, &'static ItemGlyph)>
}

fn gather_interactions_at_tile(
  wx: i32,
  wy: i32,
  dir_label: &str,
  tile_entities: Option<&Vec<Entity>>,
  iq: &mut InteractQueries,
  galaxy: &galaxy::Galaxy,
  inventory: &Inventory,
  equipped: &Loadout,
  has_ground_items: bool
) -> Vec<InteractionOption> {
  let mut opts = Vec::new();
  if let Some(entities) = tile_entities {
    for &e in entities {
      if iq.tree_q.get(e).is_ok() {
        opts.push(InteractionOption {
          label: format!("Chop tree ({dir_label})"),
          action: InteractionAction::ChopTree(e)
        });
      }
      if let Ok((named, dialogue)) = iq.dialogue_q.get(e) {
        let speaker_color = iq
          .glyph_q
          .get(e)
          .ok()
          .map(|g| g.sprite_palette.map(|(primary, _)| primary).unwrap_or(g.color))
          .unwrap_or(Color::srgb(0.78, 0.80, 0.86));
        opts.push(InteractionOption {
          label: format!("Talk to {}", named.name),
          action: InteractionAction::Talk {
            speaker: named.name,
            tree: dialogue.0,
            speaker_color
          }
        });
      }
      if let Ok(state) = iq.follower_q.get(e)
        && let Ok((named, ..)) = iq.named_q.get(e)
      {
        opts.push(match *state {
          FollowerState::Available | FollowerState::Dismissed => InteractionOption {
            label: "Follow me".into(),
            action: InteractionAction::RecruitFollower { entity: e, name: named.name }
          },
          FollowerState::Following => InteractionOption {
            label: "Go home".into(),
            action: InteractionAction::DismissFollower { entity: e, name: named.name }
          }
        });
      }
      if iq.loot_chest_q.get_mut(e).is_ok_and(|(c, _, _)| !c.opened) {
        opts.push(InteractionOption {
          label: format!("Open chest ({dir_label})"),
          action: InteractionAction::OpenChest(e)
        });
      }
      if let Ok((_, _, Some(_bed))) = iq.named_q.get(e) {
        opts.push(InteractionOption {
          label: format!("Sleep ({dir_label})"),
          action: InteractionAction::SaveAtBed
        });
      }
      if let Ok(door) = iq.door_q.get(e) {
        let verb = if door.open { "Close" } else { "Open" };
        let name = iq.named_q.get(e).map_or("door", |(n, ..)| n.name);
        opts.push(InteractionOption {
          label: format!("{verb} {name} ({dir_label})"),
          action: InteractionAction::ToggleDoor(e)
        });
      }
      if let Ok(elev) = iq.elevator_q.get(e) {
        let is_cave = iq.named_q.get(e).is_ok_and(|(n, ..)| n.name.contains("Cave"));
        for &(z, dx, dy) in elev.floors.iter().filter(|&&(z, _, _)| z != elev.current_z) {
          opts.push(InteractionOption {
            label: if is_cave {
              if z == 0 { "Return to surface".into() } else { "Enter cave".into() }
            } else {
              format!("Elevator — Deck {}", z + 1)
            },
            action: InteractionAction::TakeElevator { dest_z: z, dest_x: dx, dest_y: dy }
          });
        }
      }
      if let Ok((flight, loadout, craft_table)) = iq.console_q.get(e) {
        if let Some(_) = flight {
          let mut dests: Vec<_> = galaxy
            .all_location_names()
            .filter(|&(id, _)| {
              galaxy
                .get(id)
                .map_or(true, |loc| loc.location_type != LocationType::ShipInterior)
            })
            .map(|(id, name)| InteractionOption {
              label: format!("Chart course — {name}"),
              action: InteractionAction::Navigate { dest: id }
            })
            .collect();
          dests.sort_by_key(|o| o.label.clone());
          opts.extend(dests);
        }
        if let Some(_) = loadout {
          opts.extend(loadout_options(inventory, equipped));
        }
        if let Some(_) = craft_table {
          opts.push(InteractionOption {
            label: "Crafting Table".into(),
            action: InteractionAction::OpenCraftingTable
          });
        }
      }
    }
  }
  if has_ground_items {
    opts.push(InteractionOption {
      label: format!("Pick up item ({dir_label})"),
      action: InteractionAction::PickUpItem(wx, wy)
    });
  }
  opts
}

fn resolve_bump_interact(
  mut pending: ResMut<PendingBumpInteract>,
  mut flash: ResMut<BumpInteractFlash>,
  mut ui: ResMut<UiState>,
  index: Res<TileEntityIndex>,
  galaxy: Res<galaxy::Galaxy>,
  mut iq: InteractQueries,
  player: Single<(&Inventory, &Loadout), With<Player>>
) {
  let Some((tx, ty, tz)) = pending.0.take() else {
    pending.1 = None;
    return;
  };
  let (inventory, equipped) = *player;
  let has_items = iq
    .item_glyph_q
    .iter()
    .any(|(_, ig)| ig.x == tx as usize && ig.y == ty as usize && ig.z == tz);
  let opts = gather_interactions_at_tile(
    tx,
    ty,
    "ahead",
    index.0.get(&(tx, ty, tz)),
    &mut iq,
    &galaxy,
    inventory,
    equipped,
    has_items
  );
  let highlighted = utils::mapv(|o: &_| is_equipped(&o.action, equipped), &opts);
  let disabled = utils::mapv(|o: &_| is_disabled(&o.action, equipped), &opts);
  match opts.len() {
    0 => {
      pending.1 = None;
    }
    1 => flash.0 = opts.into_iter().next(),
    _ => {
      ui.interact =
        InteractMenu::Open { options: opts, selected: 0, highlighted, disabled }
    }
  }
}

fn apply_bump_auto_interact(
  mut flash: ResMut<BumpInteractFlash>,
  mut pending_bump: ResMut<PendingBumpInteract>,
  frame: Res<RenderFrame>,
  mut commands: Commands,
  mut gw: ResMut<CurrentZone>,
  mut clock: ResMut<Clock>,
  mut tb: ResMut<TurnBasedWorldState>,
  mut ui: ResMut<UiState>,
  mut log: ResMut<LogEntries>,
  mut player_query: Query<(&mut Location, &mut Inventory, &mut Loadout), With<Player>>,
  mut deferred: ResMut<DeferredActions>,
  mut door_q: Query<(
    &mut Door,
    &mut Glyph,
    Option<&mut Collidable>,
    &Location,
    Option<&mut AirlockDoor>
  ), Without<Player>>,
  asset_server: Res<AssetServer>,
  mut palette_cache: ResMut<PaletteImageCache>,
  mut images: ResMut<Assets<Image>>,
  item_glyph_q: Query<(Entity, &ItemGlyph)>
) {
  if let Some((entity, dir)) = pending_bump.1.take() {
    commands.entity(entity).insert(BumpLunge { dir, start_frame: frame.0 });
  }
  if let Some(option) = flash.0.take() {
    dispatch_interactive_choice(
      option,
      &mut commands,
      &mut gw.0,
      &mut clock,
      &mut tb,
      &mut ui,
      &mut log,
      &mut player_query,
      &mut deferred,
      &mut door_q,
      &asset_server,
      &mut palette_cache,
      &mut images,
      &item_glyph_q
    );
  }
}

fn flush_pending_loot(
  mut pending: ResMut<DeferredActions>,
  mut commands: Commands,
  mut player_q: Query<(&mut Location, &mut Inventory, &mut Loadout), With<Player>>,
  mut loot_chest_q: Query<(&mut LootChest, &mut Glyph, &Location), Without<Player>>,
  fixed_q: Query<&FixedChestLoot>,
  mut log: ResMut<LogEntries>,
  mut clock: ResMut<Clock>,
  mut tb: ResMut<TurnBasedWorldState>
) {
  if let Some(ent) = pending.loot_chest.take() {
    apply_open_chest(
      &mut commands,
      ent,
      &mut player_q,
      &mut loot_chest_q,
      &fixed_q,
      &mut *log,
      &mut *clock,
      &mut *tb
    );
  }
}

fn apply_bed_save(
  mut deferred: ResMut<DeferredActions>,
  mut bed_save: ResMut<BedSave>,
  ship: Res<ship::Ship>,
  player: Single<(&Location, &Inventory, &Loadout), With<Player>>,
  mut log: ResMut<LogEntries>
) {
  if std::mem::take(&mut deferred.save_at_bed)
    && let (&Location::Coords { x, y, z, .. }, inventory, loadout) = *player
  {
    bed_save.0 = Some(SaveData {
      docked_at: ship.docked_at,
      pos: (x, y, z),
      inventory: inventory.0.clone(),
      loadout: loadout.clone()
    });
    log_message(&mut log, "You rest and save your progress.".into());
  }
}

fn player_death_check(
  mut bed_save: ResMut<BedSave>,
  ship: Res<ship::Ship>,
  mut deferred: ResMut<DeferredActions>,
  mut player: Query<
    (&mut Location, &mut Stats, &mut Inventory, &mut Loadout),
    With<Player>
  >,
  mut log: ResMut<LogEntries>
) {
  let Ok((mut pos, mut stats, mut inventory, mut loadout)) = player.single_mut() else {
    return;
  };
  if stats.hp > 0 {
    return;
  }
  let Some(save) = bed_save.0.take() else { return };
  *pos = Location::xyz(save.pos.0, save.pos.1, save.pos.2);
  stats.hp = stats.max_hp;
  inventory.0 = save.inventory.clone();
  *loadout = save.loadout.clone();
  if ship.docked_at != save.docked_at {
    if let Some(dest) = save.docked_at {
      deferred.navigate = Some(dest);
      deferred.post_navigate_pos = Some(save.pos);
    }
  }
  bed_save.0 = Some(save);
  log_message(&mut log, "You wake up in bed, shaken but alive.".into());
}

fn handle_interact(
  keys: Res<ButtonInput<KeyCode>>,
  galaxy: Res<galaxy::Galaxy>,
  mut ui: ResMut<UiState>,
  mut flash: ResMut<BumpInteractFlash>,
  index: Res<TileEntityIndex>,
  player: Single<(&Location, &Inventory, &Loadout), With<Player>>,
  mut iq: InteractQueries
) {
  let space_consumed = std::mem::take(&mut ui.space_consumed);
  if ui.any_open() || space_consumed || !keys.just_pressed(KeyCode::Space) {
    return;
  }

  let (&Location::Coords { x: px, y: py, z: pz, .. }, inventory, equipped) = *player else {
    unreachable!()
  };
  let mut options = Vec::new();
  for dy in -1i32..=1 {
    for dx in -1i32..=1 {
      let (wx, wy) = (px + dx, py + dy);
      let dir =
        if dx == 0 && dy == 0 { "here".to_string() } else { direction_name(dx, dy) };
      let has_items = iq
        .item_glyph_q
        .iter()
        .any(|(_, ig)| ig.x == wx as usize && ig.y == wy as usize && ig.z == pz);
      options.extend(gather_interactions_at_tile(
        wx,
        wy,
        &dir,
        index.0.get(&(wx, wy, pz)),
        &mut iq,
        &galaxy,
        inventory,
        equipped,
        has_items
      ));
    }
  }

  let highlighted = utils::mapv(|o: &_| is_equipped(&o.action, equipped), &options);
  let disabled = utils::mapv(|o: &_| is_disabled(&o.action, equipped), &options);
  match options.len() {
    0 => {}
    1 => flash.0 = options.into_iter().next(),
    _ => ui.interact = InteractMenu::Open { options, selected: 0, highlighted, disabled }
  }
}

fn spawn_tilemaps(
  commands: &mut Commands,
  zone: &active_zone::ActiveZone,
  tileset: Handle<Image>
) {
  for z in 0..zone.depth {
    let level = zone.level(z);
    let tile_data = vec![None; level.width * level.height];
    commands.spawn((
      TilemapChunk {
        chunk_size: UVec2::new(zone.width as u32, zone.height as u32),
        tile_display_size: UVec2::splat(TILE_SIZE as u32),
        tileset: tileset.clone(),
        alpha_mode: AlphaMode2d::Blend
      },
      TilemapChunkTileData(tile_data),
      Transform::from_xyz(-TILE_SIZE / 2.0, TILE_SIZE / 2.0, 0.0),
      TilemapLayer(z),
      if z == 0 { Visibility::Visible } else { Visibility::Hidden }
    ));
  }
}

fn spawn_zone_geometry(
  commands: &mut Commands,
  zone: &active_zone::ActiveZone,
  galaxy: &galaxy::Galaxy,
  docked_at: Option<galaxy::LocationId>,
  tileset: Handle<Image>,
  palette_cache: &mut PaletteImageCache,
  images: &mut Assets<Image>
) {
  spawn_tilemaps(commands, zone, tileset);
  let (sox, soy) = zone.ship_origin;
  let ship = prefabs::Prefab::starting_ship();
  ship.stamp_entities(commands, sox, soy, 0);
  let ship_footprint = ship.occupied_positions(sox, soy);
  if docked_at == Some(locations::starter_planet::ID)
    && let Some((dox, doy)) = zone.dest_origin
  {
    locations::starter_planet::surface_prefab().stamp_entities_excluding(
      commands,
      dox,
      doy,
      0,
      &ship_footprint
    );
  }
  if docked_at == Some(locations::mushroom_planet::ID)
    && let Some((dox, doy)) = zone.dest_origin
  {
    locations::mushroom_planet::mushroom_prefab().stamp_entities_excluding(
      commands,
      dox,
      doy,
      0,
      &ship_footprint
    );
  }
  if docked_at == Some(locations::gamma_station::ID)
    && let Some((dox, doy)) = zone.dest_origin
  {
    locations::gamma_station::station_prefab().stamp_entities_excluding(
      commands,
      dox,
      doy,
      0,
      &ship_footprint
    );
  }
  if docked_at == Some(locations::meridian_station::ID)
    && let Some((dox, doy)) = zone.dest_origin
  {
    locations::meridian_station::station_prefab().stamp_entities_excluding(
      commands,
      dox,
      doy,
      0,
      &ship_footprint
    );
    for &(lx, ly) in locations::meridian_station::NPC_COORDS {
      let wx = dox + lx;
      let wy = doy + ly;
      if ship_footprint.contains(&(wx, wy)) {
        continue;
      }
      let obj = match (lx, ly) {
        (23, 3) => locations::meridian_station::dock1(),
        (23, 10) => locations::meridian_station::aiden3(),
        (6, 14) => locations::meridian_station::wren9(),
        (41, 14) => locations::meridian_station::forge(),
        _ => continue
      };
      obj.spawn_at(commands, wx, wy, 0);
    }
  }

  // Spawn any objects registered on the destination location (e.g. elevators).
  if let Some(dest_id) = docked_at
    && let Some(dest_loc) = galaxy.get(dest_id)
    && let Some((dox, doy)) = zone.dest_origin
  {
    for (lx, ly, lz, obj) in &dest_loc.spawn_objects {
      let wx = dox + lx;
      let wy = doy + ly;
      if ship_footprint.contains(&(wx, wy)) {
        continue;
      }
      let ent = obj.clone().spawn_at(commands, wx, wy, *lz);
      commands.entity(ent).queue(move |mut e: bevy::ecs::world::EntityWorldMut| {
        if let Some(mut elev) = e.get_mut::<Elevator>() {
          for (_, x, y) in &mut elev.floors {
            *x += dox;
            *y += doy;
          }
        }
      });
    }
  }
  spawn_item_glyphs(commands, zone, palette_cache, images);
}

fn apply_pending_navigation(
  mut pending: ResMut<DeferredActions>,
  mut commands: Commands,
  mut galaxy: ResMut<galaxy::Galaxy>,
  mut ship: ResMut<ship::Ship>,
  mut current: ResMut<CurrentZone>,
  mut fov: ResMut<Fov>,
  mut log: ResMut<LogEntries>,
  tileset: Res<Tileset>,
  to_despawn: Query<
    Entity,
    (
      Or<(With<TilemapLayer>, With<ItemGlyph>, With<GlyphVisual>, With<Location>)>,
      Without<Player>
    )
  >,
  mut player: Query<
    (&mut Location, &mut Visuals, &mut Transform),
    With<Player>
  >,
  mut clock: ResMut<Clock>,
  mut tb: ResMut<TurnBasedWorldState>,
  mut palette_cache: ResMut<PaletteImageCache>,
  mut images: ResMut<Assets<Image>>
) {
  let Some(dest) = pending.navigate.take() else {
    return;
  };
  if ship.docked_at == Some(dest) {
    log_message(
      &mut *log,
      "Astrogation: already holding position at that chart solution.".into()
    );
    return;
  }
  let Some(new_zone) = docking::dock(&mut galaxy, &mut ship, dest) else {
    log_message(
      &mut *log,
      "Astrogation: cannot plot a dock for that destination.".into()
    );
    return;
  };
  for e in to_despawn.iter() {
    commands.entity(e).despawn();
  }
  // Capture the player's offset within the OLD ship before swapping zones.
  let player_ship_offset = player.single().ok().and_then(|(pos, ..)| {
    if let Location::Coords { x, y, .. } = *pos {
      let (old_sox, old_soy) = current.0.ship_origin;
      let rel_x = (x - old_sox).clamp(0, ship::SHIP_WIDTH as i32 - 1);
      let rel_y = (y - old_soy).clamp(0, ship::SHIP_HEIGHT as i32 - 1);
      Some((rel_x, rel_y))
    } else {
      None
    }
  });

  *current = CurrentZone(new_zone);
  fov.0 = FovGrid::new(current.0.width, current.0.height);
  spawn_zone_geometry(
    &mut commands,
    &current.0,
    &galaxy,
    ship.docked_at,
    tileset.0.handle.clone(),
    &mut palette_cache,
    &mut images
  );
  let (sox, soy) = current.0.ship_origin;
  let (offset_x, offset_y) = player_ship_offset
    .unwrap_or((ship::SHIP_WIDTH as i32 / 2, ship::SHIP_HEIGHT as i32 / 2));
  let local_x = sox + offset_x;
  let local_y = soy + offset_y;
  if let Ok((mut location, mut vis, mut tf)) = player.single_mut() {
    let (lx, ly, lz) = pending.post_navigate_pos.take().unwrap_or((local_x, local_y, 0));
    *location = Location::xyz(lx, ly, lz);
    let start_local = Vec2::new(lx as f32, ly as f32);
    vis.prev = start_local;
    vis.display = start_local;
    vis.last_pos = start_local;
    vis.last_move_start_frame = None;
    tf.translation =
      tile_screen_pos(lx as f32, ly as f32, current.0.width, current.0.height) + Vec3::Z;
  }
  clock.spend_turn(&mut tb);
  let dest_name = galaxy.get(dest).map_or("destination", |loc| loc.name);
  log_message(&mut *log, format!("Astrogation: docked — {dest_name} sector."));
}

fn init_follower_homes(mut follower_q: Query<(&mut FollowerData, &Location)>) {
  for (mut data, location) in follower_q.iter_mut() {
    if let Location::Coords { x, y, z, .. } = *location {
      data.home = (x, y, z);
    }
  }
}

fn setup(
  mut commands: Commands,
  current: Res<CurrentZone>,
  galaxy: Res<galaxy::Galaxy>,
  ship: Res<ship::Ship>,
  mut images: ResMut<Assets<Image>>,
  mut palette_cache: ResMut<PaletteImageCache>,
  mut log: ResMut<LogEntries>,
  mut clock: ResMut<Clock>,
  mut tb: ResMut<TurnBasedWorldState>,
  mut bed_save: ResMut<BedSave>,
  render_target: Res<post_process::GameRenderTarget>,
  entity_rt: Res<post_process::EntityRenderTarget>
) {
  clock.spend_turn(&mut tb);
  commands.spawn((
    Camera2d,
    Msaa::Off,
    post_process::GameCamera,
    post_process::game_render_target(&render_target),
    Camera {
      clear_color: ClearColorConfig::Custom(Color::srgba(0.0, 0.0, 0.0, 0.0)),
      ..default()
    }
  ));
  commands.spawn((
    Camera2d,
    Msaa::Off,
    post_process::EntityCamera,
    RenderLayers::layer(post_process::LAYER_ENTITIES),
    post_process::entity_render_target(&entity_rt),
    Camera {
      order: 5,
      clear_color: ClearColorConfig::Custom(Color::srgba(0.0, 0.0, 0.0, 0.0)),
      ..default()
    }
  ));

  let tileset_info = sprites::build_tileset(&mut images);
  let tileset_handle = tileset_info.handle.clone();
  commands.insert_resource(Tileset(tileset_info));

  spawn_zone_geometry(
    &mut commands,
    &current.0,
    &galaxy,
    ship.docked_at,
    tileset_handle,
    &mut palette_cache,
    &mut images
  );

  let hover_img = white_pixel_image(&mut images);
  commands.spawn((
    TileHoverHighlight,
    Sprite {
      image: hover_img.clone(),
      custom_size: Some(Vec2::splat(TILE_SIZE)),
      color: Color::srgba(0.95, 0.92, 0.45, 0.28),
      ..default()
    },
    Transform::from_translation(Vec3::new(0.0, 0.0, 0.25)),
    Visibility::Hidden
  ));

  let (dox, doy) = current.0.dest_origin.unwrap_or(current.0.ship_origin);
  let (spx, spy) = locations::starter_planet::surface_prefab()
    .find_char('@')
    .expect("starter planet must have a player spawn marker (@)");
  let local_x = dox + spx;
  let local_y = doy + spy;

  let start_local = Vec2::new(local_x as f32, local_y as f32);

  commands.spawn((
    Sprite {
      image: palette_sprite_handle(
        "textures/space_qud/tough guy 1.png",
        Color::srgb(0.72, 0.72, 0.72),
        Color::srgb(0.35, 0.55, 0.72),
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
    Location::xyz(local_x, local_y, 0),
    Stats { hp: 20, max_hp: 20, attack: 5, move_speed: 3.0, attack_speed: 1.0 },
    {
      let mut inv = Inventory::default();
      inv.0.insert(Item::PhaseDevice, 3);
      inv
    },
    {
      let mut l = Loadout::default();
      l.equip_device(Item::PhaseDevice);
      l
    },
    Glyph::palette_sprite(
      "textures/space_qud/tough guy 1.png",
      '@',
      Color::srgb(0.72, 0.72, 0.72),
      Color::srgb(0.35, 0.55, 0.72)
    ),
    WalkAnim {
      idle: "textures/space_qud/tough guy 1.png",
      idle_frames: &["textures/space_qud/tough guy frame 2.png"],
      walk_frames: &[
        "textures/space_qud/tough guy walk frame 1.png",
        "textures/space_qud/tough guy walk frame 2.png"
      ],
      interval: 8,
      idle_interval: 30
    },
    GlyphVisual,
    RenderLayers::layer(post_process::LAYER_ENTITIES),
    Visuals {
      prev: start_local,
      last_move_start_frame: None,
      display: start_local,
      last_pos: start_local
    }
  ));

  bed_save.0 = Some(SaveData {
    docked_at: ship.docked_at,
    pos: (local_x, local_y, 0),
    inventory: HashMap::new(),
    loadout: Loadout::default()
  });

  log_message(
    &mut *log,
    "You wake up in a small outpost on the Origin World. A robot hums nearby."
      .to_string()
  );
}

fn update_fov(
  mut fov: ResMut<Fov>,
  current: Res<CurrentZone>,
  player_pos: Single<&Location, With<Player>>,
  sight_q: Query<&Location, (With<BlocksSight>, Without<Player>)>
) {
  let Location::Coords { x, y, z, .. } = **player_pos else { unreachable!() };
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
  compute_fov(&mut fov.0, current.0.level(z as usize), x, y, FOV_RADIUS, |tx, ty| {
    blockers.contains(&(tx, ty))
  });
}

fn spawn_item_glyphs(
  _commands: &mut Commands,
  _zone: &active_zone::ActiveZone,
  _palette_cache: &mut PaletteImageCache,
  _images: &mut Assets<Image>
) {
}

fn materialize_ground_items(
  mut commands: Commands,
  q: Query<(Entity, &entities::GroundItem, &Location), Without<ItemGlyph>>
) {
  for (entity, gi, location) in q.iter() {
    if let Location::Coords { x, y, z, .. } = *location {
      commands.entity(entity).insert(ItemGlyph {
        x: x as usize,
        y: y as usize,
        z,
        item: gi.0
      });
    }
  }
}

fn camera_follow(
  vis: Single<&Visuals, With<Player>>,
  current: Res<CurrentZone>,
  mut cam_tf: Single<
    &mut Transform,
    (With<post_process::GameCamera>, Without<post_process::EntityCamera>)
  >,
  mut entity_cam_tf: Single<
    &mut Transform,
    (With<post_process::EntityCamera>, Without<post_process::GameCamera>)
  >,
  win: Single<&Window>,
  mut player_screen: ResMut<post_process::PlayerScreenPos>
) {
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
  entity_cam_tf.translation = cam_tf.translation;
  cam_tf.translation = t;
  let rel = world_pos - t.truncate();
  let scale = win.scale_factor();
  player_screen.0 = Vec2::new(
    (w * 0.5 + rel.x) * scale,
    (h * 0.5 - rel.y) * scale
  );
}

#[derive(bevy::ecs::system::SystemParam)]
struct PlayerInputRes<'w> {
  frame: Res<'w, RenderFrame>,
  current: Res<'w, CurrentZone>,
  ui: Res<'w, UiState>,
  clock: ResMut<'w, Clock>,
  tb: ResMut<'w, TurnBasedWorldState>,
  time_mode_auto: ResMut<'w, TimeModeAuto>,
  index: Res<'w, TileEntityIndex>,
  pending_bump: ResMut<'w, PendingBumpInteract>,
  log: ResMut<'w, ui::LogEntries>
}

fn player_input(
  mut commands: Commands,
  keys: Res<ButtonInput<KeyCode>>,
  mut acc: ResMut<AccumulatedDir>,
  mut r: PlayerInputRes,
  player: Single<
    (
      Entity,
      &mut Location,
      &Stats,
      &mut Inventory,
      &Loadout,
      Option<&entities::Grabbed>,
      Option<&entities::Phasing>
    ),
    With<Player>
  >,
  mut enemy_query: Query<&mut Stats, (With<Enemy>, Without<Player>)>,
  collidable_q: Query<&Collidable>,
  item_glyph_q: Query<(Entity, &ItemGlyph)>
) {
  if !r.ui.any_open() && keys.just_pressed(KeyCode::KeyT) {
    if keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight) {
      r.time_mode_auto.0 = true;
    } else {
      r.time_mode_auto.0 = false;
      r.clock.mode = match r.clock.mode {
        TimeMode::RealTime => TimeMode::TurnBased,
        TimeMode::TurnBased => {
          r.tb.world_tick_pending = false;
          TimeMode::RealTime
        }
      };
    }
  }

  if !r.ui.any_open() && !r.ui.dir_consumed {
    let (player_entity, mut loc, stats, mut inventory, equipped, grabbed, phasing) =
      player.into_inner();
    let &Location::Coords { x: px, y: py, z: pz, .. } = &*loc else { unreachable!() };
    let player_attack = stats.attack + equipped.weapon_attack_bonus();
    let turn_based_block = r.clock.mode == TimeMode::TurnBased && r.tb.world_tick_pending;

    let wait_pressed = keys.just_pressed(KeyCode::Period)
      || (r.clock.mode == TimeMode::TurnBased && keys.pressed(KeyCode::Space));

    if grabbed.is_some()
      && !turn_based_block
      && (wait_pressed
        || any_direction_pressed(&keys)
        || acc.up
        || acc.down
        || acc.left
        || acc.right)
    {
      *acc = AccumulatedDir::default();
      ui::log_message(&mut r.log, "You are grabbed and can't move!".into());
      r.clock.spend_turn(&mut r.tb);
    } else if !turn_based_block && wait_pressed {
      r.clock.spend_turn(&mut r.tb);
    } else if !turn_based_block
      && (any_direction_pressed(&keys) || acc.up || acc.down || acc.left || acc.right)
    {
      let level = r.current.0.level(pz);
      let up = keys.pressed(KeyCode::KeyW) || keys.pressed(KeyCode::ArrowUp) || acc.up;
      let down =
        keys.pressed(KeyCode::KeyS) || keys.pressed(KeyCode::ArrowDown) || acc.down;
      let left =
        keys.pressed(KeyCode::KeyA) || keys.pressed(KeyCode::ArrowLeft) || acc.left;
      let right =
        keys.pressed(KeyCode::KeyD) || keys.pressed(KeyCode::ArrowRight) || acc.right;
      *acc = AccumulatedDir::default();
      let raw_dx = match (left, right) {
        (true, false) => -1,
        (false, true) => 1,
        _ => 0
      };
      let raw_dy = match (up, down) {
        (true, false) => -1,
        (false, true) => 1,
        _ => 0
      };

      let is_entity_blocked = |x, y| {
        r.index.0.get(&(x, y, pz)).is_some_and(|entities| {
          entities.iter().any(|&e| {
            collidable_q.get(e).is_ok_and(|c| c.0) && enemy_query.get(e).is_err()
          })
        })
      };
      let (dx, dy) = resolve_move(
        level,
        px,
        py,
        raw_dx,
        raw_dy,
        phasing.is_some(),
        &is_entity_blocked
      );

      let diagonal_bump = raw_dx != 0
        && raw_dy != 0
        && (dx, dy) != (raw_dx, raw_dy)
        && is_entity_blocked(px + raw_dx, py + raw_dy);

      if (dx, dy) == (0, 0) || diagonal_bump {
        r.pending_bump.0 = Some((px + raw_dx, py + raw_dy, pz));
        r.pending_bump.1 =
          Some((player_entity, Vec2::new(raw_dx as f32, raw_dy as f32)));
      } else {
        let target_x = px + dx;
        let target_y = py + dy;

        let enemy_hit =
          r.index.0.get(&(target_x, target_y, pz)).and_then(|entities| {
            entities.iter().find(|&&e| enemy_query.get(e).is_ok()).copied()
          });

        if let Some(hostile) = enemy_hit {
          if let Ok(mut es) = enemy_query.get_mut(hostile) {
            es.hp -= player_attack;
          }
          commands.entity(player_entity).insert(BumpLunge {
            dir: Vec2::new(dx as f32, dy as f32),
            start_frame: r.frame.0
          });
        } else {
          *loc = Location::xyz(target_x, target_y, pz);

          for (ig_ent, ig) in item_glyph_q.iter() {
            if ig.x == target_x as usize && ig.y == target_y as usize && ig.z == pz {
              *inventory.0.entry(ig.item).or_insert(0) += 1;
              commands.entity(ig_ent).despawn();
            }
          }
        }

        r.clock.spend_turn(&mut r.tb);
      }
    }
  }
}
