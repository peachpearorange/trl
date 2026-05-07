//! Procedural chest contents: value scales with cave depth and a small surface random factor.

use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;

use crate::level::Item;

const WEAPONS: &[Item] = &[
  Item::IronSword,
  Item::SteelAxe,
  Item::CopperKnife,
  Item::CombatSpear,
  Item::PipeRevolver,
];
const ARMOR: &[Item] = &[
  Item::LeatherVest,
  Item::ChainMail,
  Item::SteelBoots,
  Item::SynthHelmet,
];
const CONSUMABLES: &[Item] = &[
  Item::HealthPotion,
  Item::StimPack,
  Item::CannedGoods,
  Item::FilterWater,
];
const COMPONENTS: &[Item] = &[
  Item::Wood,
  Item::Steel,
  Item::Copper,
  Item::Screws,
  Item::Crystal,
  Item::SyntheticMaterial,
  Item::Glass,
  Item::OrganicMaterial,
];

/// Loot rolled when opening a chest at world tile `(wx, wy, z)`.
pub fn roll_chest_loot(world_seed: u64, wx: i32, wy: i32, z: usize) -> Vec<(Item, u32)> {
  let mut rng = StdRng::seed_from_u64(
    world_seed
      ^ (wx as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)
      ^ (wy as u64).rotate_left(19)
      ^ (z as u64).wrapping_mul(0xC2B2_AE3D_21D0_4E21),
  );
  let depth = 0u32;
  let tier: f32 = 1.0 + rng.random_range(0.15..0.65);
  let slot_bonus = ((tier - 1.0_f32).floor() as usize).clamp(0, 4);
  let rolls = 2 + slot_bonus + rng.random_range(0..=depth.min(3) as usize);
  let mut out: Vec<(Item, u32)> = Vec::new();
  for _ in 0..rolls {
    let pick = rng.random::<f32>() * tier;
    let pool = if pick < 0.21 {
      WEAPONS
    } else if pick < 0.40 {
      ARMOR
    } else if pick < 0.60 {
      CONSUMABLES
    } else {
      COMPONENTS
    };
    let idx = rng.random_range(0..pool.len());
    let item = pool[idx];
    let qty = stack_qty(&mut rng, tier, item);
    merge(&mut out, item, qty);
  }
  let gold_p = 0.40_f64;
  if rng.random_bool(gold_p) {
    merge(
      &mut out,
      Item::GoldCoin,
      rng.random_range(2..=15u32.saturating_add(depth.saturating_mul(6))),
    );
  }
  out
}

fn stack_qty(rng: &mut impl Rng, tier: f32, item: Item) -> u32 {
  let components = matches!(
    item,
    Item::Wood
      | Item::Steel
      | Item::Copper
      | Item::Screws
      | Item::Crystal
      | Item::SyntheticMaterial
      | Item::Glass
      | Item::OrganicMaterial
  );
  if !components {
    return 1;
  }
  let base = rng.random_range(2..=7);
  let bonus = (tier * 0.5).floor() as u32;
  base + bonus
}

fn merge(out: &mut Vec<(Item, u32)>, item: Item, qty: u32) {
  if let Some((_, q)) = out.iter_mut().find(|(i, _)| *i == item) {
    *q += qty;
  } else {
    out.push((item, qty));
  }
}
