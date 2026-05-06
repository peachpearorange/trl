use bevy::prelude::*;

/// Role a crew member serves on the ship.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CrewRole {
    Engineer,
    Medic,
    Gunner,
    Passenger,
}

/// Marks an NPC as a crew member aboard a ship.
#[derive(Component, Debug)]
pub struct CrewMember {
    pub role: CrewRole,
    pub ship_id: Entity,
}
