//! Haalka-based UI layer.  Owns the full window layout and centred overlays for menus.
//! A single [`sync_ui`] system bridges Bevy game-state into cloneable resources
//! each frame; Haalka `reactive_text` / [`from_resource`] read those resources.
//!
//! # Layout tree (bottom → top)
//!
//! - **`Stack`** (full window): Haalka implements this as a CSS grid with one cell; each `.layer` is a
//!   stack child stacked in that cell.
//! - **Layer 1 — [`main_layout`]**: [`Column`] with [`PositionType::Absolute`] and zero inset so it
//!   **fills the window**. Without a definite width, `%` widths on the row (game \| sidebar) can
//!   shrink-wrap and leave empty strips at the right edge (you see the scene clear colour: dark blue‑black).
//!   - **Row** (`flex_grow` 1): transparent game pane (`flex_grow`) \| [`sidebar_column`] (~30%).
//!   - **Status bar** (fixed height).
//! - **Layer 2 — [`overlay_signal`]**: fullscreen dim when pause / interact / dialogue is open.

use {crate::{Clock, GAME_VIEWPORT_WIDTH_FRAC, STATUS_BAR_HEIGHT,
             abilities::AbilityBarData,
             entities::{Glyph, Loadout, Location, Named, ShowOnCompass, Stats},
             game_pane_rect,
             sprites::{PaletteImageCache, palette_sprite_handle},
             utils::mapv,
             world_to_level_cell,
             {CreatorName, CreatorOption, CreatorOptionIndex, CharacterCreatorData,
              STARTING_ITEMS, SPECIAL_ABILITIES}},
     bevy::{prelude::*,
            text::FontWeight,
            ui::{AlignItems, FlexWrap, JustifyContent}},
     haalka::{jonmo::SignalProcessing, prelude::*},
     jonmo::signal,
     bevy_ui_text_input::TextInputPrompt};

// ---------------------------------------------------------------------------
// Data shapes — written by sync_ui, read by Haalka signals
// ---------------------------------------------------------------------------

#[derive(Resource, Clone)]
pub struct ClockData {
  pub mode: &'static str,
  pub tick: u64
}

impl Default for ClockData {
  fn default() -> Self { Self { mode: "RT", tick: 0 } }
}

#[derive(Resource, Clone, Default)]
pub struct PlayerData {
  pub hp: i32,
  pub max_hp: i32,
  pub attack: i32,
  pub speed: f32,
  pub x: i32,
  pub y: i32,
  pub z: usize,
  pub equipped_weapon: Option<String>,
  pub equipped_armor: Option<String>,
  pub status_effects: Vec<String>
}

#[derive(Resource, Clone, Default)]
pub struct HoverInfo {
  pub coords: (i32, i32),
  pub tile_name: String,
  pub item_name: Option<String>,
  pub entity_name: Option<String>,
  pub entity_hp: Option<(i32, i32)>,
  pub flavor: Option<String>
}

/// One inline run of text with an optional color override (None = default log color).
#[derive(Clone, PartialEq, Debug)]
pub struct LogSpan {
  pub text: String,
  pub color: Option<Color>
}

impl LogSpan {
  pub fn plain(text: impl Into<String>) -> Self {
    Self { text: text.into(), color: None }
  }
  pub fn colored(text: impl Into<String>, color: Color) -> Self {
    Self { text: text.into(), color: Some(color) }
  }
}

pub type LogLine = Vec<LogSpan>;

/// Accumulated messages; capped at 100 entries. Updated by game systems in `Update`.
#[derive(Resource, Clone, Default)]
pub struct LogEntries(pub Vec<LogLine>);

/// Log body for the sidebar — last 50 lines, written by `sync_ui`, read by Haalka signal.
#[derive(Resource, Clone, Default, PartialEq)]
pub struct LogDisplayData(pub Vec<LogLine>);

/// Push a plain-text line; oldest entries are dropped to keep at most 100.
pub fn log_message(log: &mut LogEntries, line: String) {
  log_spans(log, vec![LogSpan::plain(line)]);
}

/// Push a multi-span line with per-span color control.
pub fn log_spans(log: &mut LogEntries, line: LogLine) {
  const MAX: usize = 100;
  while log.0.len() >= MAX {
    log.0.remove(0);
  }
  log.0.push(line);
}

/// One marker on the navigation compass; `x`/`y` are pixel offsets from the dial centre.
#[derive(Clone, Debug, PartialEq)]
pub struct CompassIcon {
  /// White-baked sprite of the thing being pointed at.
  pub image: Handle<Image>,
  pub x: f32,
  pub y: f32
}

/// Markers for the compass dial — written by `sync_compass`, read by per-slot signals.
#[derive(Resource, Clone, Default, PartialEq)]
pub struct CompassData(pub Vec<CompassIcon>);

/// Wall layer under the compass markers: one dial-sized image whose pixels are
/// redrawn in place by `sync_compass` when the player moves. The handle is set
/// once, so the UI signal fires once; later frames mutate the asset directly.
#[derive(Resource, Clone, Default, PartialEq)]
pub struct CompassWalls(pub Handle<Image>);

/// Per-row display state for the Interact overlay — written by sync_interact_display, read by
/// per-row signals. Split out of OverlayKind::Interact so navigation/equip never rebuilds overlay
/// nodes; only the signals for text and color re-fire.
#[derive(Resource, Clone, Default, PartialEq)]
pub struct InteractDisplayState {
  pub selected: usize,
  pub highlighted: Vec<bool>,
  pub disabled: Vec<bool>
}

/// Drives per-row selection highlight in the crafting overlay without rebuilding nodes.
#[derive(Resource, Clone, Default, PartialEq)]
pub struct CraftingDisplayState {
  pub tab: usize,
  pub selected: usize,
  pub scroll: usize
}

#[derive(Clone, Debug, PartialEq)]
pub struct CraftingEntry {
  pub label: String,
  pub detail: String,
  pub craftable: bool
}

#[derive(Resource, Clone, Debug, PartialEq)]
pub enum OverlayKind {
  PauseMain,
  PauseControls,
  /// Clickable option list; row appearance driven by [`InteractDisplayState`] signals.
  Interact {
    options: Vec<String>
  },
  /// While talking: show numbered replies (1) text …) over the playfield.
  Dialogue {
    title: String,
    options: Vec<String>
  },
  /// Dedicated crafting table UI with two tabs.
  CraftingTable {
    salvage: Vec<CraftingEntry>,
    craft: Vec<CraftingEntry>
  },
  QuestLog {
    entries: Vec<QuestLogEntry>
  },
  /// Startup character creator: name field, starting item, special ability, confirm.
  CharacterCreator
}

#[derive(Clone, Debug, PartialEq)]
pub struct QuestLogEntry {
  pub name: String,
  pub journal: String,
  pub objectives: Vec<String>,
  pub completed: bool,
  pub failed: bool,
}

/// Written by the Haalka click handler; read + cleared by `handle_menus` each frame.
#[derive(Resource, Default)]
pub struct MenuClickPending(pub Option<usize>);

/// Tracks which ability slot a [`Button`] entity belongs to.
#[derive(Component)]
pub struct AbilitySlotIndex(pub usize);

/// Formatted inventory string, updated by sync_ui.
#[derive(Resource, Clone, Default)]
pub struct InvDisplayData {
  pub formatted: String
}

// ---------------------------------------------------------------------------
// Colours & constants
// ---------------------------------------------------------------------------

const DARK_BG: Color = Color::srgba(0.0, 0.0, 0.0, 0.75);
const PANEL_BG: Color = Color::srgb(0.12, 0.12, 0.20);
const DIALOGUE_PANEL_BG: Color = Color::srgba(0.0, 0.0, 0.0, 0.75);
const BORDER: Color = Color::srgb(0.62, 0.55, 0.12);
const LIGHT_TEXT: Color = Color::srgb(0.94, 0.94, 0.97);
const DIM_TEXT: Color = Color::srgb(0.78, 0.80, 0.86);
const ACCENT: Color = Color::srgb(0.55, 0.88, 0.65);
/// Real-time / turn line in sidebar (mode name + tick).
const MODE_LINE: Color = Color::srgb(0.65, 0.95, 0.78);
/// “TURN-BASED MODE” banner in the status bar (high contrast).
const TURN_BASED_BADGE: Color = Color::srgb(1.0, 0.88, 0.35);
const HP_GREEN: Color = Color::srgb(0.35, 0.75, 0.35);
const HP_YELLOW: Color = Color::srgb(0.85, 0.75, 0.25);
const HP_RED: Color = Color::srgb(0.85, 0.30, 0.30);
const OVERLAY_DIM: Color = Color::srgba(0.0, 0.0, 0.0, 0.50);
const DIALOGUE_OVERLAY_DIM: Color = Color::srgba(0.0, 0.0, 0.0, 0.68);
const EQUIP_HIGHLIGHT: Color = Color::srgb(1.0, 0.85, 0.15);
const DISABLED_TEXT: Color = Color::srgb(0.45, 0.45, 0.50);

