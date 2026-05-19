use std::collections::HashMap;

use crate::level::Item;

pub struct Recipe {
  pub output: Item,
  pub output_qty: u32,
  pub ingredients: &'static [(Item, u32)]
}

pub static RECIPES: &[Recipe] = &[
  // Melee weapons
  Recipe {
    output: Item::CopperKnife,
    output_qty: 1,
    ingredients: &[(Item::Copper, 2), (Item::Screws, 1)]
  },
  Recipe {
    output: Item::IronSword,
    output_qty: 1,
    ingredients: &[(Item::Steel, 2), (Item::Wood, 1), (Item::Screws, 1)]
  },
  Recipe {
    output: Item::CombatSpear,
    output_qty: 1,
    ingredients: &[(Item::Wood, 2), (Item::Steel, 1), (Item::Screws, 1)]
  },
  Recipe {
    output: Item::SteelAxe,
    output_qty: 1,
    ingredients: &[(Item::Steel, 3), (Item::Wood, 2), (Item::Screws, 1)]
  },
  // Ranged weapons
  Recipe {
    output: Item::PipeRevolver,
    output_qty: 1,
    ingredients: &[(Item::Steel, 2), (Item::Copper, 1), (Item::Screws, 2)]
  },
  Recipe {
    output: Item::LaserRifle,
    output_qty: 1,
    ingredients: &[(Item::Crystal, 2), (Item::SyntheticMaterial, 2), (Item::Glass, 1), (Item::Screws, 2)]
  },
  // Armor
  Recipe {
    output: Item::LeatherVest,
    output_qty: 1,
    ingredients: &[(Item::OrganicMaterial, 3), (Item::Screws, 2)]
  },
  Recipe {
    output: Item::ChainMail,
    output_qty: 1,
    ingredients: &[(Item::Steel, 4), (Item::Screws, 3)]
  },
  Recipe {
    output: Item::SteelBoots,
    output_qty: 1,
    ingredients: &[(Item::Steel, 2), (Item::OrganicMaterial, 1), (Item::Screws, 1)]
  },
  Recipe {
    output: Item::SynthHelmet,
    output_qty: 1,
    ingredients: &[(Item::SyntheticMaterial, 3), (Item::Glass, 1), (Item::Screws, 2)]
  },
  // Grenades
  Recipe {
    output: Item::FragGrenade,
    output_qty: 1,
    ingredients: &[(Item::Steel, 1), (Item::Copper, 1), (Item::Screws, 2)]
  },
  Recipe {
    output: Item::StunGrenade,
    output_qty: 1,
    ingredients: &[(Item::Crystal, 1), (Item::Copper, 1), (Item::Screws, 2)]
  },
  // Consumables
  Recipe {
    output: Item::HealthPotion,
    output_qty: 1,
    ingredients: &[(Item::Glass, 1), (Item::OrganicMaterial, 2), (Item::Crystal, 1)]
  },
  Recipe {
    output: Item::StimPack,
    output_qty: 1,
    ingredients: &[(Item::OrganicMaterial, 2), (Item::Crystal, 1), (Item::Glass, 1)]
  },
  Recipe {
    output: Item::CannedGoods,
    output_qty: 1,
    ingredients: &[(Item::Steel, 1), (Item::OrganicMaterial, 2)]
  },
  Recipe {
    output: Item::FilterWater,
    output_qty: 1,
    ingredients: &[(Item::Glass, 2), (Item::OrganicMaterial, 1)]
  },
  // Misc
  Recipe {
    output: Item::Torch,
    output_qty: 1,
    ingredients: &[(Item::Wood, 1), (Item::OrganicMaterial, 1)]
  },
];

pub fn can_craft(inv: &HashMap<Item, u32>, recipe: &Recipe) -> bool {
  recipe.ingredients.iter().all(|&(it, need)| inv.get(&it).copied().unwrap_or(0) >= need)
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
