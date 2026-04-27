//! Haalka-based UI layer.  Owns the full window layout: game viewport on the
//! left, panel UI on the right/bottom, and centred overlays for menus and
//! dialogue.  A single [`sync_ui`] system bridges Bevy game-state into cloneable
//! resources every frame; Haalka signals react to those resources.

use {
  crate::{
    level::{ZONE_WIDTH, ZONE_HEIGHT},
    utils::mapv,
    Clock, screen_to_tile, world_to_zone, GAME_VIEWPORT_WIDTH_FRAC, STATUS_BAR_HEIGHT,
  },
  bevy::prelude::*,
  haalka::prelude::*,
  jonmo::{signal, prelude::*},
  trl::entities::{Stats, Named},
};

// ---------------------------------------------------------------------------
// Data shapes — written by sync_ui, read by Haalka signals
// ---------------------------------------------------------------------------

#[derive(Resource, Clone, Default)]
pub struct ClockData {
  pub mode: &'static str,
  pub tick: u64,
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

/// Accumulated messages; capped at 100 entries.
#[derive(Resource, Clone, Default)]
pub struct LogEntries(pub Vec<String>);

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

#[derive(Resource, Clone)]
pub struct DialogueData {
  pub speaker: String,
  pub text: String,
  pub choices: Vec<String>,
}

#[derive(Resource, Clone, Debug, PartialEq)]
pub enum OverlayKind {
  PauseMain,
  PauseControls,
  Interact(Vec<String>),
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
const LIGHT_TEXT:  Color = Color::srgb(0.88, 0.88, 0.88);
const DIM_TEXT:    Color = Color::srgb(0.55, 0.55, 0.60);
const ACCENT:      Color = Color::srgb(0.40, 0.65, 0.45);
const HP_GREEN:    Color = Color::srgb(0.35, 0.75, 0.35);
const HP_YELLOW:   Color = Color::srgb(0.85, 0.75, 0.25);
const HP_RED:      Color = Color::srgb(0.85, 0.30, 0.30);
const OVERLAY_DIM: Color = Color::srgba(0.0, 0.0, 0.0, 0.50);

const FONT_SIZE_LABEL: f32 = 13.0;
const FONT_SIZE_BODY:  f32 = 14.0;
const FONT_SIZE_TITLE: f32 = 16.0;
const FONT_SIZE_SMALL: f32 = 11.0;
const PANEL_PAD:      f32 = 8.0;

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
      .init_resource::<InvDisplayData>()
      .init_resource::<OverlayData>()
      .init_resource::<DialogueRes>()
      .init_resource::<WorldMapView>()
      .add_systems(PostUpdate, sync_ui);
  }
}

// ---------------------------------------------------------------------------
// Root layout
// ---------------------------------------------------------------------------

fn build_ui_root() -> impl Element {
  Stack::<Node>::new()
    .with_node(|mut n| { n.width = Val::Percent(100.0); n.height = Val::Percent(100.0); })
    .layer(main_layout())
    .layer_signal(dialogue_signal())
    .layer_signal(overlay_signal())
    .layer_signal(world_map_signal())
}

pub fn spawn_haalka_root(world: &mut World) {
  build_ui_root().spawn(world);
}

fn main_layout() -> impl Element {
  Column::<Node>::new()
    .with_node(|mut n| { n.width = Val::Percent(100.0); n.height = Val::Percent(100.0); })
    // ── top row: game viewport | sidebar ──
    .item(
      Row::<Node>::new()
        .with_node(|mut n| { n.width = Val::Percent(100.0); n.height = Val::Percent(100.0); n.flex_grow = 1.0; })
        // Game viewport (transparent — Camera2d renders behind)
        .item(
          El::<Node>::new()
            .with_node(|mut n| { n.width = Val::Percent(70.); n.height = Val::Percent(100.0); })
        )
        // Sidebar column
        .item(sidebar_column())
    )
    // ── bottom: status bar ──
    .item(status_bar())
}