const FONT_SIZE_LABEL: f32 = 15.0;
const FONT_SIZE_BODY: f32 = 17.0;
const FONT_SIZE_TITLE: f32 = 18.0;
const FONT_SIZE_SMALL: f32 = 14.0;
/// Sidebar clock mode + TB banner (slightly larger than body elsewhere).
const FONT_SIZE_MODE: f32 = 15.5;

const W_UI: FontWeight = FontWeight::SEMIBOLD;
const W_STRONG: FontWeight = FontWeight::BOLD;
const W_OVERLAY: FontWeight = FontWeight::MEDIUM;

const PANEL_PAD: f32 = 8.0;

/// Match `main.rs` Haalka row: sidebar width as a percent of the window width.
const SIDEBAR_WIDTH_PERCENT: f32 = (1.0 - GAME_VIEWPORT_WIDTH_FRAC) * 100.0;
const GAME_VIEWPORT_PERCENT: f32 = GAME_VIEWPORT_WIDTH_FRAC * 100.0;

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

pub struct UiPlugin;

impl Plugin for UiPlugin {
  fn build(&self, app: &mut App) {
    app
      .init_resource::<ClockData>()
      .init_resource::<PlayerData>()
      .init_resource::<HoverInfo>()
      .init_resource::<LogEntries>()
      .init_resource::<LogDisplayData>()
      .init_resource::<InvDisplayData>()
      .init_resource::<OverlayData>()
      .init_resource::<InteractDisplayState>()
      .init_resource::<CraftingDisplayState>()
      .init_resource::<MenuClickPending>()
      .init_resource::<CompassData>()
      .init_resource::<CompassWalls>()
      .add_systems(
        PostUpdate,
        (sync_interact_display, sync_crafting_display, sync_compass, sync_ui)
          .before(SignalProcessing)
      );
  }
}

// ---------------------------------------------------------------------------
// Root layout
// ---------------------------------------------------------------------------

fn build_ui_root() -> impl Element {
  Stack::<Node>::new()
    .with_node(|mut n| {
      // Vw/Vh resolve against the window, not parent — immune to Haalka wrappers.
      n.width = Val::Vw(100.0);
      n.height = Val::Vh(100.0);
      n.position_type = PositionType::Absolute;
      n.left = Val::Px(0.);
      n.top = Val::Px(0.);
    })
    .layer(main_layout())
    .layer_signal(overlay_signal())
}

pub fn spawn_haalka_root(world: &mut World) { build_ui_root().spawn(world); }

fn main_layout() -> impl Element {
  Column::<Node>::new()
    .with_node(|mut n| {
      // Vw/Vh give a definite size so children's `%` widths resolve correctly.
      n.width = Val::Vw(100.0);
      n.height = Val::Vh(100.0);
    })
    // ── top row: game viewport | sidebar ──
    .item(
      Row::<Node>::new()
        .with_node(|mut n| {
          n.width = Val::Percent(100.0);
          n.flex_grow = 1.0;
          n.column_gap = Val::Px(0.0);
          // Default Haalka Row uses AlignItems::Center — stretch so panes fill the row height.
          n.align_items = AlignItems::Stretch;
        })
        // Game viewport (transparent — Camera2d renders behind); flex so it fills space left of sidebar.
        .item(
          Column::<Node>::new()
            .with_node(|mut n| {
              n.flex_grow = 1.0;
              n.flex_shrink = 1.0;
              n.min_width = Val::Px(0.0);
              n.justify_content = JustifyContent::FlexEnd;
            })
            .item(compass())
            .item(dialogue_panel())
            .item(ability_bar())
        )
        // Sidebar column — fixed fraction of window width, flush right
        .item(sidebar_column())
    )
    // ── bottom: status bar ──
    .item(status_bar())
}

const ABILITY_SELECTED_BG: Color = Color::srgb(1.0, 0.88, 0.35);
const ABILITY_MAX_SLOTS: usize = 9;

fn ability_slot_label(i: usize, data: &AbilityBarData) -> String {
  data
    .slots
    .get(i)
    .map(|slot| match slot.cooldown {
      0 => format!(" {} {} ", i + 1, slot.name),
      cd => format!(" {} {} ({}) ", i + 1, slot.name, cd)
    })
    .unwrap_or_default()
}

fn ability_slot(i: usize) -> El<Node> {
  El::<Node>::new()
    .with_builder(move |b| {
      b.component_signal::<Node>(signal::from_resource::<AbilityBarData>().map_in(
        move |data: AbilityBarData| {
          let visible = i < data.slots.len();
          let mut node = Node::default();
          node.padding = UiRect::axes(Val::Px(2.0), Val::Px(2.0));
          node.display = if visible { Display::Flex } else { Display::None };
          Some(node)
        }
      ))
      .component_signal::<BackgroundColor>(
        signal::from_resource::<AbilityBarData>().map_in(move |data: AbilityBarData| {
          let bg =
            if data.selected == Some(i) { ABILITY_SELECTED_BG } else { Color::NONE };
          Some(BackgroundColor(bg))
        })
      )
    })
    .child(
      El::<Text>::new()
        .text_font(TextFont { font_size: FONT_SIZE_SMALL, weight: W_UI, ..default() })
        .with_builder(move |b| {
          b.component_signal::<Text>(signal::from_resource::<AbilityBarData>().map_in(
            move |data: AbilityBarData| Some(Text::new(ability_slot_label(i, &data)))
          ))
          .component_signal::<TextColor>(
            signal::from_resource::<AbilityBarData>().map_in(
              move |data: AbilityBarData| {
                let fg = if data.selected == Some(i) { PANEL_BG } else { LIGHT_TEXT };
                Some(TextColor(fg))
              }
            )
          )
        })
    )
    .insert(Button)
    .insert(AbilitySlotIndex(i))
}

fn ability_bar() -> impl Element {
  Row::<Node>::new()
    .with_node(|mut n| {
      n.width = Val::Percent(100.0);
      n.height = Val::Px(36.0);
      n.align_items = AlignItems::Center;
      n.padding = UiRect::axes(Val::Px(10.), Val::Px(0.));
      n.border = UiRect::top(Val::Px(1.0));
    })
    .background_color(BackgroundColor(PANEL_BG))
    .border_color(BorderColor::all(BORDER))
    .item(static_text("Abilities: ", FONT_SIZE_SMALL, LIGHT_TEXT, W_UI))
    .item(reactive_text(
      signal::from_resource::<AbilityBarData>()
        .map_in(|data| if data.slots.is_empty() { "—".into() } else { String::new() }),
      FONT_SIZE_SMALL,
      DIM_TEXT,
      W_UI
    ))
    .items(mapv(ability_slot, 0..ABILITY_MAX_SLOTS))
}

// ---------------------------------------------------------------------------
// Compass — circular dial in the game pane; markers at true bearing, distance
// squashed by a horizon projection r = R·d/(d+K) so far things crowd the rim
// ---------------------------------------------------------------------------

const COMPASS_DIAMETER: f32 = 144.0;
/// Markers stay inside this radius; the rim itself is "infinitely far".
const COMPASS_RADIUS: f32 = COMPASS_DIAMETER / 2.0 - 10.0;
/// Tile distance at which a marker sits halfway to the rim.
const COMPASS_HALF_DIST: f32 = 20.0;
const COMPASS_MAX_ICONS: usize = 64;
const COMPASS_ICON_PX: f32 = 16.0;

fn compass_icon(i: usize) -> El<Node> {
  El::<Node>::new().with_builder(move |b| {
    b.component_signal::<Node>(signal::from_resource::<CompassData>().map_in(
      move |data: CompassData| {
        let mut node = Node::default();
        node.position_type = PositionType::Absolute;
        node.width = Val::Px(COMPASS_ICON_PX);
        node.height = Val::Px(COMPASS_ICON_PX);
        let (left, top, display) = data
          .0
          .get(i)
          .map(|icon| {
            (
              Val::Px(COMPASS_DIAMETER / 2.0 + icon.x - COMPASS_ICON_PX / 2.0),
              Val::Px(COMPASS_DIAMETER / 2.0 + icon.y - COMPASS_ICON_PX / 2.0),
              Display::Flex
            )
          })
          .unwrap_or((Val::Auto, Val::Auto, Display::None));
        node.left = left;
        node.top = top;
        node.display = display;
        Some(node)
      }
    ))
    .component_signal::<ImageNode>(signal::from_resource::<CompassData>().map_in(
      move |data: CompassData| data.0.get(i).map(|icon| ImageNode::new(icon.image.clone()))
    ))
  })
}

