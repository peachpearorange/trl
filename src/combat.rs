use {bevy::prelude::*,
     rand::seq::SliceRandom,
     std::collections::{HashMap, HashSet},
     crate::{entities::{Collidable, DamageCloud, Enemy, GrenadeThrowComp, Location, Named,
                        Object, SporeEmitter, Stats, TimeSinceAction, WalkAroundRandomly, Wearing},
             tiles::Tile,
             ui::{LogEntries, log_message}}};

// ---------------------------------------------------------------------------
// Tile-entity spatial index
// ---------------------------------------------------------------------------

/// Maps (x, y, z) local tile coords to entities at that position.
/// Rebuilt from scratch each frame — simple, always correct.
#[derive(Resource, Default)]
pub struct TileEntityIndex(pub HashMap<(i32, i32, usize), Vec<Entity>>);

pub fn maintain_tile_index(
  mut index: ResMut<TileEntityIndex>,
  query: Query<(Entity, &Location), Without<crate::Player>>
) {
  index.0.clear();
  for (entity, location) in query.iter() {
    if let Location::Coords { x, y, z, .. } = location {
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
  target_wearing: Option<&Wearing>
) -> bool {
  let dmg = resolve_damage(attacker_attack, target_wearing);
  target_stats.hp -= dmg;
  target_stats.hp <= 0
}

/// Compute damage dealt to a target, accounting for armor DR.
pub fn resolve_damage(attack: i32, wearing: Option<&Wearing>) -> i32 {
  let dr = wearing.and_then(|w| w.0).map(|armor| armor.dr()).unwrap_or(0);
  (attack - dr).max(0)
}

// ---------------------------------------------------------------------------
// Enemy AI
// ---------------------------------------------------------------------------

/// Sim steps between moves/attacks for a given speed (actions per real-time second).
fn move_interval(move_speed: f32) -> u32 {
  (crate::SIM_STEPS_PER_SEC / move_speed).round().max(1.0) as u32
}

fn attack_interval(attack_speed: f32) -> u32 {
  (crate::SIM_STEPS_PER_SEC / attack_speed).round().max(1.0) as u32
}

fn step_toward(ex: i32, ey: i32, px: i32, py: i32) -> (i32, i32) {
  ((px - ex).signum(), (py - ey).signum())
}

/// True if the given local-space tile is impassable due to tile type or a collidable entity.
pub fn tile_blocked(
  level: &crate::level::Level,
  x: i32,
  y: i32,
  z: usize,
  index: &TileEntityIndex,
  collidable_q: &Query<&Collidable>
) -> bool {
  !level.walkable(x, y)
    || index.0.get(&(x, y, z)).is_some_and(|entities| {
      entities.iter().any(|&e| collidable_q.get(e).is_ok_and(|c| c.0))
    })
}

/// All 8-directional neighbor offsets, shuffled each call for unbiased wandering.
const NEIGHBOR_DIRS: [(i32, i32); 8] =
  [(-1, -1), (0, -1), (1, -1), (-1, 0), (1, 0), (-1, 1), (0, 1), (1, 1)];

pub fn npc_wander(
  current: Res<crate::CurrentZone>,
  index: Res<TileEntityIndex>,
  collidable_q: Query<&Collidable>,
  mut npc_q: Query<(&mut Location, &mut WalkAroundRandomly), Without<Enemy>>
) {
  let mut rng = rand::rng();
  for (mut location, mut wander) in npc_q.iter_mut() {
    wander.timer += 1;
    if wander.timer >= wander.interval
      && let Location::Coords { x, y, z, .. } = *location
    {
      wander.timer = 0;
      let level = current.0.level(z);
      let mut dirs = NEIGHBOR_DIRS;
      dirs.shuffle(&mut rng);
      if let Some(&(dx, dy)) = dirs.iter().find(|&&(dx, dy)| {
        let (nx, ny) = (x + dx, y + dy);
        !tile_blocked(level, nx, ny, z, &index, &collidable_q)
          && level.get(nx, ny) != Some(Tile::AirlockDoor)
      }) {
        *location = Location::xyz(x + dx, y + dy, z);
      }
    }
  }
}

pub fn enemy_ai(
  index: Res<TileEntityIndex>,
  current: Res<crate::CurrentZone>,
  clock: Res<crate::Clock>,
  mut tb: ResMut<crate::TurnBasedWorldState>,
  mut log: ResMut<LogEntries>,
  mut player_q: Query<
    (&crate::PlayerPos, &mut Stats),
    (With<crate::Player>, Without<Enemy>)
  >,
  mut enemy_q: Query<
    (&mut Location, &mut TimeSinceAction, &Stats, Option<&Wearing>, Option<&Named>),
    (With<Enemy>, Without<crate::Player>)
  >,
  collidable_q: Query<&Collidable>
) {
  if let Ok((player_pos, mut player_stats)) = player_q.single_mut() {
    let (px, py) = (player_pos.x, player_pos.y);
    let level = current.0.level(player_pos.z);

    let mut claimed: HashSet<(i32, i32)> = HashSet::new();

    for (mut location, mut timer, enemy_stats, enemy_wearing, enemy_named) in enemy_q.iter_mut() {
      timer.0 = timer.0.saturating_add(1);

      if let Location::Coords { x: ex, y: ey, z: ez, .. } = *location {
        let dist = (px - ex).abs().max((py - ey).abs());
        let atk_fr = attack_interval(enemy_stats.attack_speed);
        let mov_fr = move_interval(enemy_stats.move_speed);

        if dist == 1 && timer.0 >= atk_fr {
          let dmg = resolve_damage(enemy_stats.attack, enemy_wearing);
          player_stats.hp = (player_stats.hp - dmg).max(0);
          let name = enemy_named.map(|n| n.name).unwrap_or("Something");
          if dmg > 0 {
            log_message(&mut log, format!("{name} hits you for {dmg}."));
          } else {
            log_message(&mut log, format!("{name} hits you but deals no damage."));
          }
          if player_stats.hp == 0 {
            log_message(&mut log, "You died.".into());
          }
          timer.0 = 0;
        } else if timer.0 >= mov_fr {
          let (dx, dy) = step_toward(ex, ey, px, py);
          let (nex, ney) = (ex + dx, ey + dy);
          if !tile_blocked(level, nex, ney, ez, &index, &collidable_q)
            && !claimed.contains(&(nex, ney))
          {
            let below = ez
              .checked_sub(1)
              .map(|z1| current.0.level(z1).tiles[ney as usize][nex as usize]);
            let nz = if (level.tiles[ney as usize][nex as usize].causes_falling()
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
  if clock.mode == crate::TimeMode::TurnBased {
    tb.world_tick_pending = false;
  }
}

// ---------------------------------------------------------------------------
// Area-effect cloud systems
// ---------------------------------------------------------------------------

const SPORE_CLOUD_OFFSETS: [(i32, i32); 9] =
  [(0, 0), (-1, -1), (0, -1), (1, -1), (-1, 0), (1, 0), (-1, 1), (0, 1), (1, 1)];

// Manhattan radius 2: 13 tiles (diamond shape)
const EXPLOSION_OFFSETS: [(i32, i32); 13] = [
  (0, 0),
  (-1, 0), (1, 0), (0, -1), (0, 1),
  (-2, 0), (2, 0), (0, -2), (0, 2),
  (-1, -1), (1, -1), (-1, 1), (1, 1)
];

fn spawn_cloud_area(
  commands: &mut Commands,
  cx: i32,
  cy: i32,
  z: usize,
  obj: Object,
  offsets: &[(i32, i32)]
) {
  for &(dx, dy) in offsets {
    obj.clone().spawn_at(commands, cx + dx, cy + dy, z);
  }
}

/// Mushroom enemies with [`SporeEmitter`]: when the player is within range and the
/// cooldown has elapsed, burst a ring of spore clouds around the emitter.
pub fn mushroom_spore_attack(
  mut commands: Commands,
  mut log: ResMut<LogEntries>,
  player_q: Query<&crate::PlayerPos, With<crate::Player>>,
  mut emitter_q: Query<(&Location, &mut SporeEmitter, Option<&Named>), With<Enemy>>
) {
  if let Ok(&crate::PlayerPos { x: px, y: py, z: pz }) = player_q.single() {
    for (location, mut emitter, named) in emitter_q.iter_mut() {
      emitter.timer = emitter.timer.saturating_add(1);
      if let Location::Coords { x: ex, y: ey, z: ez, .. } = *location
        && ez == pz
        && (px - ex).abs().max((py - ey).abs()) <= 3
        && emitter.timer >= emitter.cooldown
      {
        emitter.timer = 0;
        let name = named.map(|n| n.name).unwrap_or("Something");
        log_message(&mut log, format!("{name} releases a cloud of spores!"));
        spawn_cloud_area(&mut commands, ex, ey, ez, Object::spore_cloud(), &SPORE_CLOUD_OFFSETS);
      }
    }
  }
}

/// Enemies with [`GrenadeThrowComp`]: lob a grenade at the player when beyond `min_range`.
pub fn grenade_thrower_ai(
  mut commands: Commands,
  mut log: ResMut<LogEntries>,
  player_q: Query<&crate::PlayerPos, With<crate::Player>>,
  mut thrower_q: Query<(&Location, &mut GrenadeThrowComp, Option<&Named>), With<Enemy>>
) {
  if let Ok(&crate::PlayerPos { x: px, y: py, z: pz }) = player_q.single() {
    for (location, mut comp, named) in thrower_q.iter_mut() {
      comp.timer = comp.timer.saturating_add(1);
      if let Location::Coords { z: ez, .. } = *location
        && ez == pz
        && let Location::Coords { x: ex, y: ey, .. } = *location
        && (px - ex).abs().max((py - ey).abs()) >= comp.min_range
        && comp.timer >= comp.cooldown
      {
        comp.timer = 0;
        let name = named.map(|n| n.name).unwrap_or("Something");
        log_message(&mut log, format!("{name} hurls a grenade!"));
        spawn_cloud_area(&mut commands, px, py, pz, Object::explosion_cloud(), &EXPLOSION_OFFSETS);
      }
    }
  }
}

/// Each sim step: advance [`DamageCloud`] timers, deal damage when the player shares a tile,
/// and despawn clouds whose lifetimes have expired.
pub fn damage_cloud_tick(
  mut commands: Commands,
  mut log: ResMut<LogEntries>,
  mut player_q: Query<(&crate::PlayerPos, &mut Stats), With<crate::Player>>,
  mut cloud_q: Query<(Entity, &Location, &mut DamageCloud, Option<&Named>)>
) {
  if let Ok((&crate::PlayerPos { x: px, y: py, z: pz }, mut player_stats)) =
    player_q.single_mut()
  {
    for (entity, location, mut cloud, named) in cloud_q.iter_mut() {
      cloud.tick_timer += 1;
      if cloud.tick_timer >= cloud.tick_interval {
        cloud.tick_timer = 0;
        if let &Location::Coords { x, y, z, .. } = location
          && x == px && y == py && z == pz
        {
          player_stats.hp = (player_stats.hp - cloud.damage_per_tick).max(0);
          let source = named.map(|n| n.name).unwrap_or("Something");
          log_message(&mut log, format!("{source} damages you for {}.", cloud.damage_per_tick));
        }
        if cloud.ticks_remaining <= 1 {
          commands.entity(entity).despawn();
        } else {
          cloud.ticks_remaining -= 1;
        }
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use {super::*,
       crate::entities::{Armor, Wearing}};

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
