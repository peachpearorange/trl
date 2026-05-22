use bevy::prelude::Color;
use enum_assoc::Assoc;

/// What kind of place a Location is. Determines atmosphere, procgen strategy, and flavor.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum LocationType {
  ShipInterior,
  SpaceStation,
  DerelictShip,
  AsteroidField,
  PlanetSurface { breathable: bool },
  DeepSpace,
  Ruins
}

pub use crate::tiles::Tile;

#[derive(Assoc, Clone, Copy, PartialEq, Eq, Debug, Hash)]
#[func(pub fn name(&self) -> &'static str)]
#[func(pub fn glyph(&self) -> &'static str)]
#[func(pub fn color(&self) -> [f32; 3])]
#[func(pub fn equip_slot(&self) -> Option<EquipSlot> { None })]
#[func(pub fn attack_bonus(&self) -> i32 { 0 })]
#[func(pub fn defense_bonus(&self) -> i32 { 0 })]
#[func(pub fn scrap_yield(&self) -> &'static [(Item, u32)] { &[] })]
#[func(pub fn is_ranged(&self) -> bool { false })]
pub enum Item {
  #[assoc(name = "Gold Coin", glyph = "$", color = [1.0, 0.85, 0.0])]
  GoldCoin,
  #[assoc(name = "Health Potion", glyph = "!", color = [0.9, 0.2, 0.3],
          scrap_yield = &[(Item::Glass, 1), (Item::OrganicMaterial, 2), (Item::Crystal, 1)])]
  HealthPotion,
  #[assoc(name = "Torch", glyph = "/", color = [1.0, 0.6, 0.1],
          scrap_yield = &[(Item::Wood, 1), (Item::OrganicMaterial, 1)])]
  Torch,
  #[assoc(name = "Rock", glyph = "`", color = [0.5, 0.5, 0.5],
          scrap_yield = &[(Item::Crystal, 1)])]
  Rock,
  #[assoc(name = "Mushroom", glyph = "%", color = [0.6, 0.3, 0.7],
          scrap_yield = &[(Item::OrganicMaterial, 2)])]
  Mushroom,
  #[assoc(name = "Wood", glyph = "/", color = [0.55, 0.35, 0.15])]
  Wood,
  #[assoc(name = "Steel", glyph = "]", color = [0.75, 0.78, 0.82])]
  Steel,
  #[assoc(name = "Copper", glyph = "}", color = [0.82, 0.55, 0.35])]
  Copper,
  #[assoc(name = "Screws", glyph = ":", color = [0.9, 0.88, 0.85])]
  Screws,
  #[assoc(name = "Crystal", glyph = "*", color = [0.65, 0.85, 1.0])]
  Crystal,
  #[assoc(name = "Synthetic Material", glyph = ">", color = [0.85, 0.45, 0.75])]
  SyntheticMaterial,
  #[assoc(name = "Glass", glyph = "=", color = [0.75, 0.88, 0.95])]
  Glass,
  #[assoc(name = "Organic Material", glyph = "~", color = [0.45, 0.65, 0.35])]
  OrganicMaterial,
  #[assoc(name = "Iron Sword", glyph = ")", color = [0.82, 0.82, 0.88],
          equip_slot = EquipSlot::Weapon, attack_bonus = 3,
          scrap_yield = &[(Item::Steel, 2), (Item::Wood, 1), (Item::Screws, 1)])]
  IronSword,
  #[assoc(name = "Steel Axe", glyph = "(", color = [0.7, 0.72, 0.76],
          equip_slot = EquipSlot::Weapon, attack_bonus = 4,
          scrap_yield = &[(Item::Steel, 3), (Item::Wood, 2), (Item::Screws, 1)])]
  SteelAxe,
  #[assoc(name = "Copper Knife", glyph = "-", color = [0.85, 0.6, 0.45],
          equip_slot = EquipSlot::Weapon, attack_bonus = 2,
          scrap_yield = &[(Item::Copper, 2), (Item::Screws, 1)])]
  CopperKnife,
  #[assoc(name = "Combat Spear", glyph = "|", color = [0.78, 0.75, 0.65],
          equip_slot = EquipSlot::Weapon, attack_bonus = 3,
          scrap_yield = &[(Item::Wood, 2), (Item::Steel, 1), (Item::Screws, 1)])]
  CombatSpear,
  #[assoc(name = "Pipe Revolver", glyph = "?", color = [0.55, 0.55, 0.58],
          equip_slot = EquipSlot::Weapon, attack_bonus = 5, is_ranged = true,
          scrap_yield = &[(Item::Steel, 2), (Item::Copper, 1), (Item::Screws, 2)])]
  PipeRevolver,
  #[assoc(name = "Leather Vest", glyph = "[", color = [0.55, 0.4, 0.22],
          equip_slot = EquipSlot::Armor, defense_bonus = 1,
          scrap_yield = &[(Item::OrganicMaterial, 3), (Item::Screws, 2)])]
  LeatherVest,
  #[assoc(name = "Chain Mail", glyph = "{", color = [0.72, 0.74, 0.78],
          equip_slot = EquipSlot::Armor, defense_bonus = 2,
          scrap_yield = &[(Item::Steel, 4), (Item::Screws, 3)])]
  ChainMail,
  #[assoc(name = "Steel Boots", glyph = "b", color = [0.68, 0.7, 0.74],
          equip_slot = EquipSlot::Armor, defense_bonus = 1,
          scrap_yield = &[(Item::Steel, 2), (Item::OrganicMaterial, 1), (Item::Screws, 1)])]
  SteelBoots,
  #[assoc(name = "Synth Helmet", glyph = "^", color = [0.55, 0.72, 0.62],
          equip_slot = EquipSlot::Armor, defense_bonus = 1,
          scrap_yield = &[(Item::SyntheticMaterial, 3), (Item::Glass, 1), (Item::Screws, 2)])]
  SynthHelmet,
  #[assoc(name = "Stim Pack", glyph = "+", color = [0.95, 0.35, 0.45],
          scrap_yield = &[(Item::OrganicMaterial, 2), (Item::Crystal, 1), (Item::Glass, 1)])]
  StimPack,
  #[assoc(name = "Canned Goods", glyph = "o", color = [0.85, 0.35, 0.12],
          scrap_yield = &[(Item::Steel, 1), (Item::OrganicMaterial, 2)])]
  CannedGoods,
  #[assoc(name = "Filtered Water", glyph = "u", color = [0.35, 0.65, 0.95],
          scrap_yield = &[(Item::Glass, 2), (Item::OrganicMaterial, 1)])]
  FilterWater,
  #[assoc(name = "Frag Grenade", glyph = "g", color = [0.55, 0.78, 0.35],
          equip_slot = EquipSlot::Grenade,
          scrap_yield = &[(Item::Steel, 1), (Item::Copper, 1), (Item::Screws, 2)])]
  FragGrenade,
  #[assoc(name = "Stun Grenade", glyph = "g", color = [0.35, 0.72, 0.92],
          equip_slot = EquipSlot::Grenade,
          scrap_yield = &[(Item::Crystal, 1), (Item::Copper, 1), (Item::Screws, 2)])]
  StunGrenade,
  #[assoc(name = "Laser Rifle", glyph = "\\", color = [0.0, 0.9, 1.0],
          equip_slot = EquipSlot::Weapon, attack_bonus = 8, is_ranged = true,
          scrap_yield = &[(Item::Crystal, 2), (Item::SyntheticMaterial, 2), (Item::Glass, 1), (Item::Screws, 2)])]
  LaserRifle,
  #[assoc(name = "Plasma Rifle", glyph = "\\", color = [0.2, 1.0, 0.3],
          equip_slot = EquipSlot::Weapon, attack_bonus = 6, is_ranged = true,
          scrap_yield = &[(Item::Crystal, 3), (Item::SyntheticMaterial, 1), (Item::Screws, 2)])]
  PlasmaRifle,
  #[assoc(name = "Scatter Gun", glyph = "?", color = [1.0, 0.4, 0.2],
          equip_slot = EquipSlot::Weapon, attack_bonus = 3, is_ranged = true,
          scrap_yield = &[(Item::Steel, 3), (Item::Copper, 2), (Item::Screws, 3)])]
  ScatterGun,
  #[assoc(name = "Pulse Cannon", glyph = "\\", color = [0.7, 0.2, 1.0],
          equip_slot = EquipSlot::Weapon, attack_bonus = 12, is_ranged = true,
          scrap_yield = &[(Item::Crystal, 2), (Item::SyntheticMaterial, 3), (Item::Glass, 2), (Item::Screws, 2)])]
  PulseCannon,
  #[assoc(name = "Stealth Device", glyph = "d", color = [0.7, 0.2, 1.0],
          equip_slot = EquipSlot::Device,
          scrap_yield = &[(Item::Crystal, 2), (Item::SyntheticMaterial, 2), (Item::Screws, 1)])]
  StealthDevice
}