fn compass() -> impl Element {
  Row::<Node>::new()
    .with_node(|mut n| {
      n.position_type = PositionType::Absolute;
      n.top = Val::Px(10.0);
      n.right = Val::Px(10.0);
      n.width = Val::Px(COMPASS_DIAMETER);
      n.height = Val::Px(COMPASS_DIAMETER);
      n.border = UiRect::all(Val::Px(1.0));
      n.border_radius = BorderRadius::all(Val::Px(COMPASS_DIAMETER / 2.0));
    })
    .background_color(BackgroundColor(PANEL_BG.with_alpha(0.55)))
    .border_color(BorderColor::all(BORDER))
    // wall layer: gray pixels where wall tiles are, under the markers
    .item(El::<Node>::new().with_builder(|b| {
      b.insert(Node {
        position_type: PositionType::Absolute,
        width: Val::Px(COMPASS_DIAMETER),
        height: Val::Px(COMPASS_DIAMETER),
        ..default()
      })
      .component_signal::<ImageNode>(
        signal::from_resource::<CompassWalls>()
          .map_in(|walls: CompassWalls| Some(ImageNode::new(walls.0)))
      )
    }))
    // centre dot: the player
    .item(
      El::<Node>::new()
        .with_node(|mut n| {
          n.position_type = PositionType::Absolute;
          n.left = Val::Px(COMPASS_DIAMETER / 2.0 - 2.0);
          n.top = Val::Px(COMPASS_DIAMETER / 2.0 - 2.0);
          n.width = Val::Px(4.0);
          n.height = Val::Px(4.0);
          n.border_radius = BorderRadius::all(Val::Px(2.0));
        })
        .background_color(BackgroundColor(DIM_TEXT))
    )
    .items(mapv(compass_icon, 0..COMPASS_MAX_ICONS))
}

/// Repaint the [`CompassWalls`] image: for each dial pixel, invert the horizon
/// projection back to tile space and paint gray scaled by wall coverage.
/// A pixel's footprint in tile space grows toward the rim (many tiles per
/// pixel), so each pixel takes a 3×3 sub-sample grid and uses the fraction of
/// wall hits as alpha — distant walls fade instead of aliasing away.
fn draw_compass_walls(image: &mut Image, level: &crate::level::Level, px: i32, py: i32) {
  let side = COMPASS_DIAMETER as usize;
  let data = image.data.as_mut().expect("compass wall image has CPU data");
  let centre = COMPASS_DIAMETER / 2.0;
  const SUB: usize = 3;
  for iy in 0..side {
    for ix in 0..side {
      let hits = (0..SUB * SUB).fold(0, |hits, s| {
        let (sx, sy) = ((s % SUB) as f32, (s / SUB) as f32);
        let (fx, fy) = (ix as f32 + (sx + 0.5) / SUB as f32 - centre,
                        iy as f32 + (sy + 0.5) / SUB as f32 - centre);
        let r = fx.hypot(fy);
        // Invert r = R·d/(d+K): d = K·r/(R−r). Past ~8× the half-distance the
        // projection crams everything into a couple of rim pixels — cut it off.
        let d = COMPASS_HALF_DIST * r / (COMPASS_RADIUS - r);
        let wall = r < COMPASS_RADIUS - 1.0
          && d < COMPASS_HALF_DIST * 8.0
          && level
            .get(px + (fx * d / r.max(0.001)).round() as i32,
                 py + (fy * d / r.max(0.001)).round() as i32)
            .is_some_and(|t| !t.walkable());
        hits + wall as u32
      });
      let alpha = (160 * hits / (SUB * SUB) as u32) as u8;
      let pixel: [u8; 4] = [150, 150, 150, alpha];
      data[(iy * side + ix) * 4..][..4].copy_from_slice(&pixel);
    }
  }
}

/// Project every [`ShowOnCompass`] entity on the player's z-level onto the compass dial.
fn sync_compass(
  player_q: Query<&Location, With<crate::Player>>,
  marked_q: Query<(&Location, &Glyph), With<ShowOnCompass>>,
  current: Res<crate::CurrentZone>,
  mut cache: ResMut<PaletteImageCache>,
  mut images: ResMut<Assets<Image>>,
  mut data: ResMut<CompassData>,
  mut walls: ResMut<CompassWalls>,
  mut walls_drawn_at: Local<Option<(i32, i32, usize)>>
) {
  let mut icons = Vec::new();
  if let Ok(&Location::Coords { x: px, y: py, z: pz, .. }) = player_q.single() {
    if walls.0 == Handle::default() {
      use bevy::{asset::RenderAssetUsages,
                 render::render_resource::{Extent3d, TextureDimension, TextureFormat}};
      let side = COMPASS_DIAMETER as u32;
      walls.0 = images.add(Image::new_fill(
        Extent3d { width: side, height: side, depth_or_array_layers: 1 },
        TextureDimension::D2,
        &[0, 0, 0, 0],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default()
      ));
    }
    if *walls_drawn_at != Some((px, py, pz)) {
      *walls_drawn_at = Some((px, py, pz));
      // get_mut bumps the change tick → one GPU re-upload, only on player movement.
      draw_compass_walls(
        images.get_mut(&walls.0).expect("compass wall image exists"),
        current.0.level(pz),
        px,
        py
      );
    }
    for (loc, glyph) in &marked_q {
      if let &Location::Coords { x, y, z, .. } = loc
        && z == pz
        && let Some(path) = glyph.texture
      {
        // Tile y grows downward on screen, same as UI y — bearings map directly.
        let (dx, dy) = ((x - px) as f32, (y - py) as f32);
        let d = dx.hypot(dy);
        let r = COMPASS_RADIUS * d / (d + COMPASS_HALF_DIST);
        let scale = if d > 0.0 { r / d } else { 0.0 };
        // White-on-white bake: solid white silhouette, Skyrim-marker style. Cached by key.
        let image =
          palette_sprite_handle(path, Color::WHITE, Color::WHITE, &mut cache, &mut images);
        icons.push(CompassIcon { image, x: dx * scale, y: dy * scale });
      }
    }
    // Stable order so markers keep their slots and signals only fire on real movement.
    icons.sort_by(|a, b| a.x.total_cmp(&b.x).then(a.y.total_cmp(&b.y)));
    icons.truncate(COMPASS_MAX_ICONS);
  }
  if data.0 != icons {
    data.0 = icons
  }
}

fn sidebar_column() -> impl Element {
  Column::<Node>::new()
    .with_node(|mut n| {
      n.width = Val::Percent(SIDEBAR_WIDTH_PERCENT);
      n.flex_shrink = 0.0;
      n.border = UiRect::left(Val::Px(1.0));
      n.padding = UiRect::all(Val::Px(PANEL_PAD));
      n.column_gap = Val::Px(6.0);
    })
    .background_color(BackgroundColor(PANEL_BG))
    .border_color(BorderColor::all(BORDER))
    .item(stats_panel())
    .item(static_text("Q: Quest log", FONT_SIZE_SMALL, DIM_TEXT, W_UI))
    .item(inventory_panel())
    .item(hover_panel())
    .item(message_log()) // flex-grows to fill remainder
}

// ---------------------------------------------------------------------------
// Stats panel
// ---------------------------------------------------------------------------

fn stats_panel() -> impl Element {
  Column::<Node>::new()
    .with_node(|mut n| {
      n.width = Val::Percent(100.);
      n.border = UiRect::all(Val::Px(1.0));
      n.border_radius = BorderRadius::all(Val::Px(4.0));
      n.padding = UiRect::all(Val::Px(PANEL_PAD));
      n.column_gap = Val::Px(4.0);
    })
    .background_color(BackgroundColor(PANEL_BG))
    .border_color(BorderColor::all(BORDER))
    .item(panel_label("Character"))
    .item(hp_bar_row())
    .item(stat_row(
      "ATK",
      signal::from_resource::<PlayerData>().map_in(|d| d.attack.to_string())
    ))
    .item(stat_row(
      "SPD",
      signal::from_resource::<PlayerData>().map_in(|d| format!("{:.1}", d.speed))
    ))
    .item(stat_row(
      "WPN",
      signal::from_resource::<PlayerData>()
        .map_in(|d| d.equipped_weapon.clone().unwrap_or_else(|| "—".into()))
    ))
    .item(stat_row(
      "ARM",
      signal::from_resource::<PlayerData>()
        .map_in(|d| d.equipped_armor.clone().unwrap_or_else(|| "—".into()))
    ))
    .item(z_level_label())
    .item(time_mode_label())
    .item(status_effects_row())
}

