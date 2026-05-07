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
//! - **Layer 3 — [`world_map_signal`]**: fullscreen dim + map panel when the world map is open.

use {
  crate::{
    utils::mapv,
    Clock, try_pick_level_tile_at_cursor, GAME_VIEWPORT_WIDTH_FRAC, STATUS_BAR_HEIGHT,
  },
  bevy::prelude::*,
  bevy::text::FontWeight,
  haalka::jonmo::SignalProcessing,
  haalka::prelude::*,
  jonmo::{signal, prelude::*},
  trl::entities::{Stats, Named},
};

// ---------------------------------------------------------------------------
// Data shapes — written by sync_ui, read by Haalka signals
// ---------------------------------------------------------------------------

#[derive(Resource, Clone)]
pub struct ClockData {
  pub mode: &'static str,
  pub tick: u64,
}

impl Default for ClockData {
  fn default() -> Self {
    Self { mode: "RT", tick: 0 }
  }
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
}

#[derive(Resource, Clone, Default)]
pub struct HoverInfo {
  pub coords: (i32, i32),
  pub tile_name: String,
  pub entity_name: Option<String>,
  pub entity_hp: Option<(i32, i32)>,
  pub flavor: Option<String>,
}

/// Accumulated messages; capped at 100 entries. Updated by game systems in `Update`.
#[derive(Resource, Clone, Default)]
pub struct LogEntries(pub Vec<String>);

/// Log body for the sidebar, like [`InvDisplayData`]: one string set in [`sync_ui`], read by
/// `reactive_text` (avoids `item_signal` + nested `Column` issues in Jonmo).
#[derive(Resource, Clone, Default)]
pub struct LogDisplayData {
  pub text: String,
}

/// Push one line or multiline block; oldest entries are dropped to keep at most 100.
pub fn log_message(log: &mut LogEntries, line: String) {
  const MAX: usize = 100;
  while log.0.len() >= MAX {
    log.0.remove(0);
  }
  log.0.push(line);
}

/// Full-island map texture and visibility (game viewport overlay). Filled in `setup`; toggled with M.
#[derive(Resource, Clone)]
pub struct WorldMapView {
  pub open: bool,
  pub image: Handle<Image>,
}

impl Default for WorldMapView {
  fn default() -> Self {
    Self { open: false, image: Handle::default() }
  }
}

#[derive(Resource, Clone, Debug, PartialEq)]
pub enum OverlayKind {
  PauseMain,
  PauseControls,
  /// Numbered option labels, same format as `Interact` (1) text …).
  Interact(Vec<String>),
  /// While talking: show numbered replies (1) text …) over the playfield.
  Dialogue { title: String, options: Vec<String> },
}

/// Formatted inventory string, updated by sync_ui.
#[derive(Resource, Clone, Default)]
pub struct InvDisplayData {
  pub formatted: String,
}

// ---------------------------------------------------------------------------
// Colours & constants
// ---------------------------------------------------------------------------

const DARK_BG:     Color = Color::srgb(0.10, 0.10, 0.18);
const PANEL_BG:    Color = Color::srgb(0.12, 0.12, 0.20);
const BORDER:      Color = Color::srgb(0.20, 0.20, 0.33);
const LIGHT_TEXT:  Color = Color::srgb(0.94, 0.94, 0.97);
const DIM_TEXT:    Color = Color::srgb(0.78, 0.80, 0.86);
const ACCENT:      Color = Color::srgb(0.55, 0.88, 0.65);
/// Real-time / turn line in sidebar (mode name + tick).
const MODE_LINE:   Color = Color::srgb(0.65, 0.95, 0.78);
/// “TURN-BASED MODE” banner in the status bar (high contrast).
const TURN_BASED_BADGE: Color = Color::srgb(1.0, 0.88, 0.35);
const HP_GREEN:    Color = Color::srgb(0.35, 0.75, 0.35);
const HP_YELLOW:   Color = Color::srgb(0.85, 0.75, 0.25);
const HP_RED:      Color = Color::srgb(0.85, 0.30, 0.30);
const OVERLAY_DIM: Color = Color::srgba(0.0, 0.0, 0.0, 0.50);