impl Item {
  pub fn can_salvage(self) -> bool { !self.scrap_yield().is_empty() }
  pub fn is_weapon(self) -> bool { self.equip_slot() == Some(EquipSlot::Weapon) }
  pub fn is_armor(self) -> bool { self.equip_slot() == Some(EquipSlot::Armor) }
  pub fn is_grenade(self) -> bool { self.equip_slot() == Some(EquipSlot::Grenade) }
  pub fn is_device(self) -> bool { self.equip_slot() == Some(EquipSlot::Device) }

  pub fn loot_texture(self) -> &'static str {
    match self {
      Item::StealthDevice => "textures/space_qud/stealth device.png",
      _ if self.is_ranged() => "textures/space_qud/gun.png",
      _ => "textures/space_qud/box with highlight border.png"
    }
  }

  pub fn loot_colors(self) -> (Color, Color) {
    match self {
      Item::StealthDevice => (Color::srgb(0.75, 0.75, 0.78), Color::srgb(0.55, 0.15, 1.0)),
      _ => {
        let [r, g, b] = self.color();
        let primary = Color::srgb(r, g, b);
        let secondary = Color::srgb((r + 0.3).min(1.0), (g + 0.3).min(1.0), (b + 0.3).min(1.0));
        (primary, secondary)
      }
    }
  }

  pub fn is_laser(self) -> bool { matches!(self, Item::LaserRifle) }
  pub fn is_plasma(self) -> bool { matches!(self, Item::PlasmaRifle) }
  pub fn is_scatter(self) -> bool { matches!(self, Item::ScatterGun) }
  pub fn is_pulse(self) -> bool { matches!(self, Item::PulseCannon) }
}