fn hp_bar_row() -> impl Element {
  Row::<Node>::new()
    .with_node(|mut n| {
      n.width = Val::Percent(100.);
      n.align_items = AlignItems::Center;
      n.column_gap = Val::Px(6.0);
    })
    // "HP:" label
    .item(static_text("HP:", FONT_SIZE_SMALL, LIGHT_TEXT, W_UI))
    // Bar background
    .item(
      El::<Node>::new()
        .with_node(|mut n| {
          n.flex_grow = 1.;
          n.height = Val::Px(20.0);
          n.border_radius = BorderRadius::all(Val::Px(3.0));
          n.overflow = Overflow::hidden();
        })
        .background_color(BackgroundColor(Color::srgb(0.15, 0.15, 0.20)))
        // Fill bar driven by HP ratio
        .child_signal(
          signal::from_resource::<PlayerData>()
            .map_in::<Option<El<Node>>, Option<El<Node>>, _>(|d| {
              let ratio = if d.max_hp > 0 { d.hp as f32 / d.max_hp as f32 } else { 0.0 };
              let pct = (ratio * 100.0).clamp(0.0, 100.0);
              let color = if ratio > 0.66 {
                HP_GREEN
              } else if ratio > 0.33 {
                HP_YELLOW
              } else {
                HP_RED
              };
              Some(
                El::<Node>::new()
                  .with_node(move |mut n| {
                    n.width = Val::Percent(pct);
                    n.height = Val::Percent(100.0);
                  })
                  .background_color(BackgroundColor(color))
              )
            })
        )
    )
    // "HP/max" text
    .item(reactive_text(
      signal::from_resource::<PlayerData>().map_in(|d| format!("{}/{}", d.hp, d.max_hp)),
      FONT_SIZE_SMALL,
      LIGHT_TEXT,
      W_UI
    ))
}

fn stat_row(
  label: &str,
  value_sig: impl Signal<Item = String> + Clone + 'static
) -> impl Element {
  Row::<Node>::new()
    .with_node(|mut n| {
      n.width = Val::Percent(100.);
      n.align_items = AlignItems::Center;
      n.justify_content = JustifyContent::SpaceBetween;
    })
    .item(static_text(label, FONT_SIZE_SMALL, DIM_TEXT, W_UI))
    .item(reactive_text(value_sig, FONT_SIZE_SMALL, LIGHT_TEXT, W_UI))
}

fn status_effects_row() -> impl Element {
  Row::<Node>::new()
    .with_node(|mut n| {
      n.width = Val::Percent(100.);
      n.flex_wrap = FlexWrap::Wrap;
      n.column_gap = Val::Px(6.0);
    })
    .item(reactive_text(
      signal::from_resource::<PlayerData>().map_in(|d| {
        if d.status_effects.is_empty() {
          String::new()
        } else {
          d.status_effects.join("  ")
        }
      }),
      FONT_SIZE_SMALL,
      HP_RED,
      W_UI
    ))
}

fn z_level_label() -> impl Element {
  reactive_text(
    signal::from_resource::<PlayerData>().map_in(|d| {
      let name = match d.z {
        0 => "Deep Cave",
        1 => "Shallow Cave",
        2 => "Surface",
        3 => "Building Upper",
        z => &*String::leak(format!("Level {}", z))
      };
      format!("{} (z={})", name, d.z)
    }),
    FONT_SIZE_SMALL,
    DIM_TEXT,
    W_UI
  )
}

fn time_mode_label() -> impl Element {
  reactive_text(
    signal::from_resource::<ClockData>().map_in(|d| {
      let icon = match d.mode {
        "RT" => "[Real Time]",
        "TB" => "[Turn Based]",
        m => m
      };
      format!("{} T:{:.0}", icon, d.tick)
    }),
    FONT_SIZE_MODE,
    MODE_LINE,
    W_STRONG
  )
}

// ---------------------------------------------------------------------------
// Inventory panel
// ---------------------------------------------------------------------------

fn inventory_panel() -> impl Element {
  Column::<Node>::new()
    .with_node(|mut n| {
      n.width = Val::Percent(100.);
      n.border = UiRect::all(Val::Px(1.0));
      n.border_radius = BorderRadius::all(Val::Px(4.0));
      n.padding = UiRect::all(Val::Px(PANEL_PAD));
      n.column_gap = Val::Px(2.0);
    })
    .background_color(BackgroundColor(PANEL_BG))
    .border_color(BorderColor::all(BORDER))
    .item(panel_label("Inventory"))
    .item(inventory_list())
}

fn inventory_list() -> impl Element {
  reactive_text(
    signal::from_resource::<InvDisplayData>().map_in(|d| d.formatted.clone()),
    FONT_SIZE_BODY,
    Color::srgb(0.92, 0.86, 0.58),
    W_UI
  )
}

// ---------------------------------------------------------------------------
// Hover info panel
// ---------------------------------------------------------------------------

fn hover_panel() -> impl Element {
  Column::<Node>::new()
    .with_node(|mut n| {
      n.width = Val::Percent(100.);
      n.border = UiRect::all(Val::Px(1.0));
      n.border_radius = BorderRadius::all(Val::Px(4.0));
      n.padding = UiRect::all(Val::Px(PANEL_PAD));
      n.column_gap = Val::Px(2.0);
    })
    .background_color(BackgroundColor(PANEL_BG))
    .border_color(BorderColor::all(BORDER))
    .item(panel_label("Target"))
    .item(hover_coords_line())
    .item(hover_tile_line())
    .item(hover_entity_lines())
}

fn hover_coords_line() -> impl Element {
  reactive_text(
    signal::from_resource::<HoverInfo>()
      .map_in(|h| format!("({}, {})", h.coords.0, h.coords.1)),
    FONT_SIZE_SMALL,
    DIM_TEXT,
    W_UI
  )
}

fn hover_tile_line() -> impl Element {
  reactive_text(
    signal::from_resource::<HoverInfo>().map_in(|h| h.tile_name.clone()),
    FONT_SIZE_BODY,
    LIGHT_TEXT,
    W_UI
  )
}

/// One text node (avoids `item_signal` + nested `Column` extra gaps in the flex layout).
fn hover_entity_lines() -> impl Element {
  reactive_text(
    signal::from_resource::<HoverInfo>().map_in(|h| format_entity_hover_block(&h)),
    FONT_SIZE_BODY,
    Color::srgb(0.96, 0.84, 0.55),
    W_UI
  )
}

fn format_entity_hover_block(h: &HoverInfo) -> String {
  let mut s = String::new();
  if let Some(ref item) = h.item_name {
    s.push_str(item);
  }
  if let Some(ref name) = h.entity_name {
    if !s.is_empty() {
      s.push('\n');
    }
    s.push_str(name);
    if let Some((hp, max)) = h.entity_hp {
      s.push('\n');
      s.push_str(&format_hp_line(hp, max));
    }
    if let Some(ref f) = h.flavor {
      s.push('\n');
      s.push_str(f);
    }
  }
  s
}

fn format_hp_line(hp: i32, max_hp: i32) -> String {
  let ratio = if max_hp > 0 { hp as f32 / max_hp as f32 } else { 0.0 };
  let filled = (ratio * 10.0).round() as usize;
  format!(
    "[{}{}] {}/{}",
    "█".repeat(filled.min(10)),
    "░".repeat(10usize.saturating_sub(filled.min(10))),
    hp,
    max_hp
  )
}

// ---------------------------------------------------------------------------
// Message log (scrollable, flex-grows)
// ---------------------------------------------------------------------------

fn message_log() -> impl Element {
  Column::<Node>::new()
    .with_node(|mut n| {
      n.width = Val::Percent(100.);
      n.flex_grow = 1.;
      n.border = UiRect::all(Val::Px(1.0));
      n.border_radius = BorderRadius::all(Val::Px(4.0));
      n.padding = UiRect::all(Val::Px(PANEL_PAD));
      n.row_gap = Val::Px(4.0);
    })
    .background_color(BackgroundColor(Color::NONE))
    .border_color(BorderColor::all(Color::NONE))
    .item(panel_label("Log"))
    .item(
      El::<Node>::new()
        .with_node(|mut n| {
          n.width = Val::Percent(100.0);
          n.flex_grow = 1.0;
          n.overflow = Overflow::clip_y();
        })
        .child_signal(signal::from_resource_changed::<LogDisplayData>().map_in(|d| {
          Column::<Node>::new()
            .with_node(|mut n| {
              n.width = Val::Percent(100.0);
              n.row_gap = Val::Px(1.0);
              n.position_type = PositionType::Absolute;
              n.bottom = Val::Px(0.0);
              n.left = Val::Px(0.0);
              n.right = Val::Px(0.0);
            })
            .items(d.0.into_iter().map(|line| {
              Row::<Node>::new()
                .with_node(|mut n| {
                  n.flex_wrap = FlexWrap::Wrap;
                })
                .items(line.into_iter().map(|span| {
                  static_text(
                    span.text,
                    FONT_SIZE_SMALL,
                    span.color.unwrap_or(DIM_TEXT),
                    W_UI
                  )
                }))
            }))
        }))
    )
}

