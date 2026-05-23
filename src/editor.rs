#[path = "tiles.rs"]
pub mod tiles;
#[path = "sprites.rs"]
#[allow(dead_code)]
mod sprites;

pub const SPRITE_TEXELS: f32 = 20.0;

use bevy::{input::mouse::AccumulatedMouseScroll, prelude::*,
           sprite_render::{AlphaMode2d, TileData, TilemapChunk, TilemapChunkTileData}};
use grid_2d::Grid;
use std::{collections::HashMap, num::NonZeroU32};
use tiles::{Tile, TileRenderMode};
use wfc::{overlapping::OverlappingPatterns, retry::{NumTimes, RetryOwn}, RunOwn, Wave};

const CELL: f32 = 20.0;
const STEP: f32 = CELL;
const CANVAS_W: usize = 40;
const CANVAS_H: usize = 40;
const PALETTE_COLS: usize = 4;
const PAL_CELL: f32 = 24.0;

const OBJECT_TEMPLATES: &[&str] = &[
    "tree", "boulder", "door", "airlock_door", "bed", "table", "chair",
    "crafting_table", "locker", "crate", "loot_chest", "flight_console",
    "loadout_console", "space_cat", "thruster",
    "rat_soldier", "armored_rat_soldier", "robot", "wack_robot",
    "alien_runner", "lava_crab", "mantis_alien", "crab_alien",
    "mushroom_creature", "grenade_thrower", "gunman", "laser_sword",
];

fn to_color(c: [f32; 3]) -> Color { Color::srgb(c[0], c[1], c[2]) }

fn tile_color(t: Tile) -> Color { to_color(t.color()) }

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq)]
enum ToolMode { Draw, RectOutline, RectFill, Copy, Move, Paste }

impl ToolMode {
    fn name(self) -> &'static str {
        match self {
            ToolMode::Draw => "Draw",
            ToolMode::RectOutline => "Rect",
            ToolMode::RectFill => "Fill",
            ToolMode::Copy => "Copy",
            ToolMode::Move => "Move",
            ToolMode::Paste => "Paste",
        }
    }
}

#[derive(Clone)]
struct Clipboard {
    tiles: Vec<Vec<Tile>>,
    objects: Vec<Vec<Option<u8>>>,
}

// ---------------------------------------------------------------------------
// Resources
// ---------------------------------------------------------------------------

#[derive(Resource)]
struct EditorCanvas {
    tiles: Vec<Vec<Tile>>,
    objects: Vec<Vec<Option<u8>>>,
}

#[derive(Resource)]
struct EditorState {
    tool: ToolMode,
    selected_tile: Tile,
    selected_object: Option<u8>,
    drag_start: Option<(usize, usize)>,
    clipboard: Option<Clipboard>,
    pattern_size: u32,
    output_mult: u32,
}

#[derive(Resource)]
struct CameraZoom(f32);

#[derive(Resource)]
struct PanState {
    active: bool,
    cursor_origin: Vec2,
    camera_origin: Vec3,
}

#[derive(Resource)]
struct TileImageCache(Vec<(Handle<Image>, Color)>);

#[derive(Resource)]
struct EditorTileset(sprites::TilesetInfo);

#[derive(Resource)]
struct UndoStack(Vec<(Vec<Vec<Tile>>, Vec<Vec<Option<u8>>>)>);

// ---------------------------------------------------------------------------
// Components
// ---------------------------------------------------------------------------

#[derive(Component)]
struct CanvasCell(usize, usize);

#[derive(Component)]
struct ObjectLabel(usize, usize);

#[derive(Component)]
struct OutputChunk;

#[derive(Component)]
struct DragPreview;

#[derive(Component)]
struct TilePaletteBtn(Tile);

#[derive(Component)]
struct ObjectPaletteBtn(Option<u8>);

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


// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn canvas_origin() -> (f32, f32) {
    (-(CANVAS_W as f32 * STEP) / 2.0, (CANVAS_H as f32 * STEP) / 2.0)
}

fn world_to_grid(cursor: Vec2) -> Option<(usize, usize)> {
    let (ox, oy) = canvas_origin();
    let gx = ((cursor.x - ox + CELL / 2.0) / STEP) as i32;
    let gy = ((oy - cursor.y + CELL / 2.0) / STEP) as i32;
    (gx >= 0 && gx < CANVAS_W as i32 && gy >= 0 && gy < CANVAS_H as i32)
        .then_some((gx as usize, gy as usize))
}

