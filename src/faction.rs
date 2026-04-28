//! Allegiance for characters and legacy entity descriptions.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Faction {
  Player,
  Friendly,
  Hostile,
  Neutral
}