// ---------------------------------------------------------------------------
// Status bar
// ---------------------------------------------------------------------------

fn status_bar() -> impl Element {
  El::<Node>::new()
    .with_node(|mut n| {
      n.width = Val::Percent(100.);
      n.height = Val::Px(STATUS_BAR_HEIGHT);
      n.border = UiRect::top(Val::Px(1.0));
      n.padding = UiRect::horizontal(Val::Px(10.0));
    })
    .background_color(BackgroundColor(DARK_BG))
    .border_color(BorderColor::all(BORDER))
    .align(Align::center())
    .align_content(Align::center())
    .child(
      Row::<Node>::new()
        .with_node(|mut n| {
          n.align_items = AlignItems::Center;
          n.column_gap = Val::Px(16.0);
        })
        .item(reactive_text(
          signal::from_resource::<HoverInfo>().map_in(|h| h.tile_name.clone()),
          FONT_SIZE_SMALL,
          DIM_TEXT,
          W_UI
        ))
        .item(reactive_text(
          signal::from_resource::<HoverInfo>()
            .map_in(|h| format!("({},{})", h.coords.0, h.coords.1)),
          FONT_SIZE_SMALL,
          DIM_TEXT,
          W_UI
        ))
        .item(
          Row::<Node>::new()
            .with_node(|mut n| {
              n.column_gap = Val::Px(10.0);
              n.align_items = AlignItems::Center;
            })
            .item(reactive_text(
              signal::from_resource::<ClockData>().map_in(|d| {
                (d.mode == "TB").then_some("TURN-BASED MODE").unwrap_or("").to_string()
              }),
              FONT_SIZE_MODE,
              TURN_BASED_BADGE,
              W_STRONG
            ))
            .item(reactive_text(
              signal::from_resource::<ClockData>()
                .map_in(|d| format!("{} T:{}", d.mode, d.tick)),
              FONT_SIZE_SMALL,
              MODE_LINE,
              W_STRONG
            ))
        )
    )
}

// Dialogue panel — lives inside the game pane at the bottom, no fullscreen dim
// ---------------------------------------------------------------------------

fn dialogue_panel() -> impl Element {
  El::<Node>::new()
    .with_node(|mut n| {
      n.width = Val::Percent(100.0);
    })
    .child_signal(
      signal::from_resource_changed::<OverlayData>()
        .map_in::<Option<El<Node>>, Option<El<Node>>, _>(|data: OverlayData| {
          if let Some(OverlayKind::Dialogue { title, options }) = data.kind {
            let mut lines: Vec<String> = options
              .iter()
              .enumerate()
              .map(|(i, t)| format!("{}) {}", i + 1, t))
              .collect();
            lines.push(String::new());
            lines.push("Space to cancel".into());
            Some(
              El::<Node>::new()
                .with_node(|mut n| {
                  n.width = Val::Percent(100.0);
                  n.border = UiRect::top(Val::Px(1.0));
                })
                .background_color(BackgroundColor(DIALOGUE_PANEL_BG))
                .border_color(BorderColor::all(BORDER))
                .child(
                  Column::<Node>::new()
                    .with_node(|mut n| {
                      n.width = Val::Percent(100.0);
                      n.padding = UiRect::all(Val::Px(16.));
                      n.row_gap = Val::Px(6.0);
                    })
                    .item(static_text(title, FONT_SIZE_TITLE, LIGHT_TEXT, W_STRONG))
                    .items(
                      lines
                        .into_iter()
                        .map(|l| static_text(l, FONT_SIZE_BODY, LIGHT_TEXT, W_OVERLAY))
                    )
                )
            )
          } else {
            None
          }
        })
    )
}

// Overlays — centred on top of everything
// ---------------------------------------------------------------------------