fn sidebar_column() -> impl Element {
  Column::<Node>::new()
    .with_node(|mut n| {
      n.width = Val::Percent(30.);
      n.height = Val::Percent(100.);
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
    .item(static_text("HP:", FONT_SIZE_SMALL, LIGHT_TEXT))
    // Bar background
    .item(
      El::<Node>::new()
        .with_node(|mut n| {
          n.flex_grow = 1.;
          n.height = Val::Px(14.0);
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
      FONT_SIZE_SMALL, LIGHT_TEXT
    ))
}

fn stat_row(label: &str, value_sig: impl Signal<Item = String> + Clone + 'static) -> impl Element {
  Row::<Node>::new()
    .with_node(|mut n| {
      n.width = Val::Percent(100.);
      n.align_items = AlignItems::Center;
      n.justify_content = JustifyContent::SpaceBetween;
    })
    .item(static_text(label, FONT_SIZE_SMALL, DIM_TEXT))
    .item(reactive_text(value_sig, FONT_SIZE_SMALL, LIGHT_TEXT))
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
    FONT_SIZE_SMALL, DIM_TEXT
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
    FONT_SIZE_SMALL, ACCENT
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
    FONT_SIZE_BODY, Color::srgb(0.80, 0.75, 0.50)
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
    .item_signal(hover_entity_section())
}

fn hover_coords_line() -> impl Element {
  reactive_text(
    signal::from_resource::<HoverInfo>().map_in(|h| format!("({}, {})", h.coords.0, h.coords.1)),
    FONT_SIZE_SMALL, DIM_TEXT
  )
}

fn hover_tile_line() -> impl Element {
  reactive_text(
    signal::from_resource::<HoverInfo>().map_in(|h| h.tile_name.clone()),
    FONT_SIZE_BODY, LIGHT_TEXT
  )
}

fn hover_entity_section() -> impl Signal<Item = Option<impl Element>> {
  signal::from_resource::<HoverInfo>().map_in(move |h| {
    h.entity_name.as_ref().map(|name| {
      let name_row = static_text(name.as_str(), FONT_SIZE_BODY, Color::srgb(0.9, 0.75, 0.45));
      if let (Some((hp, max)), Some(flavor)) = (h.entity_hp, h.flavor.as_ref()) {
        Column::<Node>::new()
          .with_node(|mut n| {
            n.width = Val::Percent(100.);
            n.column_gap = Val::Px(2.0);
          })
          .item(name_row)
          .item(hp_bar_static(hp, max))
          .item(static_text(flavor.as_str(), FONT_SIZE_SMALL, DIM_TEXT))
      } else if let Some((hp, max)) = h.entity_hp {
        Column::<Node>::new()
          .with_node(|mut n| {
            n.width = Val::Percent(100.);
            n.column_gap = Val::Px(2.0);
          })
          .item(name_row)
          .item(hp_bar_static(hp, max))
      } else if let Some(ref flavor) = h.flavor {
        Column::<Node>::new()
          .with_node(|mut n| {
            n.width = Val::Percent(100.);
            n.column_gap = Val::Px(2.0);
          })
          .item(name_row)
          .item(static_text(flavor.as_str(), FONT_SIZE_SMALL, DIM_TEXT))
      } else {
        Column::<Node>::new()
          .with_node(|mut n| {
            n.width = Val::Percent(100.);
            n.column_gap = Val::Px(2.0);
          })
          .item(name_row)
      }
    })
  })
}

fn hp_bar_static(hp: i32, max_hp: i32) -> impl Element {
  let ratio = if max_hp > 0 { hp as f32 / max_hp as f32 } else { 0.0 };
  let filled = (ratio * 10.0).round() as usize;
  let bar = format!("[{}{}] {}/{}", "█".repeat(filled.min(10)), "░".repeat(10usize.saturating_sub(filled.min(10))), hp, max_hp);
  El::<Text>::new()
    .text(Text::new(bar))
    .text_font(TextFont { font_size: FONT_SIZE_SMALL, ..default() })
    .text_color(TextColor(DIM_TEXT))
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
    .background_color(BackgroundColor(PANEL_BG))
    .border_color(BorderColor::all(BORDER))
    .item(panel_label("Log"))
    .item_signal(log_entries_signal())
}

fn log_entries_signal() -> impl Signal<Item = Option<impl Element>> {
  signal::from_resource::<LogEntries>()
    .map_in(|entries| {
      if entries.0.is_empty() {
        return None;
      }
      let lines: Vec<String> = entries.0.iter().rev().take(50).cloned().collect();
      Some(
        Column::<Node>::new()
          .with_node(|mut n| { n.width = Val::Percent(100.); n.column_gap = Val::Px(1.0); })
          .items(lines.into_iter().map(|line| static_text(line, FONT_SIZE_SMALL, DIM_TEXT)))
      )
    })
}

// ---------------------------------------------------------------------------
// Status bar
// ---------------------------------------------------------------------------

fn status_bar() -> impl Element {
  El::<Node>::new()
    .with_node(|mut n| {
      n.width = Val::Percent(100.);
      n.height = Val::Px(24.0);
      n.border = UiRect::top(Val::Px(1.0));
      n.padding = UiRect::horizontal(Val::Px(8.0));
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
          FONT_SIZE_SMALL, DIM_TEXT
        ))
        .item(reactive_text(
          signal::from_resource::<HoverInfo>().map_in(|h| format!("({},{})", h.coords.0, h.coords.1)),
          FONT_SIZE_SMALL, DIM_TEXT
        ))
        .item(reactive_text(
          signal::from_resource::<ClockData>().map_in(|d| format!("{} T:{}", d.mode, d.tick)),
          FONT_SIZE_SMALL, DIM_TEXT
        ))
    )
}

// ---------------------------------------------------------------------------
// Dialogue — Skyrim-style subtitle bar at bottom of screen
// ---------------------------------------------------------------------------

fn dialogue_signal() -> impl Signal<Item = Option<impl Element>> {
  signal::from_resource::<DialogueRes>()
    .map_in(|res| {
      res.0.as_ref().cloned().map(|data| {
        El::<Node>::new()
          .with_node(|mut n| {
            n.width = Val::Percent(70.);
            n.position_type = PositionType::Absolute;
            n.bottom = Val::Px(28.0);
            n.left = Val::Percent(15.);
          })
          .background_color(BackgroundColor(DARK_BG))
          .border_color(BorderColor::all(BORDER))
          .child(
            Column::<Node>::new()
              .with_node(|mut n| {
                n.border_radius = BorderRadius::all(Val::Px(6.0));
                n.padding = UiRect { left: Val::Px(14.), right: Val::Px(14.), top: Val::Px(10.), bottom: Val::Px(10.) };
                n.column_gap = Val::Px(4.0);
              })
              .item(
                Row::<Node>::new()
                  .with_node(|mut n| { n.justify_content = JustifyContent::SpaceBetween; })
                  .item(static_text(&data.speaker, FONT_SIZE_TITLE, Color::srgb(0.80, 0.75, 0.50)))
                  .item(static_text("───", FONT_SIZE_BODY, BORDER))
              )
              .item(static_text(&data.text, FONT_SIZE_BODY, LIGHT_TEXT))
              .items({
                let choices = mapv(|c| format!("  {c}"), &data.choices);
                choices.into_iter().map(|c|
                  static_text(c, FONT_SIZE_BODY, Color::srgb(0.7, 0.65, 0.5))
                )
              })
          )
      })
    })
}

// World map — same column width as the game view (70%), shows full generated island
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
              n.width = Val::Percent(70.);
              n.max_height = Val::Percent(96.);
              n.padding = UiRect::all(Val::Px(14.));
              n.column_gap = Val::Px(8.0);
              n.align_items = AlignItems::Center;
            })
            .background_color(BackgroundColor(DARK_BG))
            .border_color(BorderColor::all(BORDER))
            .item(static_text("World map", FONT_SIZE_TITLE, ACCENT))
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
            .item(static_text("M  or  Esc  to close  ·  one pixel = one world tile", FONT_SIZE_SMALL, DIM_TEXT))
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
          OverlayKind::Interact(_) => "Interact",
        };
        let lines: Vec<String> = match kind {
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
              .item(static_text(label, FONT_SIZE_TITLE, LIGHT_TEXT))
              .items(lines.into_iter().map(|l| static_text(l, FONT_SIZE_BODY, LIGHT_TEXT)))
          )
      })
    })
  }