/// The loadout slot an equippable item occupies.
/// Extend this as new gear categories are added (Grenade, Utility, etc.).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum EquipSlot {
  Weapon,
  Armor,
  Grenade,
  Device
}


#[derive(Clone, Debug)]
pub struct Level {
  pub tiles: Vec<Vec<Tile>>,
  pub width: usize,
  pub height: usize
}

impl Level {
  pub fn new(width: usize, height: usize, fill: Tile) -> Self {
    Level {
      tiles: vec![vec![fill; width]; height],
      width,
      height
    }
  }

  pub fn get(&self, x: i32, y: i32) -> Option<Tile> {
    if x < 0 || y < 0 {
      return None;
    }
    let (ux, uy) = (x as usize, y as usize);
    (ux < self.width && uy < self.height).then(|| self.tiles[uy][ux])
  }


  pub fn set(&mut self, x: i32, y: i32, tile: Tile) {
    if x >= 0 && y >= 0 && (x as usize) < self.width && (y as usize) < self.height {
      self.tiles[y as usize][x as usize] = tile;
    }
  }

  pub fn walkable(&self, x: i32, y: i32) -> bool {
    self.get(x, y).is_some_and(|t| t.walkable())
  }
}

// ---------------------------------------------------------------------------
// Builder utilities — usable by hand-crafted levels and procgen alike
// ---------------------------------------------------------------------------

/// Fill a rectangular region with a tile.
pub fn fill_rect(level: &mut Level, x: i32, y: i32, w: usize, h: usize, tile: Tile) {
  for dy in 0..h as i32 {
    for dx in 0..w as i32 {
      level.set(x + dx, y + dy, tile);
    }
  }
}

///// Place a room: wall border with floor interior.
pub fn place_room(level: &mut Level, x: i32, y: i32, w: usize, h: usize, wall: Tile) {
  fill_rect(level, x, y, w, h, wall);
  if w > 2 && h > 2 {
    fill_rect(level, x + 1, y + 1, w - 2, h - 2, Tile::DeckPlate);
  }
}