const FONT_SIZE_LABEL: f32 = 15.0;
const FONT_SIZE_BODY:  f32 = 17.0;
const FONT_SIZE_TITLE: f32 = 18.0;
const FONT_SIZE_SMALL: f32 = 14.0;
/// Sidebar clock mode + TB banner (slightly larger than body elsewhere).
const FONT_SIZE_MODE: f32 = 15.5;

const W_UI: FontWeight = FontWeight::SEMIBOLD;
const W_STRONG: FontWeight = FontWeight::BOLD;
const W_OVERLAY: FontWeight = FontWeight::MEDIUM;

const PANEL_PAD:      f32 = 8.0;

/// Match `main.rs` Haalka row: sidebar width as a percent of the window width.
const SIDEBAR_WIDTH_PERCENT: f32 = (1.0 - GAME_VIEWPORT_WIDTH_FRAC) * 100.0;
const GAME_VIEWPORT_PERCENT: f32 = GAME_VIEWPORT_WIDTH_FRAC * 100.0;

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

pub struct UiPlugin;

impl Plugin for UiPlugin {
  fn build(&self, app: &mut App) {
    app.init_resource::<ClockData>()
      .init_resource::<PlayerData>()
      .init_resource::<HoverInfo>()
      .init_resource::<LogEntries>()
      .init_resource::<LogDisplayData>()
      .init_resource::<InvDisplayData>()
    .init_resource::<OverlayData>()
    .init_resource::<WorldMapView>()
      .add_systems(PostUpdate, sync_ui.before(SignalProcessing));
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
    .layer_signal(world_map_signal())
}

pub fn spawn_haalka_root(world: &mut World) {
  build_ui_root().spawn(world);
}

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
          El::<Node>::new()
            .with_node(|mut n| {
              n.flex_grow = 1.0;
              n.flex_shrink = 1.0;
              n.min_width = Val::Px(0.0);
            })
        )
        // Sidebar column — fixed fraction of window width, flush right
        .item(sidebar_column())
    )
    // ── bottom: status bar ──
    .item(status_bar())
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
    .item(stat_row("ATK", signal::from_resource::<PlayerData>().map_in(|d| d.attack.to_string())))
    .item(stat_row("SPD", signal::from_resource::<PlayerData>().map_in(|d| format!("{:.1}", d.speed))))
    .item(z_level_label())
    .item(time_mode_label())
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
              let color = if ratio > 0.66 { HP_GREEN } else if ratio > 0.33 { HP_YELLOW } else { HP_RED };
              Some(
                El::<Node>::new()
                  .with_node(move |mut n| { n.width = Val::Percent(pct); n.height = Val::Percent(100.0); })
                  .background_color(BackgroundColor(color))
              )
            })
        )
    )
    // "HP/max" text
    .item(reactive_text(
      signal::from_resource::<PlayerData>().map_in(|d| format!("{}/{}", d.hp, d.max_hp)),
      FONT_SIZE_SMALL, LIGHT_TEXT, W_UI
    ))
}

fn stat_row(label: &str, value_sig: impl Signal<Item = String> + Clone + 'static) -> impl Element {
  Row::<Node>::new()
    .with_node(|mut n| {
      n.width = Val::Percent(100.);
      n.align_items = AlignItems::Center;
      n.justify_content = JustifyContent::SpaceBetween;
    })
    .item(static_text(label, FONT_SIZE_SMALL, DIM_TEXT, W_UI))
    .item(reactive_text(value_sig, FONT_SIZE_SMALL, LIGHT_TEXT, W_UI))
}

fn z_level_label() -> impl Element {
  reactive_text(
    signal::from_resource::<PlayerData>().map_in(|d| {
      let name = match d.z {
        0 => "Deep Cave",
        1 => "Shallow Cave",
        2 => "Surface",
        3 => "Building Upper",
        z => return format!("Level {}", z),
      };
      format!("{} (z={})", name, d.z)
    }),
    FONT_SIZE_SMALL, DIM_TEXT, W_UI
  )
}

fn time_mode_label() -> impl Element {
  reactive_text(
    signal::from_resource::<ClockData>().map_in(|d| {
      let icon = match d.mode {
        "RT" => "[Real Time]",
        "TB" => "[Turn Based]",
        m => m,
      };
      format!("{} T:{:.0}", icon, d.tick)
    }),
    FONT_SIZE_MODE, MODE_LINE, W_STRONG
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
    signal::from_resource::<HoverInfo>().map_in(|h| format!("({}, {})", h.coords.0, h.coords.1)),
    FONT_SIZE_SMALL, DIM_TEXT, W_UI
  )
}

