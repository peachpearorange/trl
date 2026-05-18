use {bevy::prelude::*,
     rand::seq::SliceRandom,
     std::collections::{BinaryHeap, HashMap, HashSet, VecDeque},
     std::cmp::Reverse,
     crate::{entities::{Collidable, DamageCloud, Enemy, FollowerData, FollowerState,
                        Gear, Glyph, GrenadeInFlight, Loadout, Location, Named,
                        Object, Path, Stats, TimeSinceAction,
                        WalkAroundRandomly},
             path_overlay::{bresenham_path, euclidean_los_point},
             particles::{ParticleEffects, spawn_bullet_trail, spawn_explosion_burst},
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
pub fn bump_attack(
  attacker_attack: i32,
  target_stats: &mut Stats,
  target_dr: i32
) -> bool {
  let dmg = resolve_damage(attacker_attack, target_dr);
  target_stats.hp -= dmg;
  target_stats.hp <= 0
}

pub fn resolve_damage(attack: i32, dr: i32) -> i32 {
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

// ---------------------------------------------------------------------------
// Pathfinding
// ---------------------------------------------------------------------------

fn chebyshev(a: (i32, i32), b: (i32, i32)) -> i32 {
  (a.0 - b.0).abs().max((a.1 - b.1).abs())
}

/// BFS flow field from `origin` outward. Each reachable tile maps to the adjacent tile
/// one step closer to `origin`, so enemies can look up their next move in O(1).
fn bfs_flow_field(origin: (i32, i32), level: &crate::level::Level) -> HashMap<(i32, i32), (i32, i32)> {
  let mut came_from: HashMap<(i32, i32), (i32, i32)> = HashMap::new();
  let mut queue: VecDeque<(i32, i32)> = VecDeque::new();
  came_from.insert(origin, origin);
  queue.push_back(origin);
  while let Some((x, y)) = queue.pop_front() {
    for &(dx, dy) in &NEIGHBOR_DIRS {
      let (nx, ny) = (x + dx, y + dy);
      if came_from.contains_key(&(nx, ny)) || !level.walkable(nx, ny) {
        continue;
      }
      if dx != 0 && dy != 0 && !level.walkable(x + dx, y) && !level.walkable(x, y + dy) {
        continue;
      }
      came_from.insert((nx, ny), (x, y));
      queue.push_back((nx, ny));
    }
  }
  came_from
}

/// Cached BFS flow field from the player's position. Recomputed whenever the player moves.
/// `field[tile]` = the adjacent tile one step closer to the player.
#[derive(Resource, Default)]
pub struct FlowField {
  field: HashMap<(i32, i32), (i32, i32)>,
  computed_for: Option<(i32, i32, usize)>,
}

pub fn compute_flow_field(
  current: Res<crate::CurrentZone>,
  player: Single<&crate::PlayerPos, With<crate::Player>>,
  mut flow: ResMut<FlowField>,
) {
  let pos = player.into_inner();
  let key = (pos.x, pos.y, pos.z);
  if flow.computed_for == Some(key) {
    return;
  }
  flow.field = bfs_flow_field((pos.x, pos.y), current.0.level(pos.z));
  flow.computed_for = Some(key);
}

/// A* pathfinding on a single z-level. Returns steps from start (exclusive) to goal (inclusive).
/// Only checks static tile walkability — dynamic entities are handled at execution time.
/// Returns an empty deque when start == goal or no path exists.
fn astar(
  start: (i32, i32),
  goal: (i32, i32),
  level: &crate::level::Level
) -> VecDeque<(i32, i32)> {
  if start == goal {
    return VecDeque::new();
  }

  // (f_score, g_score, x, y) — BinaryHeap is max-heap so we wrap in Reverse
  let mut open: BinaryHeap<Reverse<(i32, i32, i32, i32)>> = BinaryHeap::new();
  let mut came_from: HashMap<(i32, i32), (i32, i32)> = HashMap::new();
  let mut g_score: HashMap<(i32, i32), i32> = HashMap::new();

  g_score.insert(start, 0);
  open.push(Reverse((chebyshev(start, goal), 0, start.0, start.1)));

  while let Some(Reverse((_, g, x, y))) = open.pop() {
    if (x, y) == goal {
      let mut path = Vec::new();
      let mut cur = (x, y);
      while cur != start {
        path.push(cur);
        cur = came_from[&cur];
      }
      path.reverse();
      return VecDeque::from(path);
    }
    // Skip stale open-set entries
    if g_score.get(&(x, y)).copied().unwrap_or(i32::MAX) < g {
      continue;
    }
    for &(dx, dy) in &NEIGHBOR_DIRS {
      let (nx, ny) = (x + dx, y + dy);
      // Allow stepping onto the goal tile even if the entity standing there is non-walkable
      if !level.walkable(nx, ny) && (nx, ny) != goal {
        continue;
      }
      // Diagonal blocked only when both cardinal neighbours are impassable
      if dx != 0 && dy != 0 && !level.walkable(x + dx, y) && !level.walkable(x, y + dy) {
        continue;
      }
      let ng = g + 1;
      if ng < g_score.get(&(nx, ny)).copied().unwrap_or(i32::MAX) {
        g_score.insert((nx, ny), ng);
        came_from.insert((nx, ny), (x, y));
        open.push(Reverse((ng + chebyshev((nx, ny), goal), ng, nx, ny)));
      }
    }
  }

  VecDeque::new()
}

pub fn npc_wander(
  current: Res<crate::CurrentZone>,
  index: Res<TileEntityIndex>,
  collidable_q: Query<&Collidable>,
  mut npc_q: Query<(&mut Location, &mut WalkAroundRandomly, Option<&FollowerState>), Without<Enemy>>
) {
  let mut rng = rand::rng();
  for (mut location, mut wander, follower_state) in npc_q.iter_mut() {
    let wandering = follower_state.map_or(true, |s| *s == FollowerState::Available);
    wander.timer += 1;
    if wandering
      && wander.timer >= wander.interval
      && let Location::Coords { x, y, z, .. } = *location
    {
      wander.timer = 0;
      let level = current.0.level(z);
      let mut dirs = NEIGHBOR_DIRS;
      dirs.shuffle(&mut rng);
      if let Some(&(dx, dy)) = dirs.iter().find(|&&(dx, dy)| {
        let (nx, ny) = (x + dx, y + dy);
        !tile_blocked(level, nx, ny, z, &index, &collidable_q)
      }) {
        *location = Location::xyz(x + dx, y + dy, z);
      }
    }
  }
}

pub fn follower_ai(
  current: Res<crate::CurrentZone>,
  index: Res<TileEntityIndex>,
  collidable_q: Query<&Collidable>,
  player: Single<(&crate::PlayerPos, &Stats), With<crate::Player>>,
  mut follower_q: Query<(&mut Location, &mut FollowerState, &mut FollowerData, &Stats, &mut Path)>
) {
  let (player_pos, player_stats) = *player;
  let (px, py, pz) = (player_pos.x, player_pos.y, player_pos.z);

  for (mut location, mut state, mut data, stats, mut path) in follower_q.iter_mut() {
    if let Location::Coords { x: fx, y: fy, z: fz, .. } = *location {
      match *state {
        FollowerState::Available => {}
        FollowerState::Following => {
          let dist = (px - fx).abs().max((py - fy).abs());
          if fz == pz && dist > 2 {
            data.move_timer += 1;
            // Hurry to keep up: never move slower than the player when following
            let follow_speed = stats.move_speed.max(player_stats.move_speed);
            if data.move_timer >= move_interval(follow_speed) {
              data.move_timer = 0;
              let level = current.0.level(fz);
              let needs_recompute = path.steps.is_empty()
                || path.cached_goal.map_or(true, |g| chebyshev(g, (px, py)) > 1);
              if needs_recompute {
                path.steps = astar((fx, fy), (px, py), level);
                path.cached_goal = Some((px, py));
              }
              if let Some(&(nx, ny)) = path.steps.front() {
                if !tile_blocked(level, nx, ny, fz, &index, &collidable_q) {
                  path.steps.pop_front();
                  *location = Location::xyz(nx, ny, fz);
                } else {
                  path.steps.clear();
                }
              }
            }
          }
        }
        FollowerState::Dismissed => {
          let (hx, hy, hz) = data.home;
          if fx == hx && fy == hy && fz == hz {
            *state = FollowerState::Available;
            path.steps.clear();
            path.cached_goal = None;
          } else if fz == hz {
            data.move_timer += 1;
            if data.move_timer >= move_interval(stats.move_speed) {
              data.move_timer = 0;
              let level = current.0.level(fz);
              let needs_recompute = path.steps.is_empty()
                || path.cached_goal.map_or(true, |g| g != (hx, hy));
              if needs_recompute {
                path.steps = astar((fx, fy), (hx, hy), level);
                path.cached_goal = Some((hx, hy));
              }
              if let Some(&(nx, ny)) = path.steps.front() {
                if !tile_blocked(level, nx, ny, fz, &index, &collidable_q) {
                  path.steps.pop_front();
                  *location = Location::xyz(nx, ny, fz);
                } else {
                  path.steps.clear();
                }
              }
            }
          }
        }
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
  flow: Res<FlowField>,
  fov: Res<crate::Fov>,
  player: Single<
    (&crate::PlayerPos, &mut Stats, &Loadout),
    (With<crate::Player>, Without<Enemy>)
  >,
  mut enemy_q: Query<
    (&mut Location, &mut TimeSinceAction, &Stats, &Loadout, Option<&Named>, Option<&crate::entities::DriftChance>),
    (With<Enemy>, Without<crate::Player>)
  >,
  collidable_q: Query<&Collidable>
) {
  let (player_pos, ref mut player_stats, player_loadout) = player.into_inner();
  let (px, py, pz) = (player_pos.x, player_pos.y, player_pos.z);

  let mut claimed: HashSet<(i32, i32)> = HashSet::new();

  let mut rng = rand::rng();
  for (mut location, mut timer, enemy_stats, _enemy_loadout, enemy_named, drift) in enemy_q.iter_mut() {
    timer.0 = timer.0.saturating_add(1);

    if let Location::Coords { x: ex, y: ey, z: ez, .. } = *location {
      let dist = (px - ex).abs().max((py - ey).abs());
      if dist > 24 || !fov.0.is_visible(ex as usize, ey as usize) {
        continue;
      }
      let atk_fr = attack_interval(enemy_stats.attack_speed);
      let mov_fr = move_interval(enemy_stats.move_speed);

      if dist == 1 && timer.0 >= atk_fr {
        let dmg = resolve_damage(enemy_stats.attack, player_loadout.armor_dr());
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
      } else if ez == pz && timer.0 >= mov_fr {
        let level = current.0.level(ez);
        let next = if drift.is_some_and(|d| rand::Rng::random::<f32>(&mut rng) < d.0) {
          let mut dirs = NEIGHBOR_DIRS;
          dirs.shuffle(&mut rng);
          dirs.iter().find_map(|&(dx, dy)| {
            let (nx, ny) = (ex + dx, ey + dy);
            ((nx, ny) != (px, py)
              && !tile_blocked(level, nx, ny, ez, &index, &collidable_q)
              && !claimed.contains(&(nx, ny)))
            .then_some((nx, ny))
          })
        } else {
          flow.field.get(&(ex, ey)).copied().filter(|&step| step != (ex, ey)
            && step != (px, py)
            && !tile_blocked(level, step.0, step.1, ez, &index, &collidable_q)
            && !claimed.contains(&step))
        };
        if let Some((nex, ney)) = next {
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
  player_pos: Single<&crate::PlayerPos, With<crate::Player>>,
  mut emitter_q: Query<(&Location, &mut Loadout, Option<&Named>), With<Enemy>>
) {
  let &crate::PlayerPos { x: px, y: py, z: pz } = *player_pos;
  for (location, mut loadout, named) in emitter_q.iter_mut() {
    let Some(slot) = loadout.spore_mut() else { continue };
    slot.timer = slot.timer.saturating_add(1);
    if let Location::Coords { x: ex, y: ey, z: ez, .. } = *location
      && ez == pz
      && (px - ex).abs().max((py - ey).abs()) <= 3
      && slot.timer >= slot.cooldown
    {
      slot.timer = 0;
      let name = named.map(|n| n.name).unwrap_or("Something");
      log_message(&mut log, format!("{name} releases a cloud of spores!"));
      spawn_cloud_area(&mut commands, ex, ey, ez, Object::spore_cloud(), &SPORE_CLOUD_OFFSETS);
    }
  }
}

/// Enemies with [`GrenadeThrowComp`]: lob a grenade at the player when beyond `min_range`.
pub fn grenade_thrower_ai(
  mut commands: Commands,
  mut log: ResMut<LogEntries>,
  player_pos: Single<&crate::PlayerPos, With<crate::Player>>,
  mut thrower_q: Query<(&Location, &mut Loadout, Option<&Named>), With<Enemy>>
) {
  let &crate::PlayerPos { x: px, y: py, z: pz } = *player_pos;
  for (location, mut loadout, named) in thrower_q.iter_mut() {
    let Some(slot) = loadout.grenade_throw_mut() else { continue };
    let Gear::InnateGrenadeThrow { min_range } = slot.gear else { continue };
    slot.timer = slot.timer.saturating_add(1);
    if let Location::Coords { x: ex, y: ey, z: ez, .. } = *location
      && ez == pz
      && (px - ex).abs().max((py - ey).abs()) >= min_range
      && slot.timer >= slot.cooldown
    {
      slot.timer = 0;
      let name = named.map(|n| n.name).unwrap_or("Something");
      log_message(&mut log, format!("{name} hurls a grenade!"));
      let path = bresenham_path(ex, ey, px, py);
      commands.spawn((
        Glyph::palette_sprite(
          "textures/space_qud/grenade.png",
          'o',
          Color::srgb(0.85, 0.50, 0.10),
          Color::srgb(0.30, 0.20, 0.10)
        ),
        Location::xyz(ex, ey, pz),
        GrenadeInFlight { path, step: 0, tiles_per_turn: 4, z: pz }
      ));
    }
  }
}

pub fn gun_attacker_ai(
  mut commands: Commands,
  mut log: ResMut<LogEntries>,
  current: Res<crate::CurrentZone>,
  effects: Res<ParticleEffects>,
  player_pos: Single<&crate::PlayerPos, With<crate::Player>>,
  mut player_stats: Single<&mut Stats, (With<crate::Player>, Without<Enemy>)>,
  player_loadout: Single<&Loadout, (With<crate::Player>, Without<Enemy>)>,
  mut gunner_q: Query<(&Location, &mut Loadout, Option<&Named>), (With<Enemy>, Without<crate::Player>)>
) {
  let &crate::PlayerPos { x: px, y: py, z: pz } = *player_pos;
  let level = current.0.level(pz);
  let player_dr = player_loadout.armor_dr();
  for (location, mut loadout, named) in gunner_q.iter_mut() {
    let Some(slot) = loadout.gun_mut() else { continue };
    let Gear::InnateGun { damage } = slot.gear else { continue };
    slot.timer = slot.timer.saturating_add(1);
    if let Location::Coords { x: ex, y: ey, z: ez, .. } = *location
      && ez == pz
      && slot.timer >= slot.cooldown
      && euclidean_los_point(ex as f32 + 0.5, ey as f32 + 0.5, px, py, level).is_some()
    {
      slot.timer = 0;
      let name = named.map(|n| n.name).unwrap_or("Something");
      let path = bresenham_path(ex, ey, px, py);
      spawn_bullet_trail(&mut commands, &effects, &path, level.width, level.height);
      let dmg = (damage - player_dr).max(0);
      player_stats.hp = (player_stats.hp - dmg).max(0);
      if dmg > 0 {
        log_message(&mut log, format!("{name} shoots you for {dmg}."));
      } else {
        log_message(&mut log, format!("{name} shoots at you but deals no damage."));
      }
      if player_stats.hp == 0 {
        log_message(&mut log, "You died.".into());
      }
    }
  }
}

/// Detonate a grenade at (cx, cy, z): scatter explosion-cloud tiles on the walkable
/// portion of [`EXPLOSION_OFFSETS`] and play a particle burst.
fn detonate_grenade(
  commands: &mut Commands,
  effects: &ParticleEffects,
  level: &crate::level::Level,
  cx: i32,
  cy: i32,
  z: usize
) {
  for &(dx, dy) in &EXPLOSION_OFFSETS {
    let (ex, ey) = (cx + dx, cy + dy);
    if level.walkable(ex, ey) {
      Object::explosion_cloud().spawn_at(commands, ex, ey, z);
    }
  }
  spawn_explosion_burst(commands, effects, (cx, cy), level.width, level.height);
}

/// Each sim step: advance every [`GrenadeInFlight`] along its path by `tiles_per_turn`.
/// When a grenade reaches the end of its path it detonates and despawns.
pub fn tick_grenade_in_flight(
  mut commands: Commands,
  current: Res<crate::CurrentZone>,
  effects: Res<ParticleEffects>,
  mut grenade_q: Query<(Entity, &mut GrenadeInFlight, &mut Location)>
) {
  for (entity, mut grenade, mut location) in grenade_q.iter_mut() {
    let level = current.0.level(grenade.z);
    let last_idx = grenade.path.len().saturating_sub(1);
    let target_step = (grenade.step + grenade.tiles_per_turn).min(last_idx);
    let mut hit_wall = false;
    let mut next_step = grenade.step;
    for s in (grenade.step + 1)..=target_step {
      let &(sx, sy) = &grenade.path[s];
      if !level.walkable(sx, sy) {
        hit_wall = true;
        break;
      }
      next_step = s;
    }
    let &(nx, ny) = &grenade.path[next_step.max(grenade.step)];
    *location = Location::xyz(nx, ny, grenade.z);
    grenade.step = next_step;
    if hit_wall || next_step >= last_idx {
      detonate_grenade(&mut commands, &effects, level, nx, ny, grenade.z);
      commands.entity(entity).despawn();
    }
  }
}

/// Each sim step: advance [`DamageCloud`] timers, deal damage to any entity (player or enemy)
/// sharing a tile with the cloud, and despawn clouds whose lifetimes have expired.
pub fn damage_cloud_tick(
  mut commands: Commands,
  mut log: ResMut<LogEntries>,
  player: Single<(&crate::PlayerPos, &mut Stats), With<crate::Player>>,
  mut enemy_q: Query<(&Location, &mut Stats, Option<&Named>), (With<Enemy>, Without<crate::Player>)>,
  mut cloud_q: Query<(Entity, &Location, &mut DamageCloud, Option<&Named>)>
) {
  let (&crate::PlayerPos { x: px, y: py, z: pz }, ref mut player_stats) = player.into_inner();
  for (entity, location, mut cloud, source_name) in cloud_q.iter_mut() {
    cloud.tick_timer += 1;
    if cloud.tick_timer >= cloud.tick_interval {
      cloud.tick_timer = 0;
      if let &Location::Coords { x: cx, y: cy, z: cz, .. } = location {
        if cx == px && cy == py && cz == pz {
          player_stats.hp = (player_stats.hp - cloud.damage_per_tick).max(0);
          let source = source_name.map(|n| n.name).unwrap_or("Something");
          log_message(&mut log, format!("{source} damages you for {}.", cloud.damage_per_tick));
        }
        for (eloc, mut estats, _) in enemy_q.iter_mut() {
          if let &Location::Coords { x: ex, y: ey, z: ez, .. } = eloc
            && ex == cx && ey == cy && ez == cz
          {
            estats.hp -= cloud.damage_per_tick;
          }
        }
      }
      if cloud.ticks_remaining <= 1 {
        commands.entity(entity).despawn();
      } else {
        cloud.ticks_remaining -= 1;
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn no_armor_deals_full_damage() {
    assert_eq!(resolve_damage(5, 0), 5);
  }

  #[test]
  fn armor_reduces_damage() {
    assert_eq!(resolve_damage(5, 1), 4);
  }

  #[test]
  fn armor_cannot_go_below_zero() {
    assert_eq!(resolve_damage(2, 3), 0);
  }

  #[test]
  fn chain_armor_dr() {
    assert_eq!(resolve_damage(4, 2), 2);
  }

  #[test]
  fn zero_attack_deals_no_damage() {
    assert_eq!(resolve_damage(0, 1), 0);
  }
}