fn grid_to_world(gx: usize, gy: usize) -> Vec2 {
    let (ox, oy) = canvas_origin();
    Vec2::new(ox + gx as f32 * STEP, oy - gy as f32 * STEP)
}

fn cursor_world(
    windows: &Query<&Window>,
    camera_q: &Query<(&Camera, &GlobalTransform)>,
) -> Option<Vec2> {
    let window = windows.single().ok()?;
    let (camera, cam_tf) = camera_q.single().ok()?;
    window.cursor_position().and_then(|p| camera.viewport_to_world_2d(cam_tf, p).ok())
}

fn selection_rect(a: (usize, usize), b: (usize, usize)) -> (usize, usize, usize, usize) {
    (a.0.min(b.0), a.1.min(b.1), a.0.max(b.0), a.1.max(b.1))
}

fn push_undo(canvas: &EditorCanvas, undo: &mut UndoStack) {
    undo.0.push((canvas.tiles.clone(), canvas.objects.clone()));
    if undo.0.len() > 50 { undo.0.remove(0); }
}

fn build_tile_cache(
    palette_cache: &mut sprites::PaletteImageCache,
    images: &mut Assets<Image>,
) -> TileImageCache {
    let mut entries = Vec::new();
    for tile in Tile::all() {
        let extract = |rm: TileRenderMode| -> Option<(&'static str, [f32;3], [f32;3])> {
            match rm {
                TileRenderMode::SolidColor => None,
                TileRenderMode::Sprite(p, a, b) => Some((p, a, b)),
                TileRenderMode::SpritePackRandom(ps, a, b) => Some((ps[0], a, b)),
                TileRenderMode::ConnectedSprite(ps, a, b) => Some((ps[0], a, b)),
                TileRenderMode::ConnectedBorder(p, a, b) => Some((p, a, b)),
            }
        };
        let entry = extract(tile.render_mode())
            .map(|(path, pri, sec)| {
                let h = sprites::palette_sprite_handle(
                    path, to_color(pri), to_color(sec), palette_cache, images,
                );
                (h, Color::WHITE)
            })
            .unwrap_or_else(|| (Handle::default(), tile_color(tile)));
        entries.push(entry);
    }
    TileImageCache(entries)
}

// ---------------------------------------------------------------------------
// Setup
// ---------------------------------------------------------------------------

