use {bevy::prelude::*,
     rand::seq::SliceRandom,
     std::collections::{BinaryHeap, HashMap, HashSet, VecDeque},
     std::cmp::Reverse,
     crate::{entities::{Collidable, DamageCloud, Enemy, FollowerData, FollowerState,
                        Gear, Glyph, Grabbed, GrenadeInFlight, Invisible, Loadout, Location, Named, Phasing,
                        Object, Path, Stats, TimeSinceAction,
                        WalkAroundRandomly},
             level::Item,
             path_overlay::{dda_cells, euclidean_los_point},
             particles::{GunBullet, GunBulletVisual, ParticleEffects, PendingImpact,
                         spawn_explosion_burst, spawn_gun_bullet, tile_to_world},
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
  query: Query<(Entity, &Location)>
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
const FLOW_FIELD_RADIUS: i32 = 26;
const FLOW_SIDE: i32 = FLOW_FIELD_RADIUS * 2 + 1;

/// Local-offset index into the dense grid for a world tile, or None if outside the
/// [origin ± RADIUS] window. The window is a square of half-side RADIUS, so being in
/// bounds is exactly Chebyshev distance <= RADIUS.
fn flow_idx(origin: (i32, i32), x: i32, y: i32) -> Option<usize> {
  let lx = x - origin.0 + FLOW_FIELD_RADIUS;
  let ly = y - origin.1 + FLOW_FIELD_RADIUS;
  (lx >= 0 && lx < FLOW_SIDE && ly >= 0 && ly < FLOW_SIDE).then(|| (ly * FLOW_SIDE + lx) as usize)
}

fn bfs_flow_field(origin: (i32, i32), level: &crate::level::Level) -> Vec<Option<(i32, i32)>> {
  let mut field: Vec<Option<(i32, i32)>> = vec![None; (FLOW_SIDE * FLOW_SIDE) as usize];
  let mut queue: VecDeque<(i32, i32)> = VecDeque::new();
  // Origin points to itself (consumers filter out the self-step), doubling as the visited mark.
  field[flow_idx(origin, origin.0, origin.1).unwrap()] = Some(origin);
  queue.push_back(origin);
  while let Some((x, y)) = queue.pop_front() {
    for &(dx, dy) in &NEIGHBOR_DIRS {
      let (nx, ny) = (x + dx, y + dy);
      if let Some(i) = flow_idx(origin, nx, ny)
        && field[i].is_none()
        && level.walkable(nx, ny)
        && (dx == 0 || dy == 0 || level.walkable(x + dx, y) || level.walkable(x, y + dy))
      {
        field[i] = Some((x, y));
        queue.push_back((nx, ny));
      }
    }
  }
  field
}

/// Cached BFS flow field from the player's position. Recomputed whenever the player moves.
/// A dense grid over the [origin ± RADIUS] window; each cell holds the adjacent tile one
/// step closer to the player. Indexed by local offset so lookups need no hashing.
#[derive(Resource, Default)]
pub struct FlowField {
  origin: (i32, i32),
  field: Vec<Option<(i32, i32)>>,
  computed_for: Option<(i32, i32, usize)>,
}

impl FlowField {
  pub fn next_step(&self, x: i32, y: i32) -> Option<(i32, i32)> {
    flow_idx(self.origin, x, y).and_then(|i| self.field[i])
  }
}

pub fn compute_flow_field(
  current: Res<crate::CurrentZone>,
  player: Single<&Location, With<crate::Player>>,
  mut flow: ResMut<FlowField>,
) {
  let &Location::Coords { x, y, z, .. } = &*player.into_inner() else { unreachable!() };
  let key = (x, y, z);
  if flow.computed_for != Some(key) {
    flow.origin = (x, y);
    flow.field = bfs_flow_field((x, y), current.0.level(z));
    flow.computed_for = Some(key);
  }
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
  let mut rng = rand::thread_rng();
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
        location.move_to(x + dx, y + dy, z);
      }
    }
  }
}