fn overlay_signal() -> impl Signal<Item = Option<impl Element>> {
  signal::from_resource_changed::<OverlayData>().map_in(|data: OverlayData| {
    // Dialogue is rendered inside the game pane by dialogue_panel() — no fullscreen overlay.
    let kind = match data.kind {
      None | Some(OverlayKind::Dialogue { .. }) => {
        return None;
      }
      Some(k) => k
    };
    Some(match kind {
      OverlayKind::Interact { options } => El::<Node>::new()
        .with_node(|mut n| {
          n.width = Val::Percent(100.);
          n.height = Val::Percent(100.0);
        })
        .background_color(BackgroundColor(OVERLAY_DIM))
        .align(Align::center())
        .align_content(Align::center())
        .child(
          Column::<Node>::new()
            .with_node(|mut n| {
              n.border_radius = BorderRadius::all(Val::Px(6.0));
              n.padding = UiRect::all(Val::Px(16.));
              n.row_gap = Val::Px(2.0);
              n.min_width = Val::Px(300.0);
            })
            .background_color(BackgroundColor(DARK_BG))
            .border_color(BorderColor::all(BORDER))
            .item(static_text("Use what?", FONT_SIZE_TITLE, LIGHT_TEXT, W_STRONG))
            .items(options.into_iter().enumerate().map(move |(i, opt)| {
              Row::<Node>::new()
                .with_node(|mut n| {
                  n.width = Val::Percent(100.0);
                  n.padding = UiRect::vertical(Val::Px(1.0));
                })
                .item(
                  El::<Text>::new()
                    .text_font(TextFont {
                      font_size: FONT_SIZE_BODY,
                      weight: W_OVERLAY,
                      ..default()
                    })
                    .text_color(TextColor(DIM_TEXT))
                    .with_builder(move |b| {
                      b.component_signal::<Text>(
                        signal::from_resource::<InteractDisplayState>().map_in(
                          move |s: InteractDisplayState| {
                            let prefix = if s.selected == i { ">" } else { " " };
                            Some(Text::new(format!("{prefix} {opt}")))
                          }
                        )
                      )
                      .component_signal::<TextColor>(
                        signal::from_resource::<InteractDisplayState>().map_in(
                          move |s: InteractDisplayState| {
                            let is_disabled = s.disabled.get(i).copied().unwrap_or(false);
                            let is_hi = s.highlighted.get(i).copied().unwrap_or(false);
                            Some(TextColor(if is_disabled {
                              DISABLED_TEXT
                            } else if is_hi {
                              EQUIP_HIGHLIGHT
                            } else if s.selected == i {
                              LIGHT_TEXT
                            } else {
                              DIM_TEXT
                            }))
                          }
                        )
                      )
                    })
                )
                .insert(Button)
                .insert(crate::MenuOptionIndex(i))
            }))
            .item(static_text("", FONT_SIZE_BODY, DIM_TEXT, W_OVERLAY))
            .item(static_text(
              "W/S navigate  A/D/Enter confirm  Space cancel",
              FONT_SIZE_SMALL,
              DIM_TEXT,
              W_OVERLAY
            ))
        ),
      OverlayKind::CraftingTable { salvage, craft } => {
        let tab_names = ["Salvage", "Craft"];
        El::<Node>::new()
          .with_node(|mut n| {
            n.width = Val::Percent(100.);
            n.height = Val::Percent(100.0);
          })
          .background_color(BackgroundColor(OVERLAY_DIM))
          .align(Align::center())
          .align_content(Align::center())
          .child(
            Column::<Node>::new()
              .with_node(|mut n| {
                n.border_radius = BorderRadius::all(Val::Px(6.0));
                n.padding = UiRect::all(Val::Px(16.));
                n.row_gap = Val::Px(2.0);
                n.min_width = Val::Px(420.0);
                n.max_height = Val::Vh(70.0);
              })
              .background_color(BackgroundColor(DARK_BG))
              .border_color(BorderColor::all(BORDER))
              .item(static_text("Crafting Table", FONT_SIZE_TITLE, LIGHT_TEXT, W_STRONG))
              .item(
                Row::<Node>::new()
                  .with_node(|mut n| {
                    n.column_gap = Val::Px(16.0);
                    n.padding = UiRect::vertical(Val::Px(4.0));
                  })
                  .items(tab_names.into_iter().enumerate().map(move |(ti, name)| {
                    El::<Text>::new()
                      .text_font(TextFont {
                        font_size: FONT_SIZE_BODY,
                        weight: W_STRONG,
                        ..default()
                      })
                      .with_builder(move |b| {
                        b.component_signal::<Text>(
                          signal::from_resource::<CraftingDisplayState>().map_in(
                            move |s: CraftingDisplayState| {
                              let marker = if s.tab == ti { "> " } else { "  " };
                              Some(Text::new(format!("{marker}{name}")))
                            }
                          )
                        )
                        .component_signal::<TextColor>(
                          signal::from_resource::<CraftingDisplayState>().map_in(
                            move |s: CraftingDisplayState| {
                              Some(TextColor(if s.tab == ti { ACCENT } else { DIM_TEXT }))
                            }
                          )
                        )
                      })
                  }))
              )
              .item(
                El::<Node>::new()
                  .with_node(|mut n| {
                    n.width = Val::Percent(100.0);
                    n.height = Val::Px(1.0);
                    n.margin = UiRect::vertical(Val::Px(4.0));
                  })
                  .background_color(BackgroundColor(BORDER))
              )
              .item(crafting_entries_column(0, salvage))
              .item(crafting_entries_column(1, craft))
              .item(
                El::<Text>::new()
                  .text_font(TextFont {
                    font_size: FONT_SIZE_SMALL,
                    weight: W_UI,
                    ..default()
                  })
                  .text_color(TextColor(DIM_TEXT))
                  .with_builder(|b| {
                    b.component_signal::<Text>(
                      signal::from_resource::<CraftingDisplayState>().map_in(
                        |s: CraftingDisplayState| {
                          let hint = if s.scroll > 0 { "  ▲ more above" } else { "" };
                          Some(Text::new(hint.to_string()))
                        }
                      )
                    )
                  })
              )
              .item(static_text("", FONT_SIZE_SMALL, DIM_TEXT, W_OVERLAY))
              .item(static_text(
                "A/D tab  W/S navigate  Enter confirm  Space close",
                FONT_SIZE_SMALL,
                DIM_TEXT,
                W_OVERLAY
              ))
          )
      }
      OverlayKind::QuestLog { entries } => {
        let mut lines: Vec<String> = Vec::new();
        if entries.is_empty() {
          lines.push("No quests.".into());
        }
        for e in &entries {
          let status = if e.completed { " [DONE]" } else if e.failed { " [FAILED]" } else { "" };
          lines.push(format!("● {}{status}", e.name));
          lines.push(format!("  {}", e.journal));
          for obj in &e.objectives {
            lines.push(format!("  - {obj}"));
          }
          lines.push(String::new());
        }
        lines.push("Q / Space to close".into());
        El::<Node>::new()
          .with_node(|mut n| {
            n.width = Val::Percent(100.);
            n.height = Val::Percent(100.0);
          })
          .background_color(BackgroundColor(OVERLAY_DIM))
          .align(Align::center())
          .align_content(Align::center())
          .child(
            Column::<Node>::new()
              .with_node(|mut n| {
                n.border_radius = BorderRadius::all(Val::Px(6.0));
                n.padding = UiRect::all(Val::Px(16.));
                n.column_gap = Val::Px(6.0);
                n.min_width = Val::Px(400.0);
              })
              .background_color(BackgroundColor(DARK_BG))
              .border_color(BorderColor::all(BORDER))
              .item(static_text("Quest Log", FONT_SIZE_TITLE, LIGHT_TEXT, W_STRONG))
              .items(
                lines.into_iter()
                  .map(|l| static_text(l, FONT_SIZE_BODY, LIGHT_TEXT, W_OVERLAY))
              )
          )
      }
      OverlayKind::CharacterCreator => character_creator_overlay(),
      kind => {
        let (label, lines) = match &kind {
          OverlayKind::PauseMain => ("Paused", vec![
            "1) Resume".into(),
            "2) Controls".into(),
            "3) Quit Game".into(),
            String::new(),
            "Space to resume".into(),
          ]),
          OverlayKind::PauseControls => ("Controls", vec![
            "WASD / Arrows   move".into(),
            "Space           use / interact".into(),
            ".               wait".into(),
            "?               controls".into(),
            "Q               quest log".into(),
            "Tab             pause menu".into(),
          ]),
          _ => ("", vec![])
        };
        El::<Node>::new()
          .with_node(|mut n| {
            n.width = Val::Percent(100.);
            n.height = Val::Percent(100.0);
          })
          .background_color(BackgroundColor(OVERLAY_DIM))
          .align(Align::center())
          .align_content(Align::center())
          .child(
            Column::<Node>::new()
              .with_node(|mut n| {
                n.border_radius = BorderRadius::all(Val::Px(6.0));
                n.padding = UiRect::all(Val::Px(16.));
                n.column_gap = Val::Px(6.0);
              })
              .background_color(BackgroundColor(DARK_BG))
              .border_color(BorderColor::all(BORDER))
              .item(static_text(label, FONT_SIZE_TITLE, LIGHT_TEXT, W_STRONG))
              .items(
                lines
                  .into_iter()
                  .map(|l| static_text(l, FONT_SIZE_BODY, LIGHT_TEXT, W_OVERLAY))
              )
          )
      }
    })
  })
}

// ---------------------------------------------------------------------------
// Character creator overlay
// ---------------------------------------------------------------------------

const CREATOR_ROW_HI: Color = Color::srgb(0.55, 0.88, 0.65);
const CREATOR_ROW_DIM: Color = Color::srgb(0.62, 0.66, 0.74);
const CREATOR_CONFIRM: Color = Color::srgb(1.0, 0.85, 0.30);

fn character_creator_overlay() -> El<Node> {
  El::<Node>::new()
    .with_node(|mut n| {
      n.width = Val::Percent(100.0);
      n.height = Val::Percent(100.0);
    })
    .background_color(BackgroundColor(OVERLAY_DIM))
    .align(Align::center())
    .align_content(Align::center())
    .child(
      Column::<Node>::new()
        .with_node(|mut n| {
          n.border_radius = BorderRadius::all(Val::Px(6.0));
          n.padding = UiRect::all(Val::Px(20.0));
          n.row_gap = Val::Px(8.0);
          n.min_width = Val::Px(520.0);
          n.border = UiRect::all(Val::Px(1.0));
        })
        .background_color(BackgroundColor(DARK_BG))
        .border_color(BorderColor::all(BORDER))
        .item(static_text("Create Character", FONT_SIZE_TITLE, LIGHT_TEXT, W_STRONG))
        .item(static_text("", FONT_SIZE_SMALL, DIM_TEXT, W_OVERLAY))
        .item(panel_label("Name"))
        .item(
          TextInput::new()
            .with_node(|mut n| {
              n.width = Val::Percent(100.0);
              n.height = Val::Px(30.0);
              n.padding = UiRect::axes(Val::Px(6.0), Val::Px(0.0));
              n.border = UiRect::all(Val::Px(1.0));
              n.border_radius = BorderRadius::all(Val::Px(3.0));
            })
            .text_font(TextFont { font_size: FONT_SIZE_BODY, weight: W_UI, ..default() })
            .text_color(TextColor(LIGHT_TEXT))
            .text_input_prompt(TextInputPrompt::new("Enter your name…"))
            .focus()
            .on_change(|In((_, text)): In<(Entity, String)>, mut name: ResMut<CreatorName>| {
              name.0 = text;
            })
        )
        .item(static_text("", FONT_SIZE_SMALL, DIM_TEXT, W_OVERLAY))
        .item(panel_label("Starting Items (pick up to 3)"))
        .items(
          STARTING_ITEMS.iter().enumerate().map(|(i, item)| {
            creator_row(CreatorOption::Item(i), format!("{} — {}", item.name(), item_glyph(item)))
          })
        )
        .item(static_text("", FONT_SIZE_SMALL, DIM_TEXT, W_OVERLAY))
        .item(panel_label("Special Ability"))
        .items(
          SPECIAL_ABILITIES.iter().enumerate().map(|(i, ability)| {
            creator_row(CreatorOption::Ability(i), format!("{} — {}", ability.name, ability.flavor))
          })
        )
        .item(static_text("", FONT_SIZE_SMALL, DIM_TEXT, W_OVERLAY))
        .item(
          El::<Node>::new()
            .with_node(|mut n| {
              n.width = Val::Percent(100.0);
              n.height = Val::Px(36.0);
              n.border = UiRect::all(Val::Px(1.0));
              n.border_radius = BorderRadius::all(Val::Px(3.0));
              n.align_items = AlignItems::Center;
              n.justify_content = JustifyContent::Center;
            })
            .background_color(BackgroundColor(Color::srgb(0.10, 0.12, 0.08)))
            .border_color(BorderColor::all(CREATOR_CONFIRM))
            .with_builder(|b| {
              b.component_signal::<TextColor>(
                signal::from_resource::<CharacterCreatorData>().map_in(|_| Some(TextColor(CREATOR_CONFIRM)))
              )
            })
            .child(
              El::<Text>::new()
                .text(Text::new("Begin ▸"))
                .text_font(TextFont { font_size: FONT_SIZE_TITLE, weight: W_STRONG, ..default() })
                .text_color(TextColor(CREATOR_CONFIRM))
            )
            .insert(Button)
            .insert(CreatorOptionIndex(CreatorOption::Confirm))
        )
        .item(static_text(
          "W/S move cursor  ·  Space toggle item  ·  A/D pick ability  ·  Tab/Enter begin",
          FONT_SIZE_SMALL,
          DIM_TEXT,
          W_OVERLAY
        ))
    )
}

