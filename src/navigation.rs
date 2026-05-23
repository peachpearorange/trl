use crate::{galaxy::{Galaxy, LocationId},
            ship::Ship};

/// Transit state: ship is traveling between locations.
#[derive(Default)]
pub struct TransitState {
  pub active: bool,
  pub elapsed: f32,
  pub duration: f32,
  pub dest_coords: Option<LocationId>
}

/// Known locations are all locations that have been generated or discovered.
pub fn known_locations(galaxy: &Galaxy) -> Vec<(LocationId, &crate::galaxy::Location)> {
  galaxy
    .locations
    .iter()
    .filter(|(id, _)| **id != (-1, -1, -1)) // exclude ship
    .map(|(id, loc)| (*id, loc))
    .collect()
}

fn fuel_cost(from: Option<LocationId>, to: LocationId) -> u32 {
  let origin = from.unwrap_or((0, 0, 0));
  Galaxy::distance(origin, to).ceil() as u32 * 10
}

/// Initiate a jump to a destination. Consumes fuel and returns the new TransitState.
pub fn initiate_jump(dest: LocationId, ship: &mut Ship) -> TransitState {
  let cost = fuel_cost(ship.docked_at, dest);
  ship.fuel = ship.fuel.saturating_sub(cost);
  TransitState { active: true, elapsed: 0.0, duration: 20.0, dest_coords: Some(dest) }
}

/// Advance transit timer. Returns true when transit has just completed.
pub fn tick_transit(delta_secs: f32, transit: &mut TransitState) -> bool {
  transit.active && {
    transit.elapsed += delta_secs;
    if transit.elapsed >= transit.duration {
      transit.active = false;
      true
    } else {
      false
    }
  }
}