fn hover_tile_line() -> impl Element {
  reactive_text(
    signal::from_resource::<HoverInfo>().map_in(|h| h.tile_name.clone()),
    FONT_SIZE_BODY, LIGHT_TEXT, W_UI
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
  h.entity_name.as_deref().map_or_else(String::new, |name| {
    let mut s = name.to_string();
    if let Some((hp, max)) = h.entity_hp {
      s.push('\n');
      s.push_str(&format_hp_line(hp, max));
    }
    if let Some(ref f) = h.flavor {
      s.push('\n');
      s.push_str(f);
    }
    s
  })
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
      n.overflow = Overflow::scroll_y();
      n.column_gap = Val::Px(1.0);
    })
    .background_color(BackgroundColor(Color::NONE))
    .border_color(BorderColor::all(Color::NONE))
    .item(panel_label("Log"))
    .item(
      // Same pattern as `inventory_list`: one `reactive_text` driven by a sync-only resource.
      El::<Node>::new()
        .with_node(|mut n| { n.width = Val::Percent(100.0); n.min_height = Val::Px(4.0); })
        .child(
          reactive_text(
            signal::from_resource::<LogDisplayData>().map_in(|d| d.text.clone()),
            FONT_SIZE_SMALL, DIM_TEXT, W_UI
          )
        )
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
        .with_node(|mut n| { n.align_items = AlignItems::Center; n.column_gap = Val::Px(16.0); })
        .item(reactive_text(
          signal::from_resource::<HoverInfo>().map_in(|h| h.tile_name.clone()),
          FONT_SIZE_SMALL, DIM_TEXT, W_UI
        ))
        .item(reactive_text(
          signal::from_resource::<HoverInfo>().map_in(|h| format!("({},{})", h.coords.0, h.coords.1)),
          FONT_SIZE_SMALL, DIM_TEXT, W_UI
        ))
        .item(
          Row::<Node>::new()
            .with_node(|mut n| { n.column_gap = Val::Px(10.0); n.align_items = AlignItems::Center; })
            .item(reactive_text(
              signal::from_resource::<ClockData>().map_in(|d| {
                (d.mode == "TB").then_some("TURN-BASED MODE").unwrap_or("").to_string()
              }),
              FONT_SIZE_MODE, TURN_BASED_BADGE, W_STRONG
            ))
            .item(reactive_text(
              signal::from_resource::<ClockData>().map_in(|d| format!("{} T:{}", d.mode, d.tick)),
              FONT_SIZE_SMALL, MODE_LINE, W_STRONG
            ))
        )
    )
}

// World map — same column width as the game view, shows full generated island
// ---------------------------------------------------------------------------

fn world_map_signal() -> impl Signal<Item = Option<impl Element>> {
  signal::from_resource::<WorldMapView>().map_in(|m| {
    m.open.then_some(
      El::<Node>::new()
        .with_node(|mut n| {
          n.width = Val::Percent(100.);
          n.height = Val::Percent(100.);
          n.position_type = PositionType::Absolute;
          n.align_items = AlignItems::Center;
          n.justify_content = JustifyContent::Center;
        })
        .background_color(BackgroundColor(OVERLAY_DIM))
        .child(
          Column::<Node>::new()
            .with_node(|mut n| {
              n.width = Val::Percent(GAME_VIEWPORT_PERCENT);
              n.max_height = Val::Percent(96.);
              n.padding = UiRect::all(Val::Px(14.));
              n.column_gap = Val::Px(8.0);
              n.align_items = AlignItems::Center;
            })
            .background_color(BackgroundColor(DARK_BG))
            .border_color(BorderColor::all(BORDER))
            .item(static_text("World map", FONT_SIZE_TITLE, ACCENT, W_STRONG))
            .item(
              // Square region: the texture is 1:1; avoid non-square flex slots (they caused Stretch to flatten).
              El::<Node>::new()
                .with_node(|mut n| {
                  n.width = Val::Percent(100.);
                  n.aspect_ratio = Some(1.0);
                })
                .align(Align::center())
                .child(
                  El::<ImageNode>::new()
                    .with_node(|mut n| {
                      n.width = Val::Percent(100.);
                      n.height = Val::Percent(100.);
                    })
                    .with_builder(|builder| {
                      builder.on_spawn_with_system(
                        |In(entity): In<Entity>, map: Res<WorldMapView>, mut commands: Commands| {
                          if let Ok(mut e) = commands.get_entity(entity) {
                            e.insert(ImageNode::new(map.image.clone()));
                          }
                        },
                      )
                    })
                )
            )
            .item(static_text("M  or  Esc  to close  ·  one pixel = one world tile", FONT_SIZE_SMALL, DIM_TEXT, W_OVERLAY))
        )
    )
  })
}