#[derive(Resource, Clone, Default)]
pub struct OverlayData {
  pub kind: Option<OverlayKind>,
}

// ---------------------------------------------------------------------------
// Dialogue overlay (Skyrim-style, anchored above status bar)
// ---------------------------------------------------------------------------

#[derive(Resource, Clone, Default)]
pub struct DialogueRes(pub Option<DialogueData>);

// ---------------------------------------------------------------------------
// sync_ui — reads Bevy world, writes signal resources
// ---------------------------------------------------------------------------

fn sync_ui(
  clock: Res<Clock>,
  player_q: Query<(&crate::PlayerPos, &Stats, &crate::Inventory), With<crate::Player>>,
  ui: Res<crate::UiState>,
  gw: Res<crate::GameWorld>,
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
  mut dialogue_res: ResMut<DialogueRes>,
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
    &windows, &camera_q, &gw, player_q, &fov, &index, &named_q
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
  });

  // ── Dialogue ──
  dialogue_res.0 = match &ui.dialogue {
    crate::DialogueState::Open { speaker, tree, node_name } => {
      let node = tree.find(node_name);
      let choices = mapv(
        |(i, c)| format!("{}) {}", i + 1, c.text),
        node.choices.iter().enumerate()
      );
      Some(DialogueData {
        speaker: (*speaker).into(),
        text: node.text.into(),
        choices,
      })
    }
    crate::DialogueState::Closed => None,
  };
}

