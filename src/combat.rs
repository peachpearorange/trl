use {
  bevy::prelude::*,
  std::collections::{HashMap, HashSet},
  trl::entities::{Enemy, Location, Stats, TimeSinceAction, Wearing},
};

// ---------------------------------------------------------------------------
// Tile-entity spatial index
// ---------------------------------------------------------------------------

/// Maps (x, y, z) world tile coords to entities at that position.
/// Rebuilt from scratch each frame — simple, always correct.
/// Only indexes entities in the player's current zone.
#[derive(Resource, Default)]
pub struct TileEntityIndex(pub HashMap<(i32, i32, usize), Vec<Entity>>);

pub fn maintain_tile_index(
  mut index: ResMut<TileEntityIndex>,
  query: Query<(Entity, &Location)>,
  player_q: Query<&crate::PlayerPos, With<crate::Player>>,
) {
  index.0.clear();
  if let Ok(pos) = player_q.single() {
    let player_zx = pos.x as usize / crate::level::ZONE_WIDTH;
    let player_zy = pos.y as usize / crate::level::ZONE_HEIGHT;
    for (entity, location) in query.iter() {
      if let Location::Coords { x, y, z, .. } = location {
        let ezx = *x as usize / crate::level::ZONE_WIDTH;
        let ezy = *y as usize / crate::level::ZONE_HEIGHT;
        if ezx == player_zx && ezy == player_zy {
          index.0.entry((*x, *y, *z)).or_default().push(entity);
        }
      }
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

/// Assumed 60 display updates per real-time second; maps `Stats` speeds to frame counts.
const ASSUMED_RENDER_HZ: f32 = 60.0;

fn move_interval_frames(move_speed: f32) -> u32 {
  ((ASSUMED_RENDER_HZ / move_speed).round() as u32).max(1)
}

fn attack_interval_frames(attack_speed: f32) -> u32 {
  ((ASSUMED_RENDER_HZ / attack_speed).round() as u32).max(1)
}

fn step_toward(ex: i32, ey: i32, px: i32, py: i32) -> (i32, i32) {
  ((px - ex).signum(), (py - ey).signum())
}

pub fn enemy_ai(
  index: Res<TileEntityIndex>,
  gw: Res<crate::GameWorld>,
  mut player_q: Query<(&crate::PlayerPos, &mut Stats), (With<crate::Player>, Without<Enemy>)>,
  mut enemy_q: Query<
    (&mut Location, &mut TimeSinceAction, &Stats, Option<&Wearing>),
    (With<Enemy>, Without<crate::Player>),
  >,
) {
  if let Ok((player_pos, mut player_stats)) = player_q.single_mut() {
    let (px, py) = (player_pos.x, player_pos.y);
    let player_zx = px as usize / crate::level::ZONE_WIDTH;
    let player_zy = py as usize / crate::level::ZONE_HEIGHT;
    let level = gw.0.zone(player_zx, player_zy, player_pos.z);

    let mut claimed: HashSet<(i32, i32)> = HashSet::new();

    for (mut location, mut timer, enemy_stats, enemy_wearing) in enemy_q.iter_mut() {
      timer.0 = timer.0.saturating_add(1);

      if let Location::Coords { x: ex, y: ey, z: ez, .. } = *location {
        let ezx = ex as usize / crate::level::ZONE_WIDTH;
        let ezy = ey as usize / crate::level::ZONE_HEIGHT;
        if ezx != player_zx || ezy != player_zy || ez != player_pos.z { continue; }

        // Convert to local coords for tile access
        let lex = ex as usize % crate::level::ZONE_WIDTH;
        let ley = ey as usize % crate::level::ZONE_HEIGHT;
        let _ = (lex, ley); // used implicitly via world coord math below

        let dist = (px - ex).abs().max((py - ey).abs());
        let atk_fr = attack_interval_frames(enemy_stats.attack_speed);
        let mov_fr = move_interval_frames(enemy_stats.move_speed);

        if dist == 1 && timer.0 >= atk_fr {
          let dmg = resolve_damage(enemy_stats.attack, enemy_wearing);
          player_stats.hp = (player_stats.hp - dmg).max(0);
          if player_stats.hp == 0 {
            bevy::log::info!("You died.");
          }
          timer.0 = 0;
        } else if timer.0 >= mov_fr {
          let (dx, dy) = step_toward(ex, ey, px, py);
          let (nex, ney) = (ex + dx, ey + dy); // world coords
          let nlx = nex as usize % crate::level::ZONE_WIDTH;
          let nly = ney as usize % crate::level::ZONE_HEIGHT;
          if level.walkable(nlx as i32, nly as i32)
            && !index.0.contains_key(&(nex, ney, ez))
            && !claimed.contains(&(nex, ney))
          {
            let below = ez.checked_sub(1)
              .map(|z1| gw.0.zone(player_zx, player_zy, z1).tiles[nly][nlx]);
            let nz = if (level.tiles[nly][nlx].causes_falling()
              || below.is_some_and(|t| t.causes_falling()))
              && ez > 0
            {
              ez - 1
            } else {
              ez
            };
            *location = Location::xyz(nex, ney, nz);
            claimed.insert((nex, ney));
            timer.0 = 0;
          }
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