// Overlays — centred on top of everything
// ---------------------------------------------------------------------------

fn overlay_signal() -> impl Signal<Item = Option<impl Element>> {
  signal::from_resource::<OverlayData>()
    .map_in(|data| {
      data.kind.as_ref().map(|kind| {
        let label = match kind {
          OverlayKind::PauseMain => "Paused",
          OverlayKind::PauseControls => "Controls",
          OverlayKind::Interact(_) => "Use what?",
          OverlayKind::Dialogue { title, .. } => title,
        };
        let lines: Vec<String> = match kind {
          OverlayKind::Dialogue { options, .. } => {
            // Same layout as Interact: numbered options, empty line, Esc.
            let mut l: Vec<String> = options
              .iter()
              .enumerate()
              .map(|(i, t)| format!("{}) {}", i + 1, t))
              .collect();
            l.push(String::new());
            l.push("Esc to cancel".into());
            l
          }
          OverlayKind::PauseMain => vec![
            "1) Resume".into(),
            "2) Controls".into(),
            "3) Quit Game".into(),
            String::new(),
            "Esc to resume".into(),
          ],
          OverlayKind::PauseControls => vec![
            "WASD / Arrows   move".into(),
            "Space           use / interact".into(),
            ".               wait".into(),
            "?               controls".into(),
            "Esc             menu / back".into(),
          ],
          OverlayKind::Interact(opts) => opts.iter()
            .enumerate()
            .map(|(i, o)| format!("{}) {}", i + 1, o))
            .chain(core::iter::once(String::new()))
            .chain(core::iter::once("Esc to cancel".into()))
            .collect(),
        };

        El::<Node>::new()
          .with_node(|mut n| { n.width = Val::Percent(100.); n.height = Val::Percent(100.0); })
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
              .items(lines.into_iter().map(|l| static_text(l, FONT_SIZE_BODY, LIGHT_TEXT, W_OVERLAY)))
          )
      })
    })
  }

#[derive(Resource, Clone, Default)]
pub struct OverlayData {
  pub kind: Option<OverlayKind>,
}

// ---------------------------------------------------------------------------
// sync_ui — reads Bevy world, writes signal resources
// ---------------------------------------------------------------------------

fn sync_ui(
  clock: Res<Clock>,
  player_q: Query<(&crate::PlayerPos, &Stats, &crate::Inventory), With<crate::Player>>,
  ui: Res<crate::UiState>,
  current: Res<crate::CurrentZone>,
  fov: Res<crate::Fov>,
  index: Res<crate::combat::TileEntityIndex>,
  named_q: Query<(&Named, Option<&Stats>)>,
  windows: Query<&Window>,
  camera_q: Query<(&Camera, &GlobalTransform), With<Camera2d>>,
  mut clock_data: ResMut<ClockData>,
  mut player_data: ResMut<PlayerData>,
  mut hover_info: ResMut<HoverInfo>,
  mut inv_display: ResMut<InvDisplayData>,
  mut overlay: ResMut<OverlayData>,
  res_log: Res<LogEntries>,
  mut log_display: ResMut<LogDisplayData>,
) {
  // ── Clock ──
  *clock_data = ClockData {
    mode: match clock.mode {
      crate::TimeMode::RealTime => "RT",
      crate::TimeMode::TurnBased => "TB",
    },
    tick: clock.time,
  };

  // ── Player stats ──
  if let Ok((pos, stats, inv)) = player_q.single() {
    *player_data = PlayerData {
      hp: stats.hp,
      max_hp: stats.max_hp,
      attack: stats.attack,
      speed: stats.move_speed,
      x: pos.x,
      y: pos.y,
      z: pos.z,
    };

    inv_display.formatted = if inv.0.is_empty() {
      "(empty)".into()
    } else {
      mapv(|(item, count)| format!("{}x {}", count, item.name()), &inv.0)
        .join("\n")
    };
  }

  // ── Hover info ──
  *hover_info = compute_hover_info(
    &windows, &camera_q, &current, player_q, &fov, &index, &named_q
  );

  // ── Overlay state ──
  overlay.kind = match ui.pause {
    crate::PauseMenu::Closed => None,
    crate::PauseMenu::Main => Some(OverlayKind::PauseMain),
    crate::PauseMenu::Controls => Some(OverlayKind::PauseControls),
  }.or_else(|| match &ui.interact {
    crate::InteractMenu::Open { options } => {
      Some(OverlayKind::Interact(mapv(|o| o.label.clone(), options)))
    }
    crate::InteractMenu::Closed => None,
  }).or_else(|| match &ui.dialogue {
    crate::DialogueState::Open { speaker, tree, node_name } => {
      let node = tree.find(node_name);
      let options: Vec<String> = mapv(|c| c.text.to_string(), node.choices);
      Some(OverlayKind::Dialogue { title: format!("What do you say? ({speaker})"), options })
    }
    crate::DialogueState::Closed => None,
  });

  // ── Log: oldest at top, newest at bottom; keep last 50 *messages* ──
  {
    let n = res_log.0.len();
    let start = n.saturating_sub(50);
    let lines: String = if n == 0 {
      String::new()
    } else {
      res_log.0[start..]
        .iter()
        .flat_map(|s| s.lines().map(String::from))
        .collect::<Vec<_>>()
        .join("\n")
    };
    log_display.text = if lines.is_empty() { "\u{2014}".into() } else { lines };
  }
}

