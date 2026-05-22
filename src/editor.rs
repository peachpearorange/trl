#[path = "tiles.rs"]
mod tiles;

use bevy::prelude::*;
use grid_2d::Grid;
use std::{collections::HashMap, num::NonZeroU32};
use tiles::Tile;
use wfc::{
    overlapping::OverlappingPatterns,
    retry::{NumTimes, RetryOwn},
    RunOwn, Wave,
};

const CELL: f32 = 16.0;
const GAP: f32 = 1.0;
const STEP: f32 = CELL + GAP;
const CANVAS_W: usize = 40;
const CANVAS_H: usize = 40;
const OUTPUT_MULT: u32 = 3;
const PATTERN_SIZE: u32 = 3;

const PALETTE_COLS: usize = 2;

fn tile_color(t: Tile) -> Color {
    let [r, g, b] = t.color();
    Color::srgb(r, g, b)
}

// ---------------------------------------------------------------------------
// Resources
// ---------------------------------------------------------------------------

#[derive(Resource)]
struct EditorCanvas {
    tiles: Vec<Vec<Tile>>,
    default_tile: Tile,
}

#[derive(Resource)]
struct SelectedTile(Tile);

#[derive(Resource)]
struct OutputGrid(Option<Vec<Vec<Tile>>>);

#[derive(Resource)]
struct CameraZoom(f32);

// ---------------------------------------------------------------------------
// Marker components
// ---------------------------------------------------------------------------

#[derive(Component)]
struct CanvasCell(usize, usize);

#[derive(Component)]
struct OutputCell;

#[derive(Component)]
struct PaletteEntry(Tile);

#[derive(Component)]
struct PaletteHighlight;

// ---------------------------------------------------------------------------
// Setup
// ---------------------------------------------------------------------------

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);

    let all_tiles: Vec<Tile> = Tile::all().collect();
    let palette_x = -(CANVAS_W as f32 * STEP / 2.0) - STEP * (PALETTE_COLS as f32) - 40.0;
    let palette_top = (CANVAS_H as f32 * STEP) / 2.0;

    for (i, &tile) in all_tiles.iter().enumerate() {
        let col = i % PALETTE_COLS;
        let row = i / PALETTE_COLS;
        let x = palette_x + col as f32 * STEP;
        let y = palette_top - row as f32 * STEP;
        commands.spawn((
            Sprite {
                color: tile_color(tile),
                custom_size: Some(Vec2::splat(CELL)),
                ..default()
            },
            Transform::from_xyz(x, y, 0.0),
            PaletteEntry(tile),
        ));
    }

    // highlight marker behind selected palette entry
    commands.spawn((
        Sprite {
            color: Color::srgb(1.0, 1.0, 0.0),
            custom_size: Some(Vec2::splat(CELL + 4.0)),
            ..default()
        },
        Transform::from_xyz(palette_x, palette_top, -0.1),
        PaletteHighlight,
    ));

    let canvas_origin_x = -(CANVAS_W as f32 * STEP) / 2.0;
    let canvas_origin_y = (CANVAS_H as f32 * STEP) / 2.0;

    for y in 0..CANVAS_H {
        for x in 0..CANVAS_W {
            let wx = canvas_origin_x + x as f32 * STEP;
            let wy = canvas_origin_y - y as f32 * STEP;
            commands.spawn((
                Sprite {
                    color: tile_color(Tile::Grass),
                    custom_size: Some(Vec2::splat(CELL)),
                    ..default()
                },
                Transform::from_xyz(wx, wy, 0.0),
                CanvasCell(x, y),
            ));
        }
    }
}

// ---------------------------------------------------------------------------
// Input: palette selection
// ---------------------------------------------------------------------------