fn compute_hover_info(
  windows: &Query<&Window>,
  camera_q: &Query<(&Camera, &GlobalTransform), With<Camera2d>>,
  gw: &crate::GameWorld,
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
    && let Some(cursor) = window.cursor_position()
    && cursor.x < window.resolution.width() * GAME_VIEWPORT_WIDTH_FRAC
    && cursor.y > STATUS_BAR_HEIGHT
    && let Ok(world_pos) = camera.viewport_to_world_2d(cam_tf, cursor)
  {
    let (tx, ty) = screen_to_tile(world_pos, ZONE_WIDTH, ZONE_HEIGHT);
    let (zx, zy) = world_to_zone(pos.x, pos.y);
    let level = gw.0.zone(zx, zy, pos.z);

    if tx < 0 || ty < 0 || tx as usize >= level.width || ty as usize >= level.height {
      return empty;
    }

    let visible = fov.0.is_visible(tx as usize, ty as usize);
    let revealed = fov.0.is_revealed(tx as usize, ty as usize);
    if !visible && !revealed { return empty; }

    let tile = level.tiles[ty as usize][tx as usize];
    let tile_name = if revealed && !visible {
      format!("{} (remembered)", tile.name())
    } else {
      tile.name().into()
    };

    // Look for entity on this tile
    let wx = (zx * ZONE_WIDTH) as i32 + tx;
    let wy = (zy * ZONE_HEIGHT) as i32 + ty;
    let (entity_name, entity_hp, flavor) = if visible {
      index
        .0
        .get(&(wx, wy, pos.z))
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
}

// ---------------------------------------------------------------------------
// Reusable UI helpers
// ---------------------------------------------------------------------------

fn panel_label(text: &str) -> impl Element {
  static_text(text, FONT_SIZE_LABEL, ACCENT)
}

fn static_text(text: impl Into<String>, size: f32, color: Color) -> impl Element {
  El::<Text>::new()
    .text(Text::new(text))
    .text_font(TextFont { font_size: size, ..default() })
    .text_color(TextColor(color))
}

fn reactive_text(sig: impl Signal<Item = String> + Clone + 'static, size: f32, color: Color) -> impl Element {
  El::<Text>::new()
    .text_font(TextFont { font_size: size, ..default() })
    .text_color(TextColor(color))
    .with_builder(|b| b.component_signal::<Text>(
      sig.map_in(|s| Some(Text::new(s)))
    ))
}