fn setup(
    mut commands: Commands,
    mut palette_cache: ResMut<sprites::PaletteImageCache>,
    mut images: ResMut<Assets<Image>>,
) {
    commands.spawn(Camera2d);

    let tile_cache = build_tile_cache(&mut palette_cache, &mut images);

    // --- UI sidebar ---
    commands.spawn(Node {
        width: Val::Px(PAL_CELL * PALETTE_COLS as f32 + 16.0),
        height: Val::Percent(100.0),
        flex_direction: FlexDirection::Column,
        padding: UiRect::all(Val::Px(4.0)),
        overflow: Overflow::scroll_y(),
        ..default()
    }).with_child((
        Text::new("Tiles"),
        TextFont { font_size: 12.0, ..default() },
        TextColor(Color::srgb(0.9, 0.9, 0.5)),
        Node { margin: UiRect::bottom(Val::Px(4.0)), ..default() },
    )).with_children(|sidebar| {
        let all_tiles: Vec<Tile> = Tile::all().collect();

        // Tile palette grid
        let mut tile_grid = sidebar.spawn(Node {
            flex_wrap: FlexWrap::Wrap,
            ..default()
        });
        tile_grid.with_children(|grid| {
            for &tile in &all_tiles {
                let (ref img_h, color) = tile_cache.0[tile as u16 as usize];
                let has_texture = *img_h != Handle::default();
                let mut btn = grid.spawn((
                    Button,
                    Node {
                        width: Val::Px(PAL_CELL),
                        height: Val::Px(PAL_CELL),
                        border: UiRect::all(Val::Px(1.0)),
                        ..default()
                    },
                    BorderColor::all(Color::srgba(0.3, 0.3, 0.3, 1.0)),
                    if has_texture { BackgroundColor(Color::BLACK) }
                    else { BackgroundColor(color) },
                    TilePaletteBtn(tile),
                ));
                if has_texture {
                    btn.with_child((
                        ImageNode::new(img_h.clone()),
                        Node {
                            width: Val::Percent(100.0),
                            height: Val::Percent(100.0),
                            ..default()
                        },
                    ));
                }
            }
        });

        // Separator
        sidebar.spawn((
            Text::new("Objects"),
            TextFont { font_size: 12.0, ..default() },
            TextColor(Color::srgb(0.5, 0.9, 0.5)),
            Node { margin: UiRect::vertical(Val::Px(6.0)), ..default() },
        ));

        // Object palette
        let mut obj_grid = sidebar.spawn(Node {
            flex_direction: FlexDirection::Column,
            ..default()
        });
        obj_grid.with_children(|col| {
            let entries: Vec<Option<u8>> = std::iter::once(None)
                .chain((0..OBJECT_TEMPLATES.len()).map(|i| Some(i as u8)))
                .collect();
            for &obj in &entries {
                let label = obj.map(|i| OBJECT_TEMPLATES[i as usize]).unwrap_or("none");
                col.spawn((
                    Button,
                    Node {
                        padding: UiRect::axes(Val::Px(4.0), Val::Px(1.0)),
                        border: UiRect::all(Val::Px(1.0)),
                        ..default()
                    },
                    BorderColor::all(Color::srgba(0.3, 0.3, 0.3, 1.0)),
                    BackgroundColor(Color::srgba(0.15, 0.15, 0.2, 1.0)),
                    ObjectPaletteBtn(obj),
                )).with_child((
                    Text::new(label),
                    TextFont { font_size: 11.0, ..default() },
                    TextColor(Color::srgb(0.8, 0.8, 0.8)),
                ));
            }
        });

    });

    // --- Tile hover preview popup (floating, outside sidebar) ---
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(PAL_CELL * PALETTE_COLS as f32 + 24.0),
            top: Val::Px(8.0),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            padding: UiRect::all(Val::Px(6.0)),
            display: Display::None,
            ..default()
        },
        BackgroundColor(Color::srgba(0.1, 0.1, 0.12, 0.95)),
        TilePreviewPopup,
    )).with_children(|popup| {
        popup.spawn((
            ImageNode::new(Handle::default()),
            Node {
                width: Val::Px(128.0),
                height: Val::Px(128.0),
                ..default()
            },
            BackgroundColor(Color::BLACK),
            TilePreviewImage,
        ));
        popup.spawn((
            Text::new(""),
            TextFont { font_size: 16.0, ..default() },
            TextColor(Color::srgb(0.9, 0.9, 0.9)),
            Node { margin: UiRect::top(Val::Px(6.0)), ..default() },
            TilePreviewText,
        ));
    });

    // --- Bottom mode bar ---
    commands.spawn(Node {
        position_type: PositionType::Absolute,
        bottom: Val::Px(0.0),
        left: Val::Px(0.0),
        width: Val::Percent(100.0),
        justify_content: JustifyContent::Center,
        align_items: AlignItems::Center,
        padding: UiRect::all(Val::Px(6.0)),
        column_gap: Val::Px(4.0),
        ..default()
    }).with_children(|bar| {
        let modes = [
            (ToolMode::Draw, "[D]raw"),
            (ToolMode::RectOutline, "[R]ect"),
            (ToolMode::RectFill, "[F]ill"),
            (ToolMode::Copy, "[C]opy"),
            (ToolMode::Move, "[M]ove"),
        ];
        for (mode, label) in modes {
            bar.spawn((
                Node {
                    padding: UiRect::axes(Val::Px(10.0), Val::Px(4.0)),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.15, 0.15, 0.2, 0.9)),
                ModeBarBtn(mode),
            )).with_child((
                Text::new(label),
                TextFont { font_size: 14.0, ..default() },
                TextColor(Color::srgb(0.5, 0.5, 0.5)),
                ModeBarLabel,
            ));
        }

        bar.spawn(Node { width: Val::Px(20.0), ..default() });

        bar.spawn((
            Text::new(""),
            TextFont { font_size: 13.0, ..default() },
            TextColor(Color::srgb(0.6, 0.6, 0.5)),
            ControlsLabel,
        ));
    });

    // --- Canvas cells (world-space) ---
    for y in 0..CANVAS_H {
        for x in 0..CANVAS_W {
            let w = grid_to_world(x, y);
            let (ref img, color) = tile_cache.0[Tile::Grass as u16 as usize];
            commands.spawn((
                Sprite {
                    image: img.clone(),
                    color,
                    custom_size: Some(Vec2::splat(CELL)),
                    ..default()
                },
                Transform::from_xyz(w.x, w.y, 0.0),
                CanvasCell(x, y),
            )).with_children(|parent| {
                parent.spawn((
                    Text2d::new(""),
                    TextFont { font_size: 10.0, ..default() },
                    TextColor(Color::srgb(1.0, 1.0, 1.0)),
                    Transform::from_xyz(0.0, 0.0, 0.5),
                    ObjectLabel(x, y),
                ));
            });
        }
    }

    let tileset_info = sprites::build_tileset(&mut images);
    commands.insert_resource(EditorTileset(tileset_info));
    commands.insert_resource(tile_cache);
}