fn palette_click(
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    camera_q: Query<(&Camera, &GlobalTransform)>,
    palette_q: Query<(&PaletteEntry, &Transform)>,
    mut selected: ResMut<SelectedTile>,
    mut highlight_q: Query<&mut Transform, (With<PaletteHighlight>, Without<PaletteEntry>)>,
) {
    if !mouse.just_pressed(MouseButton::Left) {
        return;
    }
    let Ok(window) = windows.single() else { return };
    let Ok((camera, cam_tf)) = camera_q.single() else { return };
    let Some(cursor) = window
        .cursor_position()
        .and_then(|p| camera.viewport_to_world_2d(cam_tf, p).ok())
    else {
        return;
    };

    for (entry, tf) in &palette_q {
        let half = CELL / 2.0;
        if (cursor.x - tf.translation.x).abs() < half
            && (cursor.y - tf.translation.y).abs() < half
        {
            selected.0 = entry.0;
            if let Ok(mut ht) = highlight_q.single_mut() {
                ht.translation.x = tf.translation.x;
                ht.translation.y = tf.translation.y;
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Input: canvas painting
// ---------------------------------------------------------------------------

fn canvas_paint(
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    camera_q: Query<(&Camera, &GlobalTransform)>,
    selected: Res<SelectedTile>,
    mut canvas: ResMut<EditorCanvas>,
) {
    let painting = mouse.pressed(MouseButton::Left);
    let erasing = mouse.pressed(MouseButton::Right);
    if !painting && !erasing {
        return;
    }
    let Ok(window) = windows.single() else { return };
    let Ok((camera, cam_tf)) = camera_q.single() else { return };
    let Some(cursor) = window
        .cursor_position()
        .and_then(|p| camera.viewport_to_world_2d(cam_tf, p).ok())
    else {
        return;
    };

    let canvas_origin_x = -(CANVAS_W as f32 * STEP) / 2.0;
    let canvas_origin_y = (CANVAS_H as f32 * STEP) / 2.0;

    let gx = ((cursor.x - canvas_origin_x + CELL / 2.0) / STEP) as i32;
    let gy = ((canvas_origin_y - cursor.y + CELL / 2.0) / STEP) as i32;

    if gx >= 0 && gx < CANVAS_W as i32 && gy >= 0 && gy < CANVAS_H as i32 {
        let tile = if erasing { canvas.default_tile } else { selected.0 };
        canvas.tiles[gy as usize][gx as usize] = tile;
    }
}

// ---------------------------------------------------------------------------
// Render: sync canvas sprites
// ---------------------------------------------------------------------------

fn sync_canvas_sprites(
    canvas: Res<EditorCanvas>,
    mut query: Query<(&CanvasCell, &mut Sprite)>,
) {
    if !canvas.is_changed() {
        return;
    }
    for (cell, mut sprite) in &mut query {
        sprite.color = tile_color(canvas.tiles[cell.1][cell.0]);
    }
}

// ---------------------------------------------------------------------------
// WFC generation
// ---------------------------------------------------------------------------

fn generate_wfc(
    keys: Res<ButtonInput<KeyCode>>,
    canvas: Res<EditorCanvas>,
    mut output: ResMut<OutputGrid>,
    mut commands: Commands,
    existing: Query<Entity, With<OutputCell>>,
) {
    if !keys.just_pressed(KeyCode::KeyG) {
        return;
    }

    for e in &existing {
        commands.entity(e).despawn();
    }

    let ow = CANVAS_W as u32 * OUTPUT_MULT;
    let oh = CANVAS_H as u32 * OUTPUT_MULT;

    let input_grid = Grid::new_fn(
        coord_2d::Size::new(CANVAS_W as u32, CANVAS_H as u32),
        |coord| canvas.tiles[coord.y as usize][coord.x as usize] as u16,
    );

    let pattern_size = NonZeroU32::new(PATTERN_SIZE).unwrap();
    let patterns = OverlappingPatterns::new(
        input_grid,
        pattern_size,
        &[wfc::orientation::Orientation::Original],
    );

    let global_stats = patterns.global_stats();
    let mut rng = rand::thread_rng();
    let output_size = coord_2d::Size::new(ow, oh);

    let run = RunOwn::new(output_size, &global_stats, &mut rng);
    let result: Result<Wave, _> = NumTimes(10).retry(run, &mut rng);

    let mut grid = vec![vec![Tile::Air; ow as usize]; oh as usize];
    match result {
        Ok(wave) => {
            for (coord, cell) in wave.grid().enumerate() {
                if let Ok(pattern_id) = cell.chosen_pattern_id() {
                    let &tile_val = patterns.pattern_top_left_value(pattern_id);
                    if let Ok(t) = Tile::try_from(tile_val) {
                        grid[coord.y as usize][coord.x as usize] = t;
                    }
                }
            }
        }
        Err(_) => {
            eprintln!("WFC generation failed after retries");
        }
    }

    let output_origin_x = (CANVAS_W as f32 * STEP) / 2.0 + 40.0;
    let output_origin_y = (oh as f32 * STEP) / 2.0;
    let scale = CANVAS_W as f32 / ow as f32;
    let cell_size = CELL * scale;
    let step = STEP * scale;

    for y in 0..oh as usize {
        for x in 0..ow as usize {
            let wx = output_origin_x + x as f32 * step;
            let wy = output_origin_y - y as f32 * step;
            commands.spawn((
                Sprite {
                    color: tile_color(grid[y][x]),
                    custom_size: Some(Vec2::splat(cell_size)),
                    ..default()
                },
                Transform::from_xyz(wx, wy, 0.0),
                OutputCell,
            ));
        }
    }

    output.0 = Some(grid);
    eprintln!("WFC generated {}x{} output", ow, oh);
}

// ---------------------------------------------------------------------------
// Camera pan & zoom
// ---------------------------------------------------------------------------

fn camera_controls(
    keys: Res<ButtonInput<KeyCode>>,
    mut camera_q: Query<&mut Transform, With<Camera2d>>,
    zoom: Res<CameraZoom>,
    time: Res<Time>,
) {
    let Ok(mut tf) = camera_q.single_mut() else { return };
    let speed = 400.0 * zoom.0 * time.delta_secs();

    if keys.pressed(KeyCode::ArrowLeft) {
        tf.translation.x -= speed;
    }
    if keys.pressed(KeyCode::ArrowRight) {
        tf.translation.x += speed;
    }
    if keys.pressed(KeyCode::ArrowUp) {
        tf.translation.y += speed;
    }
    if keys.pressed(KeyCode::ArrowDown) {
        tf.translation.y -= speed;
    }
}

fn camera_zoom(
    keys: Res<ButtonInput<KeyCode>>,
    mut camera_q: Query<&mut Transform, With<Camera2d>>,
    mut zoom: ResMut<CameraZoom>,
) {
    let Ok(mut tf) = camera_q.single_mut() else { return };
    if keys.pressed(KeyCode::Equal) {
        zoom.0 = (zoom.0 * 0.97).max(0.2);
    }
    if keys.pressed(KeyCode::Minus) {
        zoom.0 = (zoom.0 * 1.03).min(5.0);
    }
    tf.scale = Vec3::splat(zoom.0);
}

// ---------------------------------------------------------------------------
// Clear canvas
// ---------------------------------------------------------------------------

fn clear_canvas(keys: Res<ButtonInput<KeyCode>>, mut canvas: ResMut<EditorCanvas>) {
    if keys.just_pressed(KeyCode::KeyC) && !keys.pressed(KeyCode::ControlLeft) {
        let fill = canvas.default_tile;
        canvas.tiles.iter_mut().for_each(|row| row.fill(fill));
    }
}

// ---------------------------------------------------------------------------
// Export as prefab
// ---------------------------------------------------------------------------

fn export_prefab(keys: Res<ButtonInput<KeyCode>>, canvas: Res<EditorCanvas>) {
    if !keys.just_pressed(KeyCode::KeyE) {
        return;
    }

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

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

fn main() {
    let default_tile = Tile::Grass;
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Level Editor".into(),
                resolution: (1600, 900).into(),
                ..default()
            }),
            ..default()
        }))
        .insert_resource(ClearColor(Color::srgb(0.08, 0.08, 0.1)))
        .insert_resource(EditorCanvas {
            tiles: vec![vec![default_tile; CANVAS_W]; CANVAS_H],
            default_tile,
        })
        .insert_resource(SelectedTile(Tile::Wall))
        .insert_resource(OutputGrid(None))
        .insert_resource(CameraZoom(1.0))
        .add_systems(Startup, setup)
        .add_systems(Update, (
            palette_click,
            canvas_paint,
            sync_canvas_sprites,
            generate_wfc,
            camera_controls,
        ))
        .add_systems(Update, (
            camera_zoom,
            clear_canvas,
            export_prefab,
        ))
        .run();
}
