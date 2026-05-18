//! Player ability bar: Fire Gun, Throw Grenade, etc.
//! Each sim turn, cooldowns decrement. Number keys select an ability; left-click fires it.

use {std::collections::HashMap,
     bevy::prelude::*,
     crate::{Clock, CurrentZone, Inventory, Player, PlayerPos, TimeMode, TurnBasedWorldState, UiState,
             entities::{Enemy, Glyph, GrenadeInFlight, Loadout, Location, Named, Stats},
             level::Item,
             particles::{ParticleEffects, spawn_bullet_trail, spawn_laser_beam, tile_to_world},
             path_overlay::{bresenham_path, dda_cells, euclidean_los_point, ray_cast_target},
             ui::{LogEntries, log_message}}};

/// Grenade flight speed: tiles traversed per sim turn before detonation.
const GRENADE_TILES_PER_TURN: usize = 4;

/// What each ability slot does.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum AbilityKind {
  FireGun,
  FireLaser,
  ThrowGrenade { slot: usize, item: Item }
}

/// One slot in the player's ability bar.
#[derive(Clone, Debug, PartialEq)]
pub struct AbilitySlot {
  pub kind: AbilityKind,
  pub name: String,
  pub cooldown: u32,
  pub max_cooldown: u32
}

/// The player's current ability bar state plus which slot is selected for targeting.
/// Written by [`sync_ability_bar`], read by the UI via `from_resource_changed`.
#[derive(Resource, Clone, Default)]
pub struct AbilityBarData {
  pub slots: Vec<AbilitySlot>,
  pub selected: Option<usize>
}

/// Internal targeting + cooldown state — not observed by UI signals.
/// Cooldowns live here so [`tick_cooldowns`] never touches [`AbilityBarData`].
#[derive(Resource, Default)]
pub struct TargetingState {
  pub selected: Option<usize>,
  /// Remaining cooldown turns, keyed by ability kind.
  pub cooldowns: HashMap<AbilityKind, u32>,
  /// Queued fire: fires automatically once the cooldown expires.
  /// Stores the ability kind and the cursor tile the player aimed at.
  pub pending_fire: Option<(AbilityKind, (i32, i32))>
}

// ---------------------------------------------------------------------------
// Systems
// ---------------------------------------------------------------------------

/// Rebuild the ability bar from equipped items each frame, preserving existing cooldowns.
/// Only writes to [`AbilityBarData`] when the displayed data actually changes.
pub fn sync_ability_bar(
  loadout: Single<&Loadout, With<Player>>,
  targeting: Res<TargetingState>,
  mut bar: ResMut<AbilityBarData>
) {
  let mut new_slots = Vec::new();

  if let Some(weapon) = loadout.weapon()
    && weapon.is_ranged()
  {
    let (kind, name, max_cd) = if weapon.is_laser() {
      (AbilityKind::FireLaser, "Fire Laser", 4)
    } else {
      (AbilityKind::FireGun, "Fire Gun", 3)
    };
    let cd = targeting.cooldowns.get(&kind).copied().unwrap_or(0);
    new_slots.push(AbilitySlot { kind, name: name.into(), cooldown: cd, max_cooldown: max_cd });
  }

  for (slot, item) in loadout.grenade_slots() {
    let kind = AbilityKind::ThrowGrenade { slot, item };
    let cd = targeting.cooldowns.get(&kind).copied().unwrap_or(0);
    new_slots.push(AbilitySlot {
      kind,
      name: format!("Throw {}", item.name()),
      cooldown: cd,
      max_cooldown: 5
    });
  }

  let new_selected = targeting.selected;
  if bar.slots != new_slots || bar.selected != new_selected {
    bar.slots = new_slots;
    bar.selected = new_selected;
  }
}

/// Number keys 1-9 select an ability slot (or toggle it off if already selected).
pub fn handle_ability_keys(
  keys: Res<ButtonInput<KeyCode>>,
  ui: Res<UiState>,
  bar: Res<AbilityBarData>,
  mut targeting: ResMut<TargetingState>
) {
  if !ui.any_open() {
    let pressed_idx = [
      KeyCode::Digit1, KeyCode::Digit2, KeyCode::Digit3,
      KeyCode::Digit4, KeyCode::Digit5, KeyCode::Digit6,
      KeyCode::Digit7, KeyCode::Digit8, KeyCode::Digit9
    ].iter().position(|k| keys.just_pressed(*k));

    if let Some(idx) = pressed_idx
      && idx < bar.slots.len()
    {
      targeting.selected = (targeting.selected != Some(idx)).then_some(idx);
    }
  }
}