// ---------------------------------------------------------------------------
// Camera: RMB pan
// ---------------------------------------------------------------------------

fn camera_pan(
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    mut camera_q: Query<(&Camera, &GlobalTransform, &mut Transform), With<Camera2d>>,
    mut pan: ResMut<PanState>,
) {
    if let Some(cursor_pos) = windows.single().ok().and_then(|w| w.cursor_position()) {
        if mouse.just_pressed(MouseButton::Right) {
            if let Ok((_, _, tf)) = camera_q.single() {
                pan.active = true;
                pan.cursor_origin = cursor_pos;
                pan.camera_origin = tf.translation;
            }
        }
        if mouse.just_released(MouseButton::Right) {
            pan.active = false;
        }
        if pan.active {
            if let Ok((_, _, mut tf)) = camera_q.single_mut() {
                let delta = cursor_pos - pan.cursor_origin;
                tf.translation.x = pan.camera_origin.x - delta.x * tf.scale.x;
                tf.translation.y = pan.camera_origin.y + delta.y * tf.scale.y;
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Camera: scroll wheel zoom
// ---------------------------------------------------------------------------

fn camera_zoom(
    scroll: Res<AccumulatedMouseScroll>,
    windows: Query<&Window>,
    mut camera_q: Query<(&Camera, &GlobalTransform, &mut Transform), With<Camera2d>>,
    mut zoom: ResMut<CameraZoom>,
) {
    if scroll.delta.y != 0.0
        && let Ok((cam, cam_gt, mut tf)) = camera_q.single_mut()
    {
        let cursor_world = windows.single().ok()
            .and_then(|w| w.cursor_position())
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
    mut state: ResMut<EditorState>,
) {
    for (interaction, btn) in &interaction_q {
        if *interaction == Interaction::Pressed {
            state.selected_tile = btn.0;
        }
    }
}

fn ui_tile_highlight(
    state: Res<EditorState>,
    mut btn_q: Query<(&TilePaletteBtn, &mut BorderColor)>,
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
    mut state: ResMut<EditorState>,
) {
    for (interaction, btn) in &interaction_q {
        if *interaction == Interaction::Pressed {
            state.selected_object = btn.0;
        }
    }
}

fn ui_object_highlight(
    state: Res<EditorState>,
    mut btn_q: Query<(&ObjectPaletteBtn, &mut BorderColor)>,
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
    hover_q: Query<(&Interaction, &TilePaletteBtn), Changed<Interaction>>,
    tile_cache: Res<TileImageCache>,
    mut img_q: Query<(&mut ImageNode, &mut BackgroundColor), With<TilePreviewImage>>,
    mut text_q: Query<&mut Text, With<TilePreviewText>>,
    mut popup_q: Query<&mut Node, With<TilePreviewPopup>>,
) {
    let mut any_hovered = false;
    for (interaction, btn) in &hover_q {
        if *interaction == Interaction::Hovered || *interaction == Interaction::Pressed {
            any_hovered = true;
            let (ref img_h, color) = tile_cache.0[btn.0 as u16 as usize];
            let has_texture = *img_h != Handle::default();
            if let Ok((mut img_node, mut bg)) = img_q.single_mut() {
                if has_texture {
                    img_node.image = img_h.clone();
                    bg.0 = Color::BLACK;
                } else {
                    img_node.image = Handle::default();
                    bg.0 = color;
                }
            }
            if let Ok(mut text) = text_q.single_mut() {
                text.0 = format!("{:?}", btn.0);
            }
        }
    }
    if !hover_q.is_empty() {
        if let Ok(mut node) = popup_q.single_mut() {
            node.display = if any_hovered { Display::Flex } else { Display::None };
        }
    }
}

fn update_mode_bar(
    state: Res<EditorState>,
    btn_q: Query<(&ModeBarBtn, &Children)>,
    mut label_q: Query<&mut TextColor, With<ModeBarLabel>>,
    mut status_q: Query<&mut Text, With<ControlsLabel>>,
) {
    if state.is_changed() {
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
        let obj_name = state.selected_object
            .map(|i| OBJECT_TEMPLATES[i as usize])
            .unwrap_or("none");
        if let Ok(mut text) = status_q.single_mut() {
            text.0 = format!(
                "tile:{:?}  obj:{}  pat:{}  |  U:undo G:gen [/]:pat Ctrl+S/O:save/load",
                state.selected_tile,
                obj_name,
                state.pattern_size,
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Tool switching & object cycling
// ---------------------------------------------------------------------------

fn tool_keys(
    keys: Res<ButtonInput<KeyCode>>,
    mut state: ResMut<EditorState>,
) {
    if keys.just_pressed(KeyCode::KeyD) { state.tool = ToolMode::Draw; }
    if keys.just_pressed(KeyCode::KeyR) { state.tool = ToolMode::RectOutline; }
    if keys.just_pressed(KeyCode::KeyF) { state.tool = ToolMode::RectFill; }
    if keys.just_pressed(KeyCode::KeyC) && !keys.pressed(KeyCode::ControlLeft) && !keys.pressed(KeyCode::ControlRight) {
        state.tool = ToolMode::Copy;
        state.drag_start = None;
    }
    if keys.just_pressed(KeyCode::KeyM) {
        state.tool = ToolMode::Move;
        state.drag_start = None;
    }
    if keys.just_pressed(KeyCode::Escape) {
        state.tool = ToolMode::Draw;
        state.drag_start = None;
    }

    if keys.just_pressed(KeyCode::BracketLeft) {
        state.pattern_size = (state.pattern_size - 1).max(2);
    }
    if keys.just_pressed(KeyCode::BracketRight) {
        state.pattern_size = (state.pattern_size + 1).min(5);
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
    mut undo: ResMut<UndoStack>,
    pan: Res<PanState>,
) {
    if pan.active { return; }
    let Some(cursor) = cursor_world(&windows, &camera_q) else { return };
    let grid_pos = world_to_grid(cursor);

    match state.tool {
        ToolMode::Draw => {
            if mouse.pressed(MouseButton::Left) {
                if let Some((gx, gy)) = grid_pos {
                    if mouse.just_pressed(MouseButton::Left) {
                        push_undo(&canvas, &mut undo);
                    }
                    canvas.tiles[gy][gx] = state.selected_tile;
                    canvas.objects[gy][gx] = state.selected_object;
                }
            }
        }
        ToolMode::RectOutline | ToolMode::RectFill => {
            if mouse.just_pressed(MouseButton::Left) {
                if let Some(pos) = grid_pos {
                    state.drag_start = Some(pos);
                }
            }
            if mouse.just_released(MouseButton::Left) {
                if let (Some(start), Some(end)) = (state.drag_start, grid_pos) {
                    push_undo(&canvas, &mut undo);
                    let (x1, y1, x2, y2) = selection_rect(start, end);
                    let filled = state.tool == ToolMode::RectFill;
                    for y in y1..=y2 {
                        for x in x1..=x2 {
                            if filled || x == x1 || x == x2 || y == y1 || y == y2 {
                                canvas.tiles[y][x] = state.selected_tile;
                                canvas.objects[y][x] = state.selected_object;
                            }
                        }
                    }
                    state.drag_start = None;
                }
            }
        }
        ToolMode::Copy | ToolMode::Move => {
            if mouse.just_pressed(MouseButton::Left) {
                if let Some(pos) = grid_pos {
                    state.drag_start = Some(pos);
                }
            }
            if mouse.just_released(MouseButton::Left) {
                if let (Some(start), Some(end)) = (state.drag_start, grid_pos) {
                    let (x1, y1, x2, y2) = selection_rect(start, end);
                    let mut clip_tiles = Vec::new();
                    let mut clip_objects = Vec::new();
                    for y in y1..=y2 {
                        let mut row_t = Vec::new();
                        let mut row_o = Vec::new();
                        for x in x1..=x2 {
                            row_t.push(canvas.tiles[y][x]);
                            row_o.push(canvas.objects[y][x]);
                        }
                        clip_tiles.push(row_t);
                        clip_objects.push(row_o);
                    }
                    state.clipboard = Some(Clipboard { tiles: clip_tiles, objects: clip_objects });
                    if state.tool == ToolMode::Move {
                        push_undo(&canvas, &mut undo);
                        for y in y1..=y2 {
                            for x in x1..=x2 {
                                canvas.tiles[y][x] = Tile::Grass;
                                canvas.objects[y][x] = None;
                            }
                        }
                    }
                    state.tool = ToolMode::Paste;
                    state.drag_start = None;
                }
            }
        }
        ToolMode::Paste => {
            if mouse.just_pressed(MouseButton::Left) {
                if let (Some((gx, gy)), Some(clip)) = (grid_pos, state.clipboard.clone()) {
                    push_undo(&canvas, &mut undo);
                    for (dy, row) in clip.tiles.iter().enumerate() {
                        for (dx, &tile) in row.iter().enumerate() {
                            let tx = gx + dx;
                            let ty = gy + dy;
                            if tx < CANVAS_W && ty < CANVAS_H {
                                canvas.tiles[ty][tx] = tile;
                                canvas.objects[ty][tx] = clip.objects[dy][dx];
                            }
                        }
                    }
                    state.tool = ToolMode::Draw;
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
    mut state: ResMut<EditorState>,
) {
    let pick = (keys.pressed(KeyCode::AltLeft) && mouse.just_pressed(MouseButton::Left))
        || mouse.just_pressed(MouseButton::Middle);
    if pick {
        if let Some((gx, gy)) = cursor_world(&windows, &camera_q).and_then(world_to_grid) {
            state.selected_tile = canvas.tiles[gy][gx];
            state.selected_object = canvas.objects[gy][gx];
        }
    }
}

// ---------------------------------------------------------------------------
// Copy / Cut / Paste / Undo
// ---------------------------------------------------------------------------

fn undo_key(
    keys: Res<ButtonInput<KeyCode>>,
    mut canvas: ResMut<EditorCanvas>,
    mut undo: ResMut<UndoStack>,
) {
    let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    if keys.just_pressed(KeyCode::KeyU) || (ctrl && keys.just_pressed(KeyCode::KeyZ)) {
        if let Some((tiles, objects)) = undo.0.pop() {
            canvas.tiles = tiles;
            canvas.objects = objects;
        }
    }
}

// ---------------------------------------------------------------------------
// Sync canvas tile sprites
// ---------------------------------------------------------------------------

fn sync_canvas_sprites(
    canvas: Res<EditorCanvas>,
    tile_cache: Res<TileImageCache>,
    mut query: Query<(&CanvasCell, &mut Sprite)>,
) {
    if canvas.is_changed() {
        for (cell, mut sprite) in &mut query {
            let tile = canvas.tiles[cell.1][cell.0];
            let (ref img, color) = tile_cache.0[tile as u16 as usize];
            sprite.image = img.clone();
            sprite.color = color;
        }
    }
}

// ---------------------------------------------------------------------------
// Sync object labels
// ---------------------------------------------------------------------------

fn sync_object_labels(
    canvas: Res<EditorCanvas>,
    mut query: Query<(&ObjectLabel, &mut Text2d)>,
) {
    if canvas.is_changed() {
        for (label, mut text) in &mut query {
            text.0 = canvas.objects[label.1][label.0]
                .map(|idx| {
                    let name = OBJECT_TEMPLATES[idx as usize];
                    name.chars().take(3).collect()
                })
                .unwrap_or_default();
        }
    }
}

// ---------------------------------------------------------------------------
// Selection & drag preview overlays
// ---------------------------------------------------------------------------

fn update_overlays(
    mut commands: Commands,
    state: Res<EditorState>,
    windows: Query<&Window>,
    camera_q: Query<(&Camera, &GlobalTransform)>,
    existing_drag: Query<Entity, With<DragPreview>>,
) {
    for e in &existing_drag { commands.entity(e).despawn(); }

    if state.drag_start.is_some() {
        if let Some(cursor) = cursor_world(&windows, &camera_q) {
            if let (Some(start), Some(end)) = (state.drag_start, world_to_grid(cursor)) {
                let (x1, y1, x2, y2) = selection_rect(start, end);
                let tl = grid_to_world(x1, y1);
                let br = grid_to_world(x2, y2);
                let center = (tl + br) / 2.0;
                let size = Vec2::new(
                    (x2 - x1 + 1) as f32 * STEP,
                    (y2 - y1 + 1) as f32 * STEP,
                );
                let color = match state.tool {
                    ToolMode::RectOutline | ToolMode::RectFill =>
                        Color::srgba(1.0, 1.0, 0.0, 0.3),
                    _ => Color::srgba(0.2, 0.6, 1.0, 0.3),
                };
                commands.spawn((
                    Sprite {
                        color,
                        custom_size: Some(size),
                        ..default()
                    },
                    Transform::from_xyz(center.x, center.y, 2.0),
                    DragPreview,
                ));
            }
        }
    }
}

// ---------------------------------------------------------------------------
// WFC generation
// ---------------------------------------------------------------------------

fn encode_cell(tile: Tile, obj: Option<u8>) -> u16 {
    (tile as u16) | ((obj.map(|o| o as u16 + 1).unwrap_or(0)) << 8)
}

fn decode_cell(val: u16) -> (Option<Tile>, Option<u8>) {
    let tile = Tile::try_from(val & 0xFF).ok();
    let obj = match val >> 8 { 0 => None, n => Some((n - 1) as u8) };
    (tile, obj)
}

#[derive(Component)]
struct OutputLabel;

fn generate_wfc(
    keys: Res<ButtonInput<KeyCode>>,
    canvas: Res<EditorCanvas>,
    state: Res<EditorState>,
    tileset: Res<EditorTileset>,
    mut commands: Commands,
    existing: Query<Entity, Or<(With<OutputChunk>, With<OutputLabel>)>>,
) {
    if !keys.just_pressed(KeyCode::KeyG) { return; }

    for e in &existing { commands.entity(e).despawn(); }

    let ow = CANVAS_W as u32 * state.output_mult;
    let oh = CANVAS_H as u32 * state.output_mult;

    let input_grid = Grid::new_fn(
        coord_2d::Size::new(CANVAS_W as u32, CANVAS_H as u32),
        |coord| encode_cell(
            canvas.tiles[coord.y as usize][coord.x as usize],
            canvas.objects[coord.y as usize][coord.x as usize],
        ),
    );

    let pattern_size = NonZeroU32::new(state.pattern_size).unwrap();
    let patterns = OverlappingPatterns::new(
        input_grid,
        pattern_size,
        &[
            wfc::orientation::Orientation::Original,
            wfc::orientation::Orientation::Clockwise90,
            wfc::orientation::Orientation::Clockwise180,
            wfc::orientation::Orientation::Clockwise270,
            wfc::orientation::Orientation::DiagonallyFlipped,
            wfc::orientation::Orientation::DiagonallyFlippedClockwise90,
            wfc::orientation::Orientation::DiagonallyFlippedClockwise180,
            wfc::orientation::Orientation::DiagonallyFlippedClockwise270,
        ],
    );

    let global_stats = patterns.global_stats();
    let mut rng = rand::thread_rng();
    let output_size = coord_2d::Size::new(ow, oh);

    let run = RunOwn::new(output_size, &global_stats, &mut rng);
    let result: Result<Wave, _> = NumTimes(20).retry(run, &mut rng);

    match result {
        Ok(wave) => {
            let canvas_right = (CANVAS_W as f32 * STEP) / 2.0;
            let origin_x = canvas_right + 40.0;

            let mut tile_data: Vec<Option<TileData>> = vec![None; (ow * oh) as usize];
            for coord_y in 0..oh {
                for coord_x in 0..ow {
                    let cell = wave.grid().get(coord_2d::Coord::new(coord_x as i32, coord_y as i32)).unwrap();
                    let val = cell.chosen_pattern_id().ok()
                        .map(|id| *patterns.pattern_top_left_value(id));
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
                            sprites::TileSelect::Connected => info.base,
                        };
                        let chunk_idx = ((oh - 1 - coord_y) * ow + coord_x) as usize;
                        tile_data[chunk_idx] = Some(TileData {
                            tileset_index,
                            color: Color::WHITE,
                            visible: true,
                        });
                    }
                    if let Some(idx) = obj {
                        if (idx as usize) < OBJECT_TEMPLATES.len() {
                            let name: String = OBJECT_TEMPLATES[idx as usize].chars().take(3).collect();
                            let x = origin_x + coord_x as f32 * CELL;
                            let y = (oh - 1 - coord_y) as f32 * CELL;
                            commands.spawn((
                                Text2d::new(name),
                                TextFont { font_size: 8.0, ..default() },
                                TextColor(Color::srgb(1.0, 0.8, 0.2)),
                                Transform::from_xyz(x, y, 1.0),
                                OutputLabel,
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
                    alpha_mode: AlphaMode2d::Blend,
                },
                TilemapChunkTileData(tile_data),
                Transform::from_xyz(origin_x - CELL / 2.0, CELL / 2.0, 0.0),
                OutputChunk,
            ));
            eprintln!("WFC generated {ow}x{oh} output (pattern_size={})", state.pattern_size);
        }
        Err(_) => {
            eprintln!("WFC generation failed after retries");
        }
    }
}

// ---------------------------------------------------------------------------
// Save / Load
// ---------------------------------------------------------------------------

const SAVE_PATH: &str = "editor_save.txt";

fn save_load(
    keys: Res<ButtonInput<KeyCode>>,
    mut canvas: ResMut<EditorCanvas>,
    mut undo: ResMut<UndoStack>,
) {
    let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    if ctrl && keys.just_pressed(KeyCode::KeyS) {
        let mut out = format!("{} {}\n", CANVAS_W, CANVAS_H);
        for y in 0..CANVAS_H {
            for x in 0..CANVAS_W {
                let t = canvas.tiles[y][x] as u16;
                let o = canvas.objects[y][x].map(|v| v as i16).unwrap_or(-1);
                out.push_str(&format!("{t} {o} "));
            }
            out.push('\n');
        }
        std::fs::write(SAVE_PATH, &out).unwrap();
        eprintln!("Saved to {SAVE_PATH}");
    }
    if ctrl && keys.just_pressed(KeyCode::KeyO) {
        if let Ok(text) = std::fs::read_to_string(SAVE_PATH) {
            let mut nums = text.split_whitespace();
            let w: usize = nums.next().and_then(|s| s.parse().ok()).unwrap_or(CANVAS_W);
            let h: usize = nums.next().and_then(|s| s.parse().ok()).unwrap_or(CANVAS_H);
            push_undo(&canvas, &mut undo);
            for y in 0..h.min(CANVAS_H) {
                for x in 0..w.min(CANVAS_W) {
                    let t: u16 = nums.next().and_then(|s| s.parse().ok()).unwrap_or(0);
                    let o: i16 = nums.next().and_then(|s| s.parse().ok()).unwrap_or(-1);
                    canvas.tiles[y][x] = Tile::try_from(t).unwrap_or(Tile::Grass);
                    canvas.objects[y][x] = (o >= 0).then_some(o as u8);
                }
            }
            eprintln!("Loaded from {SAVE_PATH}");
        }
    }
}

// ---------------------------------------------------------------------------
// Export prefab
// ---------------------------------------------------------------------------

fn export_prefab(keys: Res<ButtonInput<KeyCode>>, canvas: Res<EditorCanvas>) {
    let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    if keys.just_pressed(KeyCode::KeyE) && !ctrl {
        let mut chars_used = HashMap::<u16, char>::new();
        let mut next_char = b'a';

        for row in &canvas.tiles {
            for &tile in row {
                chars_used.entry(tile as u16).or_insert_with(|| {
                    let c = next_char as char;
                    next_char += 1;
                    if next_char == b'{' { next_char = b'A'; }
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

        let out = format!("{}\n{}", layout, assocs);
        let path = "editor_export.txt";
        std::fs::write(path, &out).unwrap();
        eprintln!("Exported to {path}");
    }
}

// ---------------------------------------------------------------------------
// Window title (shows mode/tool/tile/object info)
// ---------------------------------------------------------------------------

fn update_title(state: Res<EditorState>, mut windows: Query<&mut Window>) {
    if state.is_changed() {
        if let Ok(mut win) = windows.single_mut() {
            win.title = format!("Level Editor | {}", state.tool.name());
        }
    }
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

fn main() {
    App::new()
        .add_plugins(DefaultPlugins
            .set(ImagePlugin::default_nearest())
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: "Level Editor".into(),
                    resolution: (1600, 900).into(),
                    ..default()
                }),
                ..default()
            }))
        .insert_resource(ClearColor(Color::srgb(0.08, 0.08, 0.1)))
        .insert_resource(EditorCanvas {
            tiles: vec![vec![Tile::Grass; CANVAS_W]; CANVAS_H],
            objects: vec![vec![None; CANVAS_W]; CANVAS_H],
        })
        .insert_resource(EditorState {
            tool: ToolMode::Draw,
            selected_tile: Tile::Wall,
            selected_object: None,
            drag_start: None,
            clipboard: None,
            pattern_size: 3,
            output_mult: 3,
        })
        .insert_resource(CameraZoom(1.0))
        .insert_resource(PanState {
            active: false,
            cursor_origin: Vec2::ZERO,
            camera_origin: Vec3::ZERO,
        })
        .insert_resource(UndoStack(Vec::new()))
        .init_resource::<sprites::PaletteImageCache>()
        .add_systems(Startup, setup)
        .add_systems(Update, (
            camera_pan,
            camera_zoom,
            tool_keys,
            ui_tile_palette,
        ))
        .add_systems(Update, (
            ui_tile_highlight,
            ui_object_palette,
            ui_object_highlight,
        ))
        .add_systems(Update, (
            update_tile_preview,
            update_mode_bar,
        ))
        .add_systems(Update, (
            eyedropper,
            canvas_interact,
            undo_key,
            sync_canvas_sprites,
        ))
        .add_systems(Update, (
            sync_object_labels,
            update_overlays,
            generate_wfc,
            save_load,
        ))
        .add_systems(Update, (
            export_prefab,
            update_title,
        ))
        .run();
}
