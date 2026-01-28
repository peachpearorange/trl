use bevy::prelude::*;

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

fn main() {
  App::new()
    .add_plugins(DefaultPlugins.set(WindowPlugin {
      primary_window: Some(Window {
        title: "TRL - Terminal Roguelike".to_string(),
        resolution: (1280, 720).into(),
        ..default()
      }),
      ..default()
    }))
    .insert_resource(ClearColor(Color::srgb(0.05, 0.05, 0.1)))
    .add_systems(Startup, setup)
    // .add_systems(Update, handle_input)
    // .add_systems(Update, update_camera)
    // .add_systems(Update, render_game)
    .run();
}

fn setup(mut commands: Commands) {
  commands.spawn(Camera2d);

  let game = Game::new();
  commands.insert_resource(GameResource { game });
}

// TODO: Commented out - needs Bevy 0.17 API updates
// fn handle_input(...) { ... }
// fn update_camera(...) { ... }
// fn render_game(...) { ... }