pub fn follower_ai(
  current: Res<crate::CurrentZone>,
  index: Res<TileEntityIndex>,
  collidable_q: Query<&Collidable>,
  player: Single<(&Location, &Stats), With<crate::Player>>,
  mut follower_q: Query<(&mut Location, &mut FollowerState, &mut FollowerData, &Stats, &mut Path), Without<crate::Player>>
) {
  let (player_loc, player_stats) = *player;
  let &Location::Coords { x: px, y: py, z: pz, .. } = player_loc else { unreachable!() };

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
                  location.move_to(nx, ny, fz);
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
                  location.move_to(nx, ny, fz);
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
  mut commands: Commands,
  frame: Res<crate::RenderFrame>,
  index: Res<TileEntityIndex>,
  current: Res<crate::CurrentZone>,
  clock: Res<crate::Clock>,
  mut tb: ResMut<crate::TurnBasedWorldState>,
  mut log: ResMut<LogEntries>,
  flow: Res<FlowField>,
  fov: Res<crate::Fov>,
  player: Single<
    (Entity, &Location, &mut Stats, &Loadout, Option<&Invisible>),
    (With<crate::Player>, Without<Enemy>)
  >,
  mut enemy_q: Query<
    (Entity, &mut Location, &mut TimeSinceAction, &Stats, &mut Loadout, Option<&Named>, Option<&crate::entities::DriftChance>),
    (With<Enemy>, Without<crate::Player>)
  >,
  collidable_q: Query<&Collidable>
) {
  let (player_entity, player_loc, ref mut player_stats, player_loadout, player_invis) = player.into_inner();
  let &Location::Coords { x: px, y: py, z: pz, .. } = player_loc else { unreachable!() };
  let player_invisible = player_invis.is_some();

  let mut claimed: HashSet<(i32, i32)> = HashSet::new();

  let mut rng = rand::thread_rng();
  for (enemy_entity, mut location, mut timer, enemy_stats, mut enemy_loadout, enemy_named, drift) in enemy_q.iter_mut() {
    timer.attack = timer.attack.saturating_add(1);
    timer.movement = timer.movement.saturating_add(1);
    if let Some(grab_slot) = enemy_loadout.grab_mut() {
      grab_slot.timer = grab_slot.timer.saturating_add(1);
    }

    if let Location::Coords { x: ex, y: ey, z: ez, .. } = *location
        && ez == pz
        && let dist = (px - ex).abs().max((py - ey).abs())
        && !(player_invisible && dist > 1)
        && dist <= 24 && fov.0.is_visible(ex as usize, ey as usize)
    {
      let atk_fr = attack_interval(enemy_stats.attack_speed);
      let mov_fr = move_interval(enemy_stats.move_speed);

      if dist == 1 && timer.attack >= atk_fr {
        let dmg = resolve_damage(enemy_stats.attack, player_loadout.armor_dr());
        player_stats.hp = (player_stats.hp - dmg).max(0);
        let name = enemy_named.map(|n| n.name.as_ref()).unwrap_or("Something");
        if dmg > 0 {
          log_message(&mut log, format!("{name} hits you for {dmg}."));
        } else {
          log_message(&mut log, format!("{name} hits you but deals no damage."));
        }
        if let Some(grab_slot) = enemy_loadout.grab_mut()
            && grab_slot.timer >= grab_slot.cooldown
        {
          commands.entity(player_entity).insert(Grabbed { by: enemy_entity, turns_remaining: 3 });
          log_message(&mut log, format!("{name} grabs you!"));
          grab_slot.timer = 0;
        }
        if player_stats.hp == 0 {
          log_message(&mut log, "You died.".into());
        }
        commands.entity(enemy_entity).insert(crate::BumpLunge {
          dir: Vec2::new((px - ex) as f32, (py - ey) as f32),
          start_frame: frame.0,
        });
        timer.attack = 0;
      }
      if timer.movement >= mov_fr && dist > 1 {
        let level = current.0.level(ez);
        let next = if drift.is_some_and(|d| rand::Rng::r#gen::<f32>(&mut rng) < d.0) {
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
          flow.next_step(ex, ey).filter(|&step| step != (ex, ey)
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
          location.move_to(nex, ney, nz);
          claimed.insert((nex, ney));
          timer.movement = 0;
        }
      }
    }
  }
  if clock.mode == crate::TimeMode::TurnBased {
    tb.world_tick_pending = tb.world_tick_pending.saturating_sub(1);
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
  player_q: Single<(&Location, Option<&Invisible>), With<crate::Player>>,
  mut emitter_q: Query<(&Location, &mut Loadout, Option<&Named>), With<Enemy>>
) {
  let (&Location::Coords { x: px, y: py, z: pz, .. }, player_invis) = *player_q else { unreachable!() };
  if player_invis.is_none() {
    for (location, mut loadout, named) in emitter_q.iter_mut() {
    let Some(slot) = loadout.spore_mut() else { continue };
    slot.timer = slot.timer.saturating_add(1);
    if let Location::Coords { x: ex, y: ey, z: ez, .. } = *location
      && ez == pz
      && (px - ex).abs().max((py - ey).abs()) <= 3
      && slot.timer >= slot.cooldown
    {
      slot.timer = 0;
      let name = named.map(|n| n.name.as_ref()).unwrap_or("Something");
      log_message(&mut log, format!("{name} releases a cloud of spores!"));
      spawn_cloud_area(&mut commands, ex, ey, ez, Object::SPORE_CLOUD, &SPORE_CLOUD_OFFSETS);
    }
  }
  }
}

/// Enemies with [`GrenadeThrowComp`]: lob a grenade at the player when beyond `min_range`.
pub fn grenade_thrower_ai(
  mut commands: Commands,
  mut log: ResMut<LogEntries>,
  player_q: Single<(&Location, Option<&Invisible>), With<crate::Player>>,
  mut thrower_q: Query<(&Location, &mut Loadout, Option<&Named>), With<Enemy>>
) {
  let (&Location::Coords { x: px, y: py, z: pz, .. }, player_invis) = *player_q else { unreachable!() };
  if player_invis.is_none() {
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
      let name = named.map(|n| n.name.as_ref()).unwrap_or("Something");
      log_message(&mut log, format!("{name} hurls a grenade!"));
      let from = Vec2::new(ex as f32 + 0.5, ey as f32 + 0.5);
      let to   = Vec2::new(px as f32 + 0.5, py as f32 + 0.5);
      commands.spawn((
        Glyph::recolor_sprite(
          "textures/space_qud/grenade.png",
          'o',
          Color::srgb(0.85, 0.50, 0.10),
          Color::srgb(0.30, 0.20, 0.10)
        ),
        Location::xyz(ex, ey, pz),
        GrenadeInFlight {
          dir: (to - from).normalize(),
          pos: from,
          target: to,
          tiles_per_turn: 4.0,
          z: pz,
          item: Item::FragGrenade
        }
      ));
    }
  }
  }
}