/// Place a room with a door on a given side at a relative offset along that side.
pub fn place_room_with_door(
  level: &mut Level,
  x: i32,
  y: i32,
  w: usize,
  h: usize,
  door_side: Side,
  door_offset: usize,
  wall: Tile
) {
  place_room(level, x, y, w, h, wall);
  let (dx, dy) = match door_side {
    Side::North => (x + door_offset as i32, y),
    Side::South => (x + door_offset as i32, y + h as i32 - 1),
    Side::West => (x, y + door_offset as i32),
    Side::East => (x + w as i32 - 1, y + door_offset as i32)
  };
  level.set(dx, dy, Tile::DeckPlate);
}

#[derive(Clone, Copy)]
pub enum Side {
  North,
  South,
  East,
  West
}

/// Carve an L-shaped corridor between two points (horizontal first, then vertical).
pub fn place_corridor(level: &mut Level, x1: i32, y1: i32, x2: i32, y2: i32) {
  let (mut cx, cy1, cy2) = (x1, y1, y2);
  let dx = if x2 > x1 { 1 } else { -1 };
  while cx != x2 {
    level.set(cx, cy1, Tile::DeckPlate);
    cx += dx;
  }
  let mut cy = cy1;
  let dy = if cy2 > cy1 { 1 } else { -1 };
  while cy != cy2 {
    level.set(x2, cy, Tile::DeckPlate);
    cy += dy;
  }
  level.set(x2, cy2, Tile::DeckPlate);
}

/// Place a pair of stairs connecting two levels at the same (x, y).
/// Caller is responsible for ensuring both levels exist.
pub fn place_stairs(levels: &mut [Level], z_from: usize, z_to: usize, x: i32, y: i32) {
  if z_to > z_from {
    levels[z_from].set(x, y, Tile::StairsUp);
    levels[z_to].set(x, y, Tile::StairsDown);
  } else {
    levels[z_from].set(x, y, Tile::StairsDown);
    levels[z_to].set(x, y, Tile::StairsUp);
  }
}

/// Carve an organic blob (rough circle) of floor tiles.
pub fn carve_blob(level: &mut Level, cx: i32, cy: i32, radius: i32, tile: Tile) {
  let r2 = radius * radius;
  for dy in -radius..=radius {
    for dx in -radius..=radius {
      let d2 = dx * dx + dy * dy;
      let fudge = ((dx.wrapping_mul(7) ^ dy.wrapping_mul(13)) & 3) as i32;
      if d2 <= r2 + fudge {
        level.set(cx + dx, cy + dy, tile);
      }
    }
  }
}

/// Ensure a square of walkable floor around a point (useful around stairs).
pub fn clear_around(level: &mut Level, x: i32, y: i32, radius: i32) {
  for dy in -radius..=radius {
    for dx in -radius..=radius {
      if level.get(x + dx, y + dy).is_some_and(|t| !t.walkable()) {
        level.set(x + dx, y + dy, Tile::DeckPlate);
      }
    }
  }
}

/// Place a wide corridor (3 tiles across) between two points.
pub fn place_wide_corridor(level: &mut Level, x1: i32, y1: i32, x2: i32, y2: i32) {
  for offset in -1..=1 {
    // horizontal leg
    let (mut cx, cy) = (x1, y1);
    let dx = if x2 > x1 { 1 } else { -1 };
    while cx != x2 {
      level.set(cx, cy + offset, Tile::DeckPlate);
      cx += dx;
    }
    // vertical leg
    let mut cy2 = y1;
    let dy = if y2 > y1 { 1 } else { -1 };
    while cy2 != y2 {
      level.set(x2 + offset, cy2, Tile::DeckPlate);
      cy2 += dy;
    }
    level.set(x2 + offset, y2, Tile::DeckPlate);
  }
}

// ---------------------------------------------------------------------------
// World: a stack of levels
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Visibility: BYOND-style dual-pass shadow propagation
//
// Two passes propagate visibility outward from the viewer:
//   1. Diagonal pass (Chebyshev distance): spreads through 8-neighbors
//   2. Straight pass (Manhattan distance): spreads through 8-neighbors but
//      only to tiles already reached by the diagonal pass
// A tile must be reached by BOTH passes to be visible. Opaque tiles are
// reached (you see the wall face) but don't propagate further.
// A final boundary pass reveals wall faces adjacent to visible areas.
// ---------------------------------------------------------------------------

pub struct FovGrid {
  pub visible: Vec<Vec<bool>>,
  pub width: usize,
  pub height: usize
}