fn compute_hover_info(
  windows: &Query<&Window>,
  camera_q: &Query<(&Camera, &GlobalTransform), With<Camera2d>>,
  current: &crate::CurrentZone,
  player_q: Query<(&crate::PlayerPos, &Stats, &crate::Inventory), With<crate::Player>>,
  fov: &crate::Fov,
  index: &crate::combat::TileEntityIndex,
  named_q: &Query<(&Named, Option<&Stats>)>,
) -> HoverInfo {
  let empty = HoverInfo {
    coords: (0, 0),
    tile_name: "—".into(),
    entity_name: None,
    entity_hp: None,
    flavor: None,
  };

  if let Ok(window) = windows.single()
    && let Ok((camera, cam_tf)) = camera_q.single()
    && let Ok((pos, _, _)) = player_q.single()
  {
    let level = current.0.level(pos.z);
    if let Some((tx, ty)) =
      try_pick_level_tile_at_cursor(window, camera, cam_tf, level.width, level.height)
    {
      let visible = fov.0.is_visible(tx as usize, ty as usize);
      let revealed = fov.0.is_revealed(tx as usize, ty as usize);
      if visible || revealed {
        let tile = level.tiles[ty as usize][tx as usize];
        let tile_name = if revealed && !visible {
          format!("{} (remembered)", tile.name())
        } else {
          tile.name().into()
        };

        // Look for entity on this tile
        let (entity_name, entity_hp, flavor) = if visible {
          index
            .0
            .get(&(tx, ty, pos.z))
            .and_then(|entities| {
              entities.iter().find_map(|&e| {
                named_q.get(e).ok().map(|(named, stats)| {
                  (
                    Some(named.name.into()),
                    stats.map(|s| (s.hp, s.max_hp)),
                    Some(named.flavor.into()),
                  )
                })
              })
            })
            .unwrap_or((None, None, None))
        } else {
          (None, None, None)
        };

        HoverInfo {
          coords: (tx, ty),
          tile_name,
          entity_name,
          entity_hp,
          flavor,
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

fn static_text(text: impl Into<String>, size: f32, color: Color, weight: FontWeight) -> impl Element {
  El::<Text>::new()
    .text(Text::new(text))
    .text_font(TextFont { font_size: size, weight, ..default() })
    .text_color(TextColor(color))
}

fn reactive_text(
  sig: impl Signal<Item = String> + Clone + 'static,
  size: f32,
  color: Color,
  weight: FontWeight,
) -> impl Element {
  El::<Text>::new()
    .text_font(TextFont { font_size: size, weight, ..default() })
    .text_color(TextColor(color))
    .with_builder(|b| b.component_signal::<Text>(
      sig.map_in(|s| Some(Text::new(s)))
    ))
}