pub fn gun_attacker_ai(
  mut commands: Commands,
  mut log: ResMut<LogEntries>,
  effects: Res<ParticleEffects>,
  current: Res<crate::CurrentZone>,
  player_q: Single<(&Location, Option<&Invisible>), (With<crate::Player>, Without<Enemy>)>,
  player_loadout: Single<&Loadout, (With<crate::Player>, Without<Enemy>)>,
  mut gunner_q: Query<(&Location, &mut Loadout, Option<&Named>), (With<Enemy>, Without<crate::Player>)>
) {
  let (&Location::Coords { x: px, y: py, z: pz, .. }, player_invis) = *player_q else { unreachable!() };
  if player_invis.is_none() {
    let level = current.0.level(pz);
    let player_dr = player_loadout.armor_dr();
    for (location, mut loadout, named) in gunner_q.iter_mut() {
      let Some(slot) = loadout.gun_mut() else { continue };
      let Gear::InnateGun { damage } = slot.gear else { continue };
      slot.timer = slot.timer.saturating_add(1);
      if let Location::Coords { x: ex, y: ey, z: ez, .. } = *location
        && ez == pz
        && slot.timer >= slot.cooldown
        && let Some((aim_x, aim_y)) = euclidean_los_point(ex as f32 + 0.5, ey as f32 + 0.5, px, py, level)
      {
        slot.timer = 0;
        let name = named.map(|n| n.name.as_ref()).unwrap_or("Something");
        log_message(&mut log, format!("{name} fires at you!"));
        let dmg = (damage - player_dr).max(0);
        spawn_gun_bullet(&mut commands, &effects, ex, ey, aim_x, aim_y, dmg, false, ez, level.width, level.height);
      }
    }
  }
}