impl FovGrid {
  pub fn new(width: usize, height: usize) -> Self {
    FovGrid {
      visible: vec![vec![false; width]; height],
      width,
      height
    }
  }

  pub fn clear_visible(&mut self) {
    for row in &mut self.visible {
      for cell in row.iter_mut() {
        *cell = false;
      }
    }
  }

  pub fn mark_visible(&mut self, x: usize, y: usize) {
    if x < self.width && y < self.height {
      self.visible[y][x] = true;
    }
  }

  pub fn is_visible(&self, x: usize, y: usize) -> bool {
    x < self.width && y < self.height && self.visible[y][x]
  }
}

/// Compute FOV from (cx, cy) with the given radius on the given level.
pub fn compute_fov(
  fov: &mut FovGrid,
  level: &Level,
  cx: i32,
  cy: i32,
  radius: i32,
  mut blocks_sight: impl FnMut(i32, i32) -> bool
) {
  fov.clear_visible();
  if cx < 0 || cy < 0 || (cx as usize) >= fov.width || (cy as usize) >= fov.height {
    return;
  }

  let max_dist = cx.max((fov.width as i32) - 1 - cx)
    .max(cy.max((fov.height as i32) - 1 - cy));
  let r = radius.min(max_dist);
  let size = (2 * r + 1) as usize;
  // 0 = unvisited, positive = propagation depth, -1 = opaque (visible but blocks)
  let mut vis2 = vec![0i32; size * size];
  let mut vis = vec![0i32; size * size];

  let mut is_opaque = |dx: i32, dy: i32| -> bool {
    if (dx, dy) == (0, 0) { return false; }
    let (wx, wy) = (cx + dx, cy + dy);
    match level.get(wx, wy) {
      None => true,
      Some(t) => t.opaque() || blocks_sight(wx, wy)
    }
  };

  let idx = |dx: i32, dy: i32| (dy + r) as usize * size + (dx + r) as usize;

  fn chebyshev_ring(d: i32, mut f: impl FnMut(i32, i32)) {
    for dx in -d..=d { f(dx, -d); f(dx, d); }
    for dy in (-d + 1)..d { f(-d, dy); f(d, dy); }
  }

  fn manhattan_shell(d: i32, mut f: impl FnMut(i32, i32)) {
    for dx in -d..=d {
      let ady = d - dx.abs();
      f(dx, ady);
      if ady != 0 { f(dx, -ady); }
    }
  }

  const NEIGHBORS: [(i32, i32); 8] = [(-1, -1), (-1, 0), (-1, 1), (0, -1), (0, 1), (1, -1), (1, 0), (1, 1)];

  let in_grid = |dx: i32, dy: i32| dx >= -r && dx <= r && dy >= -r && dy <= r;

  // Pass 1: Diagonal shadow — propagate by Chebyshev distance
  // Only propagate from neighbors at exactly distance d (one ring closer)
  for d in 0..r {
    chebyshev_ring(d + 1, |dx, dy| {
      if !in_grid(dx, dy) { return; }
      let gi = idx(dx, dy);
      let reached = d == 0 || NEIGHBORS.iter().any(|&(ndx, ndy)| {
        let (nx, ny) = (dx + ndx, dy + ndy);
        in_grid(nx, ny) && vis2[idx(nx, ny)] == d
      });
      if reached {
        vis2[gi] = if is_opaque(dx, dy) { -1 } else { d + 1 };
      }
    });
  }

  // Pass 2: Straight shadow — propagate by Manhattan distance
  // Only propagate from neighbors at exactly distance d (one shell closer)
  let sum_max = 2 * r;
  for d in 0..sum_max {
    manhattan_shell(d + 1, |dx, dy| {
      if !in_grid(dx, dy) { return; }
      let gi = idx(dx, dy);
      if vis2[gi] == 0 { return; }
      let reached = d == 0 || NEIGHBORS.iter().any(|&(ndx, ndy)| {
        let (nx, ny) = (dx + ndx, dy + ndy);
        in_grid(nx, ny) && vis[idx(nx, ny)] == d
      });
      if reached {
        vis[gi] = if is_opaque(dx, dy) { -1 } else { d + 1 };
      }
    });
  }

  // Mark eye visible
  vis[idx(0, 0)] = 1;

  // Boundary pass: reveal opaque tiles adjacent to visible areas
  for dx in -r..=r {
    for dy in -r..=r {
      let gi = idx(dx, dy);
      if vis[gi] != 0 || !is_opaque(dx, dy) { continue; }
      let get_vis = |ddx: i32, ddy: i32| {
        let (nx, ny) = (dx + ddx, dy + ddy);
        if nx.abs().max(ny.abs()) <= r { vis[idx(nx, ny)] } else { 0 }
      };
      // Wall rule: both opposite cardinal neighbors visible
      if (get_vis(1, 0) != 0 && get_vis(-1, 0) != 0)
        || (get_vis(0, 1) != 0 && get_vis(0, -1) != 0)
      {
        vis[gi] = -1;
      } else {
        // Corner rule
        for &(cdx, cdy) in &[(-1, -1), (-1, 1), (1, -1), (1, 1)] {
          if get_vis(cdx, cdy) != 0
            && get_vis(cdx, 0) != 0
            && get_vis(0, cdy) != 0
            && is_opaque(dx + cdx, dy)
            && is_opaque(dx, dy + cdy)
            && !is_opaque(dx + cdx, dy + cdy)
          {
            vis[gi] = -1;
            break;
          }
        }
      }
    }
  }

  // Transfer to FovGrid
  for dx in -r..=r {
    for dy in -r..=r {
      if vis[idx(dx, dy)] != 0 {
        fov.mark_visible((cx + dx) as usize, (cy + dy) as usize);
      }
    }
  }
}

