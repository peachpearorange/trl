use crate::{active_zone::ActiveZone,
            galaxy::{Galaxy, LocationId},
            ship::Ship};

/// Dock the ship at the destination location.
/// Returns the new merged ActiveZone, or None if docking failed.
pub fn dock(
  galaxy: &mut Galaxy,
  ship: &mut Ship,
  dest_id: LocationId
) -> Option<ActiveZone> {
  galaxy.get_or_generate(dest_id);
  let ship_loc = galaxy.get(ship.location_id)?;
  let dest = galaxy.get(dest_id)?;
  let merged = ActiveZone::docked(ship_loc, dest)?;
  ship.docked_at = Some(dest_id);
  Some(merged)
}

/// Undock the ship. Returns the ship-only ActiveZone.
pub fn undock(galaxy: &Galaxy, ship: &mut Ship) -> ActiveZone {
  ship.docked_at = None;
  ActiveZone::ship_only(galaxy.get(ship.location_id).expect("ship must exist in galaxy"))
}