/// Each sim step: advance every [`GunBullet`] along its Euclidean line, check for
/// wall/entity hits, apply damage and despawn on contact.
pub fn advance_gun_bullets(
  mut commands: Commands,
  current: Res<crate::CurrentZone>,
  index: Res<TileEntityIndex>,
  effects: Res<ParticleEffects>,
  mut log: ResMut<LogEntries>,
  mut stats_q: Query<(&mut Stats, Option<&Named>, Has<crate::Player>)>,
  mut bullets: Query<(Entity, &mut GunBullet)>,
  mut visuals: Query<&mut GunBulletVisual>
) {
  for (ent, mut bullet) in bullets.iter_mut() {
    let level = current.0.level(bullet.z);
    let remaining = (bullet.target - bullet.pos).length();
    let step = bullet.tiles_per_turn.min(remaining);
    let new_pos = bullet.pos + bullet.dir * step;
    let cells = dda_cells(bullet.pos.x, bullet.pos.y, new_pos.x, new_pos.y);
    let mut final_pos = new_pos;
    let mut hit_wall = false;
    let mut hit_tile: Option<(i32, i32)> = None;
    for &(cx, cy) in cells.iter().skip(1) {
      if !level.walkable(cx, cy) {
        hit_wall = true;
        final_pos = Vec2::new(cx as f32 + 0.5 - bullet.dir.x, cy as f32 + 0.5 - bullet.dir.y);
        break;
      }
      if index.0.get(&(cx, cy, bullet.z)).is_some_and(|ents| ents.iter().any(|&e| stats_q.get(e).is_ok())) {
        hit_tile = Some((cx, cy));
        final_pos = Vec2::new(cx as f32 + 0.5, cy as f32 + 0.5);
        break;
      }
    }
    bullet.pos = final_pos;
    let cur_world = tile_to_world(final_pos.x, final_pos.y, level.width, level.height);

    if let Some(emitter) = bullet.emitter
      && let Ok(mut visual) = visuals.get_mut(emitter)
    {
      visual.dest = cur_world;
    }

    let despawn = hit_tile.is_some() || hit_wall || step >= remaining;
    if let Some((cx, cy)) = hit_tile {
      let hit_ents: Vec<Entity> = index.0.get(&(cx, cy, bullet.z)).map(|v| v.clone()).unwrap_or_default();
      for e in hit_ents {
        if let Ok((mut stats, named, is_player)) = stats_q.get_mut(e) {
          stats.hp = (stats.hp - bullet.damage).max(0);
          if bullet.is_player {
            log_message(&mut log, format!("You shoot {} for {}!", named.map(|n| n.name.as_ref()).unwrap_or("it"), bullet.damage));
          } else if is_player {
            log_message(&mut log, format!("The bullet hits you for {}!", bullet.damage));
            if stats.hp == 0 {
              log_message(&mut log, "You died.".into());
            }
          }
        }
      }
    } else if (hit_wall || step >= remaining) && bullet.is_player {
      log_message(&mut log, "Your shot hits nothing.".into());
    }
    if despawn {
      if let Some(emitter) = bullet.emitter {
        commands.entity(emitter).insert(PendingImpact {
          effect: effects.bullet_spark.clone(),
          pos: cur_world
        });
      }
      commands.entity(ent).despawn();
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
  z: usize,
  item: Item
) {
  let cloud = match item {
    Item::FrostScroll => Object::FROST_CLOUD,
    Item::LightningScroll => Object::LIGHTNING_CLOUD,
    Item::VoidScroll => Object::VOID_CLOUD,
    _ => Object::EXPLOSION_CLOUD
  };
  for &(dx, dy) in &EXPLOSION_OFFSETS {
    let (ex, ey) = (cx + dx, cy + dy);
    if level.walkable(ex, ey) {
      cloud.spawn_at(commands, ex, ey, z);
    }
  }
  spawn_explosion_burst(commands, effects, (cx, cy), level.width, level.height);
}

/// Each sim step: advance every [`GrenadeInFlight`] along its Euclidean line by `tiles_per_turn`.
/// Detonates and despawns on wall hit or arrival at target.
pub fn tick_grenade_in_flight(
  mut commands: Commands,
  current: Res<crate::CurrentZone>,
  effects: Res<ParticleEffects>,
  mut grenade_q: Query<(Entity, &mut GrenadeInFlight, &mut Location)>
) {
  for (entity, mut grenade, mut location) in grenade_q.iter_mut() {
    let level = current.0.level(grenade.z);
    let remaining = (grenade.target - grenade.pos).length();
    let step = grenade.tiles_per_turn.min(remaining);
    let new_pos = grenade.pos + grenade.dir * step;
    let cells = dda_cells(grenade.pos.x, grenade.pos.y, new_pos.x, new_pos.y);
    let mut final_tile = (grenade.pos.x.floor() as i32, grenade.pos.y.floor() as i32);
    let mut hit_wall = false;
    for &(cx, cy) in cells.iter().skip(1) {
      if !level.walkable(cx, cy) {
        hit_wall = true;
        break;
      }
      final_tile = (cx, cy);
    }
    grenade.pos = Vec2::new(final_tile.0 as f32 + 0.5, final_tile.1 as f32 + 0.5);
    location.move_to(final_tile.0, final_tile.1, grenade.z);
    if hit_wall || step >= remaining {
      detonate_grenade(&mut commands, &effects, level, final_tile.0, final_tile.1, grenade.z, grenade.item);
      commands.entity(entity).despawn();
    }
  }
}

/// Each sim step: advance [`DamageCloud`] timers, deal damage to any entity with [`Stats`]
/// sharing a tile with the cloud, and despawn clouds whose lifetimes have expired.
pub fn damage_cloud_tick(
  mut commands: Commands,
  mut log: ResMut<LogEntries>,
  index: Res<TileEntityIndex>,
  mut stats_q: Query<(&mut Stats, Has<crate::Player>)>,
  mut cloud_q: Query<(Entity, &Location, &mut DamageCloud, Option<&Named>)>
) {
  for (cloud_ent, location, mut cloud, source_name) in cloud_q.iter_mut() {
    cloud.tick_timer += 1;
    if cloud.tick_timer >= cloud.tick_interval
      && let &Location::Coords { x: cx, y: cy, z: cz, .. } = location
    {
      cloud.tick_timer = 0;
      if let Some(ents) = index.0.get(&(cx, cy, cz)) {
        for &ent in ents {
          if let Ok((mut stats, is_player)) = stats_q.get_mut(ent) {
            stats.hp = (stats.hp - cloud.damage_per_tick).max(0);
            if is_player {
              let source = source_name.map(|n| n.name.as_ref()).unwrap_or("Something");
              log_message(&mut log, format!("{source} damages you for {}.", cloud.damage_per_tick));
            }
          }
        }
      }
      if cloud.ticks_remaining <= 1 {
        commands.entity(cloud_ent).despawn();
      } else {
        cloud.ticks_remaining -= 1;
      }
    }
  }
}

pub fn tick_grabbed(
  mut commands: Commands,
  mut log: ResMut<LogEntries>,
  mut grabbed_q: Query<(Entity, &mut Grabbed)>,
  enemy_q: Query<(&Location, Option<&Named>), With<Enemy>>
) {
  for (entity, mut grabbed) in grabbed_q.iter_mut() {
    if enemy_q.get(grabbed.by).is_err() {
      commands.entity(entity).remove::<Grabbed>();
      log_message(&mut log, "You break free!".into());
    } else if grabbed.turns_remaining <= 1 {
      commands.entity(entity).remove::<Grabbed>();
      log_message(&mut log, "You break free from the grab!".into());
    } else {
      grabbed.turns_remaining -= 1;
    }
  }
}

pub fn tick_invisible(
  mut commands: Commands,
  mut log: ResMut<LogEntries>,
  mut invis_q: Query<(Entity, &mut Invisible, Option<&Named>)>
) {
  for (entity, mut invis, named) in invis_q.iter_mut() {
    if invis.0 <= 1 {
      commands.entity(entity).remove::<Invisible>();
      let name = named.map(|n| n.name.as_ref()).unwrap_or("You");
      log_message(&mut log, format!("{name} shimmer back into visibility."));
    } else {
      invis.0 -= 1;
    }
  }
}

pub fn tick_phasing(
  mut commands: Commands,
  mut log: ResMut<LogEntries>,
  mut q: Query<(Entity, &mut Phasing)>
) {
  for (entity, mut p) in q.iter_mut() {
    if p.0 <= 1 {
      commands.entity(entity).remove::<Phasing>();
      log_message(&mut log, "You resolidify.".into());
    } else {
      p.0 -= 1;
    }
  }
}

pub fn enemy_stealth_ai(
  mut commands: Commands,
  mut log: ResMut<LogEntries>,
  player_pos: Single<&Location, With<crate::Player>>,
  fov: Res<crate::Fov>,
  mut enemy_q: Query<(Entity, &Location, &mut Loadout, Option<&Named>, Option<&Invisible>), With<Enemy>>
) {
  let Location::Coords { x: px, y: py, z: pz, .. } = **player_pos else { unreachable!() };
  for (entity, location, mut loadout, named, already_invis) in enemy_q.iter_mut() {
    if already_invis.is_some() { continue; }
    let has_device = loadout.gear.iter().any(|s| matches!(s.gear, Gear::Device(crate::level::Item::StealthDevice)));
    if !has_device { continue; }
    if let Location::Coords { x: ex, y: ey, z: ez, .. } = *location
      && ez == pz
      && (px - ex).abs().max((py - ey).abs()) <= 12
      && fov.0.is_visible(ex as usize, ey as usize)
    {
      if let Some(idx) = loadout.gear.iter().position(|s| matches!(s.gear, Gear::Device(crate::level::Item::StealthDevice))) {
        let gear = loadout.gear.to_mut();
        gear[idx].count -= 1;
        if gear[idx].count == 0 {
          gear.remove(idx);
        }
      }
      commands.entity(entity).insert(Invisible(20));
      let name = named.map(|n| n.name.as_ref()).unwrap_or("Something");
      log_message(&mut log, format!("{name} activates a stealth device!"));
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