#[cfg(test)]
mod fov_tests {
  use super::*;

  fn render_fov(fov: &FovGrid, level: &Level, px: i32, py: i32) -> String {
    let mut s = String::new();
    for y in 0..level.height as i32 {
      for x in 0..level.width as i32 {
        s.push(if (x, y) == (px, py) {
          '@'
        } else if !fov.is_visible(x as usize, y as usize) {
          '?'
        } else if level.get(x, y).is_some_and(|t| t.opaque()) {
          '#'
        } else {
          '.'
        });
      }
      s.push('\n');
    }
    s
  }

  #[test]
  fn bedroom_door_open_sees_interior() {
    // #####
    // #...#
    // #...D.@
    // #...#
    // #####
    let mut level = Level::new(7, 5, Tile::StationFloor);
    for x in 0..5 { level.set(x, 0, Tile::StationWall); level.set(x, 4, Tile::StationWall); }
    for y in 0..5 { level.set(0, y, Tile::StationWall); }
    level.set(4, 0, Tile::StationWall);
    level.set(4, 1, Tile::StationWall);
    level.set(4, 3, Tile::StationWall);
    level.set(4, 4, Tile::StationWall);
    // door at (4,2) is floor (open)

    let (px, py) = (6, 2);
    let mut fov = FovGrid::new(7, 5);
    compute_fov(&mut fov, &level, px, py, 20, |_, _| false);

    let map = render_fov(&fov, &level, px, py);
    println!("door open:\n{map}");
    for y in 1..=3 {
      for x in 1..=3 {
        assert!(fov.is_visible(x, y), "({x},{y}) should be visible with door open\n{map}");
      }
    }
  }

  #[test]
  fn bedroom_door_closed_hides_interior() {
    let mut level = Level::new(7, 5, Tile::StationFloor);
    for x in 0..5 { level.set(x, 0, Tile::StationWall); level.set(x, 4, Tile::StationWall); }
    for y in 0..5 { level.set(0, y, Tile::StationWall); }
    level.set(4, 0, Tile::StationWall);
    level.set(4, 1, Tile::StationWall);
    level.set(4, 3, Tile::StationWall);
    level.set(4, 4, Tile::StationWall);

    let (px, py) = (6, 2);
    let mut fov = FovGrid::new(7, 5);
    // door at (4,2) blocks sight when closed
    compute_fov(&mut fov, &level, px, py, 20, |x, y| x == 4 && y == 2);

    let map = render_fov(&fov, &level, px, py);
    println!("door closed:\n{map}");
    // door itself is visible (wall face)
    assert!(fov.is_visible(4, 2), "door should be visible\n{map}");
    // interior should NOT be visible
    for y in 1..=3 {
      for x in 1..=3 {
        assert!(!fov.is_visible(x, y), "({x},{y}) should be hidden with door closed\n{map}");
      }
    }
  }
}