fn item_glyph(item: &crate::level::Item) -> &'static str {
  item.glyph()
}

fn creator_row(option: CreatorOption, label: String) -> Row<Node> {
  Row::<Node>::new()
    .with_node(|mut n| {
      n.width = Val::Percent(100.0);
      n.padding = UiRect::vertical(Val::Px(2.0));
    })
    .item(
      El::<Text>::new()
        .text_font(TextFont { font_size: FONT_SIZE_BODY, weight: W_OVERLAY, ..default() })
        .with_builder(move |b| {
          b.component_signal::<Text>(
            signal::from_resource::<CharacterCreatorData>().map_in(move |data: CharacterCreatorData| {
              let prefix = row_prefix(&option, &data);
              Some(Text::new(format!("{prefix}{label}")))
            })
          )
          .component_signal::<TextColor>(
            signal::from_resource::<CharacterCreatorData>().map_in(move |data: CharacterCreatorData| {
              Some(TextColor(row_color(&option, &data)))
            })
          )
        })
    )
    .insert(Button)
    .insert(CreatorOptionIndex(option))
}

/// Prefix shown before each creator row: cursor + checkmark for items, cursor for abilities.
fn row_prefix(option: &CreatorOption, data: &CharacterCreatorData) -> String {
  match *option {
    CreatorOption::Item(i) => {
      let cursor = if data.cursor_item == i { "▶" } else { " " };
      let mark = if data.selected_items.contains(&i) { "☑" } else { "☐" };
      format!("{cursor}{mark} ")
    }
    CreatorOption::Ability(i) => {
      let cursor = if data.selected_ability == i { "▶" } else { " " };
      format!("{cursor}  ")
    }
    CreatorOption::Confirm => String::new()
  }
}

/// Row text color: highlighted when selected/cursor-active, dimmed otherwise.
fn row_color(option: &CreatorOption, data: &CharacterCreatorData) -> Color {
  match *option {
    CreatorOption::Item(i) => {
      if data.selected_items.contains(&i) { CREATOR_ROW_HI }
      else if data.cursor_item == i { LIGHT_TEXT }
      else { CREATOR_ROW_DIM }
    }
    CreatorOption::Ability(i) => {
      if data.selected_ability == i { CREATOR_ROW_HI } else { CREATOR_ROW_DIM }
    }
    CreatorOption::Confirm => CREATOR_CONFIRM
  }
}

#[derive(Resource, Clone, Default, PartialEq)]
pub struct OverlayData {
  pub kind: Option<OverlayKind>
}

// ---------------------------------------------------------------------------
// sync_ui — reads Bevy world, writes signal resources
// ---------------------------------------------------------------------------

fn sync_crafting_display(
  ui: Res<crate::UiState>,
  mut state: ResMut<CraftingDisplayState>
) {
  let new = if let &crate::CraftingMenu::Open { tab, selected, scroll, .. } = &ui.crafting
  {
    CraftingDisplayState { tab, selected, scroll }
  } else {
    CraftingDisplayState::default()
  };
  if *state != new {
    *state = new;
  }
}

fn sync_interact_display(
  ui: Res<crate::UiState>,
  mut state: ResMut<InteractDisplayState>
) {
  let new = if let crate::InteractMenu::Open { selected, highlighted, disabled, .. } =
    &ui.interact
  {
    InteractDisplayState {
      selected: *selected,
      highlighted: highlighted.clone(),
      disabled: disabled.clone()
    }
  } else {
    InteractDisplayState::default()
  };
  if *state != new {
    *state = new;
  }
}

fn sync_ui(
  clock: Res<Clock>,
  player_q: Query<
    (
      &Location,
      &Stats,
      &crate::Inventory,
      &Loadout,
      Option<&crate::entities::Grabbed>,
      Option<&crate::entities::Invisible>
    ),
    With<crate::Player>
  >,
  ui: Res<crate::UiState>,
  current: Res<crate::CurrentZone>,
  fov: Res<crate::Fov>,
  index: Res<crate::combat::TileEntityIndex>,
  named_q: Query<(&Named, Option<&Stats>, Option<&crate::entities::Corpse>)>,
  item_glyph_q: Query<&crate::ItemGlyph>,
  windows: Query<&Window>,
  camera_q: Query<(&Camera, &GlobalTransform), With<crate::post_process::GameCamera>>,
  (mut clock_data, mut player_data, mut hover_info, mut inv_display, mut overlay): (
    ResMut<ClockData>,
    ResMut<PlayerData>,
    ResMut<HoverInfo>,
    ResMut<InvDisplayData>,
    ResMut<OverlayData>
  ),
  res_log: Res<LogEntries>,
  mut log_display: ResMut<LogDisplayData>,
  quest_log: Res<crate::quest::QuestLog>
) {
  // ── Clock ──
  *clock_data = ClockData {
    mode: match clock.mode {
      crate::TimeMode::RealTime => "RT",
      crate::TimeMode::TurnBased => "TB"
    },
    tick: clock.time
  };

  // ── Player stats ──
  if let Ok((pos, stats, inv, loadout, grabbed, invisible)) = player_q.single()
    && let &Location::Coords { x: px, y: py, z: pz, .. } = pos
  {
    let mut effects = Vec::new();
    if let Some(g) = grabbed {
      effects.push(format!("GRABBED ({})", g.turns_remaining));
    }
    if let Some(i) = invisible {
      effects.push(format!("INVISIBLE ({})", i.0));
    }
    *player_data = PlayerData {
      hp: stats.hp,
      max_hp: stats.max_hp,
      attack: stats.attack + loadout.weapon_attack_bonus(),
      speed: stats.move_speed,
      x: px,
      y: py,
      z: pz,
      equipped_weapon: loadout.weapon().map(|w| w.name().to_string()),
      equipped_armor: loadout.armor_item().map(|a| a.name().to_string()),
      status_effects: effects
    };

    inv_display.formatted = if inv.0.is_empty() {
      "(empty)".into()
    } else {
      mapv(
        |(item, count): (&crate::level::Item, &u32)| {
          format!("{}x {}", count, item.name())
        },
        &inv.0
      )
      .join("\n")
    };
  }

  // ── Hover info ──
  *hover_info = compute_hover_info(
    &windows,
    &camera_q,
    &current,
    player_q,
    &fov,
    &index,
    &named_q,
    &item_glyph_q
  );

  // ── Overlay state — only write when the value actually changes ──
  let new_overlay_kind = match ui.pause {
    crate::PauseMenu::Closed => None,
    crate::PauseMenu::Main => Some(OverlayKind::PauseMain),
    crate::PauseMenu::Controls => Some(OverlayKind::PauseControls)
  }
  .or_else(|| match &ui.interact {
    crate::InteractMenu::Open { options, .. } => {
      Some(OverlayKind::Interact { options: mapv(|o| o.label.clone(), options) })
    }
    crate::InteractMenu::Closed => None
  })
  .or_else(|| match &ui.crafting {
    crate::CraftingMenu::Open { salvage_entries, craft_entries, .. } => {
      Some(OverlayKind::CraftingTable {
        salvage: salvage_entries.clone(),
        craft: craft_entries.clone()
      })
    }
    crate::CraftingMenu::Closed => None
  })
  .or_else(|| match &ui.dialogue {
    crate::DialogueState::Open { speaker, tree, node_name, .. } => {
      let visible = tree.visible_choices(node_name, &quest_log);
      let options: Vec<String> = visible.iter().map(|c| c.text.to_string()).collect();
      Some(OverlayKind::Dialogue {
        title: format!("What do you say? ({speaker})"),
        options
      })
    }
    crate::DialogueState::Closed => None
  })
  .or_else(|| if ui.quest_log_open {
    let entries = quest_log.all_quests().iter().map(|&(id, name, completed, failed)| {
      QuestLogEntry {
        name: name.to_string(),
        journal: quest_log.journal(id).unwrap_or("").to_string(),
        objectives: quest_log.objectives(id).iter().map(|s| {
          if id == crate::quest::ALIEN_HUNT.id && s.contains("/10") {
            let kills = quest_log.flag(id, crate::quest::ALIEN_HUNT_KILL_FLAG);
            format!("Kill aliens ({kills}/10)")
          } else {
            s.to_string()
          }
        }).collect(),
        completed,
        failed,
      }
    }).collect();
    Some(OverlayKind::QuestLog { entries })
  } else {
    None
  })
  .or_else(|| if ui.creator_open {
    Some(OverlayKind::CharacterCreator)
  } else {
    None
  });
  if overlay.kind != new_overlay_kind {
    overlay.kind = new_overlay_kind;
  }

  // ── Log: oldest at top, newest at bottom; keep last 50 lines ──
  {
    let n = res_log.0.len();
    let start = n.saturating_sub(50);
    let new_lines: Vec<LogLine> = if n == 0 {
      vec![vec![LogSpan::plain("\u{2014}")]]
    } else {
      res_log.0[start..].to_vec()
    };
    if log_display.0 != new_lines {
      log_display.0 = new_lines;
    }
  }
}

