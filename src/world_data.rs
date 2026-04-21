use trl::entities::Object;

/// A named NPC to be spawned at a specific world position after generation.
pub struct NpcPlacement {
  pub wx: i32,
  pub wy: i32,
  pub z:  usize,
  pub object: fn() -> Object,
}

/// Hand-authored NPCs injected into the world after procgen.
/// Example (commented out until a suitable world position is confirmed):
/// ```
/// NpcPlacement { wx: 247, wy: 243, z: 2, object: Object::catgirl },
/// ```
pub fn world_npcs() -> Vec<NpcPlacement> {
  vec![]
}
