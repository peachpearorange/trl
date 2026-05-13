//! Player ability bar: Fire Gun, Throw Grenade, etc.
//! Each sim turn, cooldowns decrement. Number keys select an ability; left-click fires it.

use {std::collections::HashMap,
     bevy::prelude::*,
     crate::{Clock, CurrentZone, Inventory, Player, PlayerPos, TurnBasedWorldState, UiState,
             entities::{Enemy, Location, Named, Object, PlayerEquipped, Stats},
             level::Item,
             particles::{ParticleEffects, spawn_bullet_trail, spawn_explosion_burst},
             path_overlay::ray_cast_target,
             ui::{LogEntries, log_message}}};

const EXPLOSION_OFFSETS: [(i32, i32); 13] = [
  (0, 0),
  (-1, 0), (1, 0), (0, -1), (0, 1),
  (-2, 0), (2, 0), (0, -2), (0, 2),
  (-1, -1), (1, -1), (-1, 1), (1, 1)
];

/// What each ability slot does.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum AbilityKind {
  FireGun,
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
  pub cooldowns: HashMap<AbilityKind, u32>
}

// ---------------------------------------------------------------------------
// Systems
// ---------------------------------------------------------------------------

/// Rebuild the ability bar from equipped items each frame, preserving existing cooldowns.
/// Only writes to [`AbilityBarData`] when the displayed data actually changes.
pub fn sync_ability_bar(
  player_q: Query<&PlayerEquipped, With<Player>>,
  targeting: Res<TargetingState>,
  mut bar: ResMut<AbilityBarData>
) {
  if let Ok(equipped) = player_q.single() {
    let mut new_slots = Vec::new();

    if equipped.weapon.is_some_and(|w| w.is_ranged()) {
      let cd = targeting.cooldowns.get(&AbilityKind::FireGun).copied().unwrap_or(0);
      new_slots.push(AbilitySlot {
        kind: AbilityKind::FireGun,
        name: "Fire Gun".into(),
        cooldown: cd,
        max_cooldown: 3
      });
    }

    for slot in 0..3usize {
      if let Some(item) = equipped.grenades[slot] {
        let kind = AbilityKind::ThrowGrenade { slot, item };
        let cd = targeting.cooldowns.get(&kind).copied().unwrap_or(0);
        new_slots.push(AbilitySlot {
          kind,
          name: format!("Throw {}", item.name()),
          cooldown: cd,
          max_cooldown: 5
        });
      }
    }

    let new_selected = targeting.selected;
    if bar.slots != new_slots || bar.selected != new_selected {
      bar.slots = new_slots;
      bar.selected = new_selected;
    }
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

/// When targeting and the player left-clicks, fire the selected ability at that tile.
pub fn handle_ability_click(
  mouse: Res<ButtonInput<MouseButton>>,
  windows: Query<&Window>,
  camera_q: Query<(&Camera, &GlobalTransform), With<Camera2d>>,
  current: Res<CurrentZone>,
  mut targeting: ResMut<TargetingState>,
  bar: Res<AbilityBarData>,
  mut player_q: Query<(&PlayerPos, &mut Inventory, &mut PlayerEquipped), With<Player>>,
  mut enemy_q: Query<(&Location, &mut Stats, Option<&Named>), With<Enemy>>,
  mut commands: Commands,
  mut log: ResMut<LogEntries>,
  mut clock: ResMut<Clock>,
  mut tb: ResMut<TurnBasedWorldState>,
  effects: Res<ParticleEffects>
) {
  if let Some(slot_idx) = targeting.selected
    && mouse.just_pressed(MouseButton::Left)
    && let Ok(window) = windows.single()
    && let Ok((camera, cam_transform)) = camera_q.single()
    && let Ok((pos, mut inventory, mut equipped)) = player_q.single_mut()
  {
    if bar.slots.get(slot_idx).is_none() {
      targeting.selected = None;
    } else {
      let slot = &bar.slots[slot_idx];
      let cd = targeting.cooldowns.get(&slot.kind).copied().unwrap_or(0);
      if cd > 0 {
        log_message(&mut log, format!("{} is on cooldown ({} turns).", slot.name, cd));
        targeting.selected = None;
      } else if let Some(cursor) = window.cursor_position()
        && let Ok(world) = camera.viewport_to_world_2d(cam_transform, cursor)
      {
        let level = current.0.level(pos.z);
        let (cursor_tx, cursor_ty) = crate::world_to_level_cell(world, level.width, level.height);
        let (tx, ty) = ray_cast_target(pos.x, pos.y, cursor_tx, cursor_ty, level);
        let max_cd = slot.max_cooldown;

        // Returns true if the ability was actually fired (spend turn + set cooldown).
        let fired = match &slot.kind {
          AbilityKind::FireGun => {
            use crate::path_overlay::bresenham_path;
            let path = bresenham_path(pos.x, pos.y, tx, ty);
            let hit_pos = path.iter().skip(1).find(|&&(px, py)| {
              enemy_q.iter().any(|(loc, _, _)| {
                matches!(loc, Location::Coords { x, y, z, .. } if *x == px && *y == py && *z == pos.z)
              })
            }).copied();
            // Trail ends at the hit enemy tile or the ray endpoint.
            let trail_end = hit_pos.unwrap_or((tx, ty));
            let trail_path = bresenham_path(pos.x, pos.y, trail_end.0, trail_end.1);
            spawn_bullet_trail(&mut commands, &effects, &trail_path, level.width, level.height);
            if let Some((hx, hy)) = hit_pos
              && let Some((_, mut stats, named)) = enemy_q.iter_mut().find(|(loc, _, _)| {
                matches!(loc, Location::Coords { x, y, z, .. } if *x == hx && *y == hy && *z == pos.z)
              })
            {
              let attack = equipped.weapon.map(|w| w.attack_bonus()).unwrap_or(0) + 5;
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
          &AbilityKind::ThrowGrenade { slot: grenade_slot, item } => {
            if inventory.0.get(&item).copied().unwrap_or(0) == 0 {
              log_message(&mut log, format!("No {} in inventory.", item.name()));
              false
            } else {
              let entry = inventory.0.entry(item).or_insert(0);
              *entry = entry.saturating_sub(1);
              if *entry == 0 {
                inventory.0.remove(&item);
                equipped.grenades[grenade_slot] = None;
              }
              for &(dx, dy) in &EXPLOSION_OFFSETS {
                let (ex, ey) = (tx + dx, ty + dy);
                if level.walkable(ex, ey) {
                  Object::explosion_cloud().spawn_at(&mut commands, ex, ey, pos.z);
                }
              }
              spawn_explosion_burst(&mut commands, &effects, (tx, ty), level.width, level.height);
              log_message(&mut log, format!("You throw a {}!", item.name()));
              true
            }
          }
        };

        if fired {
          targeting.cooldowns.insert(slot.kind.clone(), max_cd);
          clock.spend_turn(&mut tb);
        }
        if !fired {
          targeting.selected = None;
        }
      }
    }
  }
}

/// Each sim step, decrement cooldowns. Only touches [`TargetingState`], never [`AbilityBarData`].
pub fn tick_cooldowns(mut targeting: ResMut<TargetingState>) {
  targeting.cooldowns.retain(|_, cd| {
    *cd = cd.saturating_sub(1);
    *cd > 0
  });
}
