use {
  bevy::prelude::*,
  std::collections::{HashMap, HashSet},
  trl::entities::{Enemy, Location, Stats, TimeSinceAction, Wearing},
};

// ---------------------------------------------------------------------------
// Tile-entity spatial index
// ---------------------------------------------------------------------------

/// Maps (x, y, z) tile coords to entities at that position across all levels.
/// Rebuilt from scratch each frame — simple, always correct.
#[derive(Resource, Default)]
pub struct TileEntityIndex(pub HashMap<(i32, i32, usize), Vec<Entity>>);

pub fn maintain_tile_index(
  mut index: ResMut<TileEntityIndex>,
  query: Query<(Entity, &Location)>,
) {
  index.0.clear();
  for (entity, location) in query.iter() {
    if let Location::Coords { x, y, z } = location {
      index.0.entry((*x, *y, *z)).or_default().push(entity);
    }
  }
}

// ---------------------------------------------------------------------------
// Damage calculation
// ---------------------------------------------------------------------------

/// Apply player attack to an enemy. Returns true if the enemy died.
/// Caller is responsible for despawning dead entities.
pub fn bump_attack(
  attacker_attack: i32,
  target_stats: &mut Stats,
  target_wearing: Option<&Wearing>,
) -> bool {
  let dmg = resolve_damage(attacker_attack, target_wearing);
  target_stats.hp -= dmg;
  target_stats.hp <= 0
}

/// Compute damage dealt to a target, accounting for armor DR.
pub fn resolve_damage(attack: i32, wearing: Option<&Wearing>) -> i32 {
  let dr = wearing
    .and_then(|w| w.0)
    .map(|armor| armor.dr())
    .unwrap_or(0);
  (attack - dr).max(0)
}

// ---------------------------------------------------------------------------
// Enemy AI
// ---------------------------------------------------------------------------

fn step_toward(ex: i32, ey: i32, px: i32, py: i32) -> (i32, i32) {
  ((px - ex).signum(), (py - ey).signum())
}

pub fn enemy_ai(
  time: Res<Time>,
  index: Res<TileEntityIndex>,
  cz: Res<crate::CurrentZ>,
  gw: Res<crate::GameWorld>,
  mut player_q: Query<(&crate::PlayerPos, &mut Stats), (With<crate::Player>, Without<Enemy>)>,
  mut enemy_q: Query<
    (&mut Location, &mut TimeSinceAction, &Stats, Option<&Wearing>),
    (With<Enemy>, Without<crate::Player>),
  >,
) {
  let Ok((player_pos, mut player_stats)) = player_q.single_mut() else { return };
  let (px, py) = (player_pos.x, player_pos.y);
  let level = gw.0.level(cz.0);
  let dt = time.delta_secs();

  let mut claimed: HashSet<(i32, i32)> = HashSet::new();

  for (mut location, mut timer, enemy_stats, enemy_wearing) in enemy_q.iter_mut() {
    timer.0 += dt;

    if let Location::Coords { x: ex, y: ey, z: ez } = *location
      && ez == cz.0
    {
      let dist = (px - ex).abs().max((py - ey).abs());

      if dist == 1 && timer.0 >= 1.0 / enemy_stats.attack_speed {
        let dmg = resolve_damage(enemy_stats.attack, enemy_wearing);
        player_stats.hp = (player_stats.hp - dmg).max(0);
        if player_stats.hp == 0 {
          bevy::log::info!("You died.");
        }
        timer.0 = 0.0;
      } else if timer.0 >= 1.0 / enemy_stats.move_speed {
        let (dx, dy) = step_toward(ex, ey, px, py);
        let (nx, ny) = (ex + dx, ey + dy);
        if level.walkable(nx, ny)
          && !index.0.contains_key(&(nx, ny, ez))
          && !claimed.contains(&(nx, ny))
        {
          let nz = if level.tiles[ny as usize][nx as usize] == crate::Tile::Pit && ez > 0 {
            ez - 1
          } else {
            ez
          };
          *location = Location::Coords { x: nx, y: ny, z: nz };
          claimed.insert((nx, ny));
          timer.0 = 0.0;
        }
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use trl::entities::{Armor, Wearing};

  #[test]
  fn no_armor_deals_full_damage() {
    assert_eq!(resolve_damage(5, None), 5);
  }

  #[test]
  fn armor_reduces_damage() {
    let wearing = Wearing(Some(Armor::Leather)); // DR 1
    assert_eq!(resolve_damage(5, Some(&wearing)), 4);
  }

  #[test]
  fn armor_cannot_go_below_zero() {
    let wearing = Wearing(Some(Armor::Plate)); // DR 3
    assert_eq!(resolve_damage(2, Some(&wearing)), 0);
  }

  #[test]
  fn chain_armor_dr() {
    let wearing = Wearing(Some(Armor::Chain)); // DR 2
    assert_eq!(resolve_damage(4, Some(&wearing)), 2);
  }

  #[test]
  fn zero_attack_deals_no_damage() {
    let wearing = Wearing(Some(Armor::Leather)); // DR 1
    assert_eq!(resolve_damage(0, Some(&wearing)), 0);
  }
}