/// Fire the selected ability, or auto-fire a pending ability once its cooldown expires.
/// If the player clicks while on cooldown the shot is queued and fires automatically.
pub fn handle_ability_click(
  mouse: Res<ButtonInput<MouseButton>>,
  windows: Query<&Window>,
  camera_q: Query<(&Camera, &GlobalTransform), With<crate::post_process::GameCamera>>,
  current: Res<CurrentZone>,
  mut targeting: ResMut<TargetingState>,
  bar: Res<AbilityBarData>,
  player: Single<(&PlayerPos, &mut Inventory, &mut Loadout), With<Player>>,
  mut enemy_q: Query<(&Location, &mut Stats, Option<&Named>), With<Enemy>>,
  mut commands: Commands,
  mut log: ResMut<LogEntries>,
  mut clock: ResMut<Clock>,
  mut tb: ResMut<TurnBasedWorldState>,
  effects: Res<ParticleEffects>
) {
  let (pos, ref mut inventory, ref mut loadout) = player.into_inner();

  // Determine what to fire this frame — either a queued shot whose cooldown just hit 0,
  // or a fresh click. Produces (kind, cursor_tx, cursor_ty, max_cd) or None.
  let fire_now: Option<(AbilityKind, i32, i32, u32)> =
    if let Some((ref kind, (ptx, pty))) = targeting.pending_fire.clone()
      && targeting.cooldowns.get(kind).copied().unwrap_or(0) == 0
    {
      targeting.pending_fire = None;
      let max_cd = bar.slots.iter()
        .find(|s| s.kind == *kind)
        .map(|s| s.max_cooldown)
        .unwrap_or(3);
      Some((kind.clone(), ptx, pty, max_cd))
    } else if let Some(slot_idx) = targeting.selected
      && mouse.just_pressed(MouseButton::Left)
      && let Ok(window) = windows.single()
      && let Ok((camera, cam_transform)) = camera_q.single()
      && let Some(cursor) = window.cursor_position()
      && let Ok(world) = camera.viewport_to_world_2d(cam_transform, cursor)
    {
      if bar.slots.get(slot_idx).is_none() {
        targeting.selected = None;
        None
      } else {
        let slot = &bar.slots[slot_idx];
        let level = current.0.level(pos.z);
        let (cursor_tx, cursor_ty) = crate::world_to_level_cell(world, level.width, level.height);
        let cd = targeting.cooldowns.get(&slot.kind).copied().unwrap_or(0);
        if cd > 0 {
          targeting.pending_fire = Some((slot.kind.clone(), (cursor_tx, cursor_ty)));
          clock.spend_turn(&mut tb);
          None
        } else {
          Some((slot.kind.clone(), cursor_tx, cursor_ty, slot.max_cooldown))
        }
      }
    } else {
      None
    };

  let Some((kind, cursor_tx, cursor_ty, max_cd)) = fire_now else { return };
  let level = current.0.level(pos.z);
  let px = pos.x as f32 + 0.5;
  let py = pos.y as f32 + 0.5;

  // Laser and grenades both require Euclidean LoS to the target tile.
  let needs_los = matches!(kind, AbilityKind::FireLaser | AbilityKind::ThrowGrenade { .. });
  let los_point = needs_los.then(|| euclidean_los_point(px, py, cursor_tx, cursor_ty, level)).flatten();

  if needs_los && los_point.is_none() {
    log_message(&mut log, "No LoS to target.".into());
    return; // keep ability selected
  }

  let (tx, ty) = ray_cast_target(pos.x, pos.y, cursor_tx, cursor_ty, level);

  let fired = match &kind {
    AbilityKind::FireLaser => {
      let (los_x, los_y) = los_point.unwrap();
      let cells = dda_cells(px, py, los_x, los_y);
      let beam_start = tile_to_world(px, py, level.width, level.height);
      let beam_end = tile_to_world(los_x, los_y, level.width, level.height);
      spawn_laser_beam(&mut commands, &effects, beam_start, beam_end);
      let attack = loadout.weapon_attack_bonus() + 5;
      let mut hit_names: Vec<&str> = vec![];
      for &(cx, cy) in cells.iter().skip(1) {
        if let Some((_, mut stats, named)) = enemy_q.iter_mut().find(|(loc, _, _)| {
          matches!(loc, Location::Coords { x, y, z, .. } if *x == cx && *y == cy && *z == pos.z)
        }) {
          stats.hp -= attack;
          hit_names.push(named.map(|n| n.name).unwrap_or("Enemy"));
        }
      }
      log_message(&mut log, match hit_names.len() {
        0 => "Your laser hits nothing.".into(),
        1 => format!("Laser burns {} for {} damage!", hit_names[0], attack),
        n => format!("Laser burns {} enemies for {} damage each!", n, attack)
      });
      true
    }
    AbilityKind::FireGun => {
      let path = bresenham_path(pos.x, pos.y, tx, ty);
      let hit_pos = path.iter().skip(1).find(|&&(px, py)| {
        enemy_q.iter().any(|(loc, _, _)| {
          matches!(loc, Location::Coords { x, y, z, .. } if *x == px && *y == py && *z == pos.z)
        })
      }).copied();
      let trail_end = hit_pos.unwrap_or((tx, ty));
      let trail_path = bresenham_path(pos.x, pos.y, trail_end.0, trail_end.1);
      spawn_bullet_trail(&mut commands, &effects, &trail_path, level.width, level.height);
      if let Some((hx, hy)) = hit_pos
        && let Some((_, mut stats, named)) = enemy_q.iter_mut().find(|(loc, _, _)| {
          matches!(loc, Location::Coords { x, y, z, .. } if *x == hx && *y == hy && *z == pos.z)
        })
      {
        let attack = loadout.weapon_attack_bonus() + 5;
        stats.hp -= attack;
        let name = named.map(|n| n.name).unwrap_or("Enemy");
        log_message(&mut log, format!("You shoot {} for {} damage!", name, attack));
      } else {
        log_message(&mut log, if (tx, ty) != (cursor_tx, cursor_ty) {
          "Your shot hit the wall."
        } else {
          "Your shot hits nothing."
        }.into());
      }
      true
    }
    AbilityKind::ThrowGrenade { slot: grenade_slot, item } => {
      let (_grenade_slot, item) = (*grenade_slot, *item);
      if inventory.0.get(&item).copied().unwrap_or(0) == 0 {
        log_message(&mut log, format!("No {} in inventory.", item.name()));
        false
      } else {
        let entry = inventory.0.entry(item).or_insert(0);
        *entry = entry.saturating_sub(1);
        if *entry == 0 {
          inventory.0.remove(&item);
          loadout.remove_grenade_by_item(item);
        }
        let path = bresenham_path(pos.x, pos.y, tx, ty);
        commands.spawn((
          Glyph::palette_sprite(
            "textures/space_qud/grenade.png",
            'o',
            Color::srgb(0.85, 0.50, 0.10),
            Color::srgb(0.30, 0.20, 0.10)
          ),
          Location::xyz(pos.x, pos.y, pos.z),
          GrenadeInFlight {
            path,
            step: 0,
            tiles_per_turn: GRENADE_TILES_PER_TURN,
            z: pos.z
          }
        ));
        log_message(&mut log, format!("You hurl a {}!", item.name()));
        true
      }
    }
  };

  if fired {
    targeting.cooldowns.insert(kind, max_cd);
    clock.spend_turn(&mut tb);
  }
  if !fired {
    targeting.selected = None;
  }
}

/// In turn-based mode, keep spending turns while a pending fire is waiting on cooldown,
/// so the world keeps ticking without player input.
pub fn advance_pending_fire(
  targeting: Res<TargetingState>,
  mut clock: ResMut<Clock>,
  mut tb: ResMut<TurnBasedWorldState>
) {
  if clock.mode == TimeMode::TurnBased
    && targeting.pending_fire.as_ref()
      .is_some_and(|(kind, _)| targeting.cooldowns.get(kind).copied().unwrap_or(0) > 0)
  {
    clock.spend_turn(&mut tb);
  }
}

/// Each sim step, decrement cooldowns. Only touches [`TargetingState`], never [`AbilityBarData`].
pub fn tick_cooldowns(mut targeting: ResMut<TargetingState>) {
  targeting.cooldowns.retain(|_, cd| {
    *cd = cd.saturating_sub(1);
    *cd > 0
  });
}
