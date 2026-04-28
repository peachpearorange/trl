//! Craft-from-components recipes (station-free for now).

use std::collections::HashMap;

use crate::level::Item;

pub struct Recipe {
  pub output:      Item,
  pub output_qty:  u32,
  pub ingredients: &'static [(Item, u32)],
}

pub static RECIPES: &[Recipe] = &[
  Recipe {
    output: Item::IronSword,
    output_qty: 1,
    ingredients: &[(Item::Steel, 2), (Item::Wood, 1), (Item::Screws, 1)],
  },
  Recipe {
    output: Item::CombatSpear,
    output_qty: 1,
    ingredients: &[(Item::Wood, 2), (Item::Steel, 1), (Item::Screws, 1)],
  },
  Recipe {
    output: Item::StimPack,
    output_qty: 1,
    ingredients: &[(Item::OrganicMaterial, 2), (Item::Crystal, 1), (Item::Glass, 1)],
  },
  Recipe {
    output: Item::Torch,
    output_qty: 1,
    ingredients: &[(Item::Wood, 1), (Item::OrganicMaterial, 1)],
  },
  Recipe {
    output: Item::SteelBoots,
    output_qty: 1,
    ingredients: &[(Item::Steel, 2), (Item::OrganicMaterial, 1), (Item::Screws, 2)],
  },
];

pub fn can_craft(inv: &HashMap<Item, u32>, recipe: &Recipe) -> bool {
  recipe
    .ingredients
    .iter()
    .all(|&(it, need)| inv.get(&it).copied().unwrap_or(0) >= need)
}

pub fn apply_craft(inv: &mut HashMap<Item, u32>, recipe: &Recipe) {
  for &(it, need) in recipe.ingredients {
    let e = inv.entry(it).or_insert(0);
    *e = e.saturating_sub(need);
    if *e == 0 {
      inv.remove(&it);
    }
  }
  *inv.entry(recipe.output).or_insert(0) += recipe.output_qty;
}