fn compute_hover_info(
  windows: &Query<&Window>,
  camera_q: &Query<(&Camera, &GlobalTransform), With<crate::post_process::GameCamera>>,
  current: &crate::CurrentZone,
  player_q: Query<
    (
      &Location,
      &Stats,
      &crate::Inventory,
      &Loadout,
      Option<&crate::entities::Grabbed>,
      Option<&crate::entities::Invisible>
    ),
    With<crate::Player>
  >,
  fov: &crate::Fov,
  index: &crate::combat::TileEntityIndex,
  named_q: &Query<(&Named, Option<&Stats>, Option<&crate::entities::Corpse>)>,
  item_glyph_q: &Query<&crate::ItemGlyph>
) -> HoverInfo {
  let empty = HoverInfo {
    coords: (0, 0),
    tile_name: "—".into(),
    item_name: None,
    entity_name: None,
    entity_hp: None,
    flavor: None
  };

  if let Ok(window) = windows.single()
    && let Ok((camera, cam_tf)) = camera_q.single()
    && let Ok((ploc, _, _, _, _, _)) = player_q.single()
    && let Location::Coords { z: pz, .. } = *ploc
  {
    let level = current.0.level(pz);
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
    if let Some((tx, ty)) = pick(window, camera, cam_tf, level.width, level.height) {
      let visible = fov.0.is_visible(tx as usize, ty as usize);
      if visible {
        let tile = level.tiles[ty as usize][tx as usize];
        let tile_name = tile.name().to_string();

        // Look for entity on this tile
        let (entity_name, entity_hp, flavor) = if visible {
          index
            .0
            .get(&(tx, ty, pz))
            .and_then(|entities| {
              entities.iter().find_map(|&e| {
                named_q.get(e).ok().map(|(named, stats, corpse)| {
                  let is_corpse = corpse.is_some();
                  (
                    Some(if is_corpse {
                      format!("dead {}", named.name)
                    } else {
                      named.name.to_string()
                    }),
                    stats.map(|s| (s.hp, s.max_hp)),
                    if is_corpse { None } else { Some(named.flavor.to_string()) }
                  )
                })
              })
            })
            .unwrap_or((None, None, None))
        } else {
          (None, None, None)
        };

        let item_name = {
          let mut items: Vec<_> = item_glyph_q
            .iter()
            .filter(|ig| ig.x == tx as usize && ig.y == ty as usize && ig.z == pz)
            .map(|ig| ig.item)
            .collect();
          if items.is_empty() {
            None
          } else {
            items.sort_by_key(|i| i.name());
            let mut parts = Vec::new();
            let mut i = 0;
            while i < items.len() {
              let item = items[i];
              let count = items[i..].iter().take_while(|&&it| it == item).count();
              parts.push(if count > 1 {
                format!("{} x{}", item.name(), count)
              } else {
                item.name().to_string()
              });
              i += count;
            }
            Some(parts.join(", "))
          }
        };
        HoverInfo {
          coords: (tx, ty),
          tile_name,
          item_name,
          entity_name,
          entity_hp,
          flavor
        }
      } else {
        empty
      }
    } else {
      empty
    }
  } else {
    empty
  }
}

// ---------------------------------------------------------------------------
// Reusable UI helpers
// ---------------------------------------------------------------------------

fn panel_label(text: &str) -> impl Element {
  static_text(text, FONT_SIZE_LABEL, ACCENT, W_STRONG)
}

fn static_text(
  text: impl Into<String>,
  size: f32,
  color: Color,
  weight: FontWeight
) -> impl Element {
  El::<Text>::new()
    .text(Text::new(text))
    .text_font(TextFont { font_size: size, weight, ..default() })
    .text_color(TextColor(color))
}

fn reactive_text(
  sig: impl Signal<Item = String> + Clone + 'static,
  size: f32,
  color: Color,
  weight: FontWeight
) -> impl Element {
  El::<Text>::new()
    .text_font(TextFont { font_size: size, weight, ..default() })
    .text_color(TextColor(color))
    .with_builder(|b| b.component_signal::<Text>(sig.map_in(|s| Some(Text::new(s)))))
}

fn crafting_entries_column(
  tab_index: usize,
  entries: Vec<CraftingEntry>
) -> Column<Node> {
  let col = Column::<Node>::new()
    .with_node(|mut n| {
      n.width = Val::Percent(100.0);
    })
    .with_builder(move |b| {
      b.component_signal::<Node>(signal::from_resource::<CraftingDisplayState>().map_in(
        move |s: CraftingDisplayState| {
          let mut node = Node::default();
          node.width = Val::Percent(100.0);
          node.flex_direction = FlexDirection::Column;
          node.display = if s.tab == tab_index { Display::Flex } else { Display::None };
          Some(node)
        }
      ))
    });
  if entries.is_empty() {
    col.item(static_text("  (nothing available)", FONT_SIZE_BODY, DIM_TEXT, W_OVERLAY))
  } else {
    col.items(
      entries
        .into_iter()
        .enumerate()
        .map(|(i, entry)| crafting_row(i, entry.label, entry.detail, entry.craftable))
    )
  }
}

const CRAFT_AVAILABLE: Color = Color::srgb(0.55, 0.88, 0.65);
const CRAFT_UNAVAILABLE: Color = Color::srgb(0.60, 0.55, 0.50);
const CRAFT_DETAIL: Color = Color::srgb(0.68, 0.65, 0.58);

fn crafting_row(
  i: usize,
  label: String,
  detail: String,
  craftable: bool
) -> Column<Node> {
  let base_color = if craftable { CRAFT_AVAILABLE } else { CRAFT_UNAVAILABLE };
  Column::<Node>::new()
    .with_node(|mut n| {
      n.width = Val::Percent(100.0);
      n.padding = UiRect::new(Val::Px(4.0), Val::Px(4.0), Val::Px(1.0), Val::Px(1.0));
    })
    .with_builder(move |b| {
      b.component_signal::<Node>(signal::from_resource::<CraftingDisplayState>().map_in(
        move |s: CraftingDisplayState| {
          let visible = i >= s.scroll && i < s.scroll + crate::CRAFT_VISIBLE_ROWS;
          let mut node = Node::default();
          node.width = Val::Percent(100.0);
          node.padding =
            UiRect::new(Val::Px(4.0), Val::Px(4.0), Val::Px(1.0), Val::Px(1.0));
          node.flex_direction = FlexDirection::Column;
          node.display = if visible { Display::Flex } else { Display::None };
          Some(node)
        }
      ))
    })
    .item(
      El::<Text>::new()
        .text_font(TextFont { font_size: FONT_SIZE_BODY, weight: W_OVERLAY, ..default() })
        .with_builder(move |b| {
          b.component_signal::<Text>(
            signal::from_resource::<CraftingDisplayState>().map_in(
              move |s: CraftingDisplayState| {
                let prefix = if s.selected == i { "> " } else { "  " };
                Some(Text::new(format!("{prefix}{label}")))
              }
            )
          )
          .component_signal::<TextColor>(
            signal::from_resource::<CraftingDisplayState>().map_in(
              move |s: CraftingDisplayState| {
                Some(TextColor(if s.selected == i { LIGHT_TEXT } else { base_color }))
              }
            )
          )
        })
    )
    .item(
      El::<Text>::new()
        .text(Text::new(format!("    {detail}")))
        .text_font(TextFont { font_size: FONT_SIZE_SMALL, weight: W_UI, ..default() })
        .text_color(TextColor(CRAFT_DETAIL))
    )
}
