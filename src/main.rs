use bevy::prelude::*;
use bevy::input::keyboard::KeyboardInput;
use bevy::input::ButtonState;

mod components;
mod game;
mod map;
mod spawnable;
mod tile_loader;

use game::Game;

#[derive(Resource)]
struct GameResource {
  game: Game
}

#[derive(Resource)]
struct AssetHandles {
  player_texture: Handle<Image>,
  goblin_texture: Handle<Image>,
  dragon_texture: Handle<Image>,
  slime_texture: Handle<Image>
}

fn main() {
  App::new()
    .add_plugins(DefaultPlugins.set(WindowPlugin {
      primary_window: Some(Window {
        title: "TRL - Terminal Roguelike".to_string(),
        resolution: (1280.0, 720.0).into(),
        ..default()
      }),
      ..default()
    }))
    .insert_resource(ClearColor(Color::rgb(0.05, 0.05, 0.1)))
    .add_systems(Startup, setup)
    .add_systems(Update, handle_input)
    .add_systems(Update, update_camera)
    .add_systems(Update, render_game)
    .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
  commands.spawn(Camera2dBundle::default());

  let game = Game::new();
  commands.insert_resource(GameResource { game });

  commands.insert_resource(AssetHandles {
    player_texture: asset_server.load("textures/player.png"),
    goblin_texture: asset_server.load("textures/goblin.png"),
    dragon_texture: asset_server.load("textures/dragon.png"),
    slime_texture: asset_server.load("textures/slime.png")
  });
}

fn handle_input(
  keyboard_input: Res<ButtonInput<KeyCode>>,
  mut game_res: ResMut<GameResource>,
  mut q_camera: Query<&mut Transform, With<Camera>>
) {
  let mut moved = false;

  if keyboard_input.pressed(KeyCode::KeyW) || keyboard_input.pressed(KeyCode::ArrowUp) {
    game_res.game.player_move(0, -1);
    moved = true;
  }
  if keyboard_input.pressed(KeyCode::KeyS) || keyboard_input.pressed(KeyCode::ArrowDown) {
    game_res.game.player_move(0, 1);
    moved = true;
  }
  if keyboard_input.pressed(KeyCode::KeyA) || keyboard_input.pressed(KeyCode::ArrowLeft) {
    game_res.game.player_move(-1, 0);
    moved = true;
  }
  if keyboard_input.pressed(KeyCode::KeyD) || keyboard_input.pressed(KeyCode::ArrowRight) {
    game_res.game.player_move(1, 0);
    moved = true;
  }
  if keyboard_input.just_pressed(KeyCode::Period) {
    game_res.game.descend();
    moved = true;
  }
  if keyboard_input.just_pressed(KeyCode::Comma) {
    game_res.game.ascend();
    moved = true;
  }
}

fn update_camera(
  game_res: Res<GameResource>,
  mut q_camera: Query<&mut Transform, With<Camera>>
) {
  let player_pos = game_res.game.state.player.pos;
  let mut transform = q_camera.single_mut();

  let target_x = player_pos.x as f32 * 16.0;
  let target_y = player_pos.y as f32 * 16.0;

  transform.translation.x = transform.translation.x.lerp(target_x, 0.1);
  transform.translation.y = transform.translation.y.lerp(target_y, 0.1);
}

fn render_game(
  mut commands: Commands,
  game_res: Res<GameResource>,
  asset_handles: Res<AssetHandles>,
  q_sprites: Query<Entity, With<Sprite>>,
  materials: Res<Assets<ColorMaterial>>
) {
  for entity in q_sprites.iter() {
    commands.entity(entity).despawn_recursive();
  }

  let game = &game_res.game;
  let map = game.current_map();

  for y in 0..map.height {
    for x in 0..map.width {
      let tile = &map.tiles[y][x];

      if !tile.visible {
        continue;
      }

      let (color, size) = match tile.tile_type {
        components::TileType::Floor => (Color::rgb(0.3, 0.3, 0.3), 16.0),
        components::TileType::Wall => (Color::rgb(0.5, 0.5, 0.5), 16.0),
        components::TileType::StairsDown => (Color::rgb(1.0, 0.8, 0.2), 16.0),
        components::TileType::StairsUp => (Color::rgb(0.8, 1.0, 0.2), 16.0),
        components::TileType::Water => (Color::rgb(0.2, 0.4, 0.8), 16.0),
        components::TileType::Grass => (Color::rgb(0.2, 0.6, 0.2), 16.0),
        components::TileType::Sand => (Color::rgb(0.8, 0.7, 0.4), 16.0)
      };

      commands.spawn(SpriteBundle {
        sprite: Sprite {
          color,
          custom_size: Some(Vec2::new(size, size)),
          ..default()
        },
        transform: Transform::from_xyz(x as f32 * 16.0 - 640.0, y as f32 * -16.0 + 360.0, 0.0),
        ..default()
      });
    }
  }

  let player = &game.state.player;
  commands.spawn(SpriteBundle {
    texture: asset_handles.player_texture.clone(),
    transform: Transform::from_xyz(
      player.pos.x as f32 * 16.0 - 640.0,
      player.pos.y as f32 * -16.0 + 360.0,
      1.0
    ),
    sprite: Sprite {
      custom_size: Some(Vec2::new(48.0, 48.0)),
      ..default()
    },
    ..default()
  });
}