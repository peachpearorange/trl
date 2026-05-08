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

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tile {
  Air,
  Floor,
  Wall,
  CobblestoneWall,
  BrickWall,
  Grass,
  Water,
  Sand,
  StairsUp,
  StairsDown,
  Door,
  TallGrass,
  Bush,
  Ash,
  Lava,
  ShallowWater,
  DeepWater,
  Road,
  WoodWall,
  WoodFloor,
  Fence,
  CaveWall,
  CaveFloor,
  CrystalFormation,
  // --- Space tiles ---
  DeckPlate,
  Bulkhead,
  Window,
  AirlockDoor,
  StationFloor,
  StationWall,
  DerelictFloor,
  DerelictWall,
  Conduit,
  AsteroidRock,
  AsteroidFloor,
  Regolith,
  Vacuum,
  IceFloor,
  IceWall,
  AlienSoil,
  AlienGrass,
  CrystalGrowth,
  AlienFluid
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Item {
  GoldCoin,
  HealthPotion,
  Torch,
  Rock,
  Mushroom,
  /// Base scrap / crafting: wood (existing chop + recipes).
  Wood,
  Steel,
  Copper,
  Screws,
  Crystal,
  SyntheticMaterial,
  Glass,
  OrganicMaterial,
  IronSword,
  SteelAxe,
  CopperKnife,
  CombatSpear,
  PipeRevolver,
  LeatherVest,
  ChainMail,
  SteelBoots,
  SynthHelmet,
  StimPack,
  CannedGoods,
  FilterWater
}

impl Item {
  pub fn name(self) -> &'static str {
    match self {
      Item::GoldCoin => "Gold Coin",
      Item::HealthPotion => "Health Potion",
      Item::Torch => "Torch",
      Item::Rock => "Rock",
      Item::Mushroom => "Mushroom",
      Item::Wood => "Wood",
      Item::Steel => "Steel",
      Item::Copper => "Copper",
      Item::Screws => "Screws",
      Item::Crystal => "Crystal",
      Item::SyntheticMaterial => "Synthetic Material",
      Item::Glass => "Glass",
      Item::OrganicMaterial => "Organic Material",
      Item::IronSword => "Iron Sword",
      Item::SteelAxe => "Steel Axe",
      Item::CopperKnife => "Copper Knife",
      Item::CombatSpear => "Combat Spear",
      Item::PipeRevolver => "Pipe Revolver",
      Item::LeatherVest => "Leather Vest",
      Item::ChainMail => "Chain Mail",
      Item::SteelBoots => "Steel Boots",
      Item::SynthHelmet => "Synth Helmet",
      Item::StimPack => "Stim Pack",
      Item::CannedGoods => "Canned Goods",
      Item::FilterWater => "Filtered Water"
    }
  }

  pub fn glyph(self) -> &'static str {
    match self {
      Item::GoldCoin => "$",
      Item::HealthPotion => "!",
      Item::Torch => "/",
      Item::Rock => "`",
      Item::Mushroom => "%",
      Item::Wood => "/",
      Item::Steel => "]",
      Item::Copper => "}",
      Item::Screws => ":",
      Item::Crystal => "*",
      Item::SyntheticMaterial => ">",
      Item::Glass => "=",
      Item::OrganicMaterial => "~",
      Item::IronSword => ")",
      Item::SteelAxe => "(",
      Item::CopperKnife => "-",
      Item::CombatSpear => "|",
      Item::PipeRevolver => "?",
      Item::LeatherVest => "[",
      Item::ChainMail => "{",
      Item::SteelBoots => "b",
      Item::SynthHelmet => "^",
      Item::StimPack => "+",
      Item::CannedGoods => "o",
      Item::FilterWater => "u"
    }
  }

  pub fn color(self) -> [f32; 3] {
    match self {
      Item::GoldCoin => [1.0, 0.85, 0.0],
      Item::HealthPotion => [0.9, 0.2, 0.3],
      Item::Torch => [1.0, 0.6, 0.1],
      Item::Rock => [0.5, 0.5, 0.5],
      Item::Mushroom => [0.6, 0.3, 0.7],
      Item::Wood => [0.55, 0.35, 0.15],
      Item::Steel => [0.75, 0.78, 0.82],
      Item::Copper => [0.82, 0.55, 0.35],
      Item::Screws => [0.9, 0.88, 0.85],
      Item::Crystal => [0.65, 0.85, 1.0],
      Item::SyntheticMaterial => [0.85, 0.45, 0.75],
      Item::Glass => [0.75, 0.88, 0.95],
      Item::OrganicMaterial => [0.45, 0.65, 0.35],
      Item::IronSword => [0.82, 0.82, 0.88],
      Item::SteelAxe => [0.7, 0.72, 0.76],
      Item::CopperKnife => [0.85, 0.6, 0.45],
      Item::CombatSpear => [0.78, 0.75, 0.65],
      Item::PipeRevolver => [0.55, 0.55, 0.58],
      Item::LeatherVest => [0.55, 0.4, 0.22],
      Item::ChainMail => [0.72, 0.74, 0.78],
      Item::SteelBoots => [0.68, 0.7, 0.74],
      Item::SynthHelmet => [0.55, 0.72, 0.62],
      Item::StimPack => [0.95, 0.35, 0.45],
      Item::CannedGoods => [0.85, 0.35, 0.12],
      Item::FilterWater => [0.35, 0.65, 0.95]
    }
  }

  /// Fallout-style breakdown: gear and some junk salvage into base components.
  pub fn scrap_yield(self) -> &'static [(Item, u32)] {
    match self {
      Item::IronSword => &[(Item::Steel, 2), (Item::Wood, 1), (Item::Screws, 1)],
      Item::SteelAxe => &[(Item::Steel, 3), (Item::Wood, 2), (Item::Screws, 1)],
      Item::CopperKnife => &[(Item::Copper, 2), (Item::Screws, 1)],
      Item::CombatSpear => &[(Item::Wood, 2), (Item::Steel, 1), (Item::Screws, 1)],
      Item::PipeRevolver => &[(Item::Steel, 2), (Item::Copper, 1), (Item::Screws, 2)],
      Item::LeatherVest => &[(Item::OrganicMaterial, 3), (Item::Screws, 2)],
      Item::ChainMail => &[(Item::Steel, 4), (Item::Screws, 3)],
      Item::SteelBoots => {
        &[(Item::Steel, 2), (Item::OrganicMaterial, 1), (Item::Screws, 1)]
      }
      Item::SynthHelmet => {
        &[(Item::SyntheticMaterial, 3), (Item::Glass, 1), (Item::Screws, 2)]
      }
      Item::HealthPotion => {
        &[(Item::Glass, 1), (Item::OrganicMaterial, 2), (Item::Crystal, 1)]
      }
      Item::StimPack => {
        &[(Item::OrganicMaterial, 2), (Item::Crystal, 1), (Item::Glass, 1)]
      }
      Item::CannedGoods => &[(Item::Steel, 1), (Item::OrganicMaterial, 2)],
      Item::FilterWater => &[(Item::Glass, 2), (Item::OrganicMaterial, 1)],
      Item::Torch => &[(Item::Wood, 1), (Item::OrganicMaterial, 1)],
      Item::Rock => &[(Item::Crystal, 1)],
      Item::Mushroom => &[(Item::OrganicMaterial, 2)],
      _ => &[]
    }
  }

  pub fn can_salvage(self) -> bool { !self.scrap_yield().is_empty() }
}

/// Properties bundled with each [`Tile`] variant.
pub struct TileProperties {
  pub glyph: &'static str,
  pub color: [f32; 3],
  pub minimap_color: [f32; 3],
  pub texture_path: Option<&'static str>,
  pub walkable: bool,
  pub opaque: bool,
  pub causes_falling: bool,
  pub name: &'static str,
  pub has_atmosphere: bool,
  /// Space Qud palette sprite: (path, primary [r,g,b], secondary [r,g,b]).
  pub space_qud_sprite: Option<(&'static str, [f32; 3], [f32; 3])>
}

impl Tile {
  pub fn properties(self) -> TileProperties {
    match self {
      Tile::Air => TileProperties {
        glyph: " ",
        color: [0.0, 0.0, 0.0],
        minimap_color: [0.04, 0.06, 0.10],
        texture_path: None,
        walkable: true,
        opaque: false,
        causes_falling: true,
        name: "Air",
        has_atmosphere: false,
        space_qud_sprite: None
      },
      Tile::Floor => TileProperties {
        glyph: ".",
        color: [0.6, 0.5, 0.3],
        minimap_color: [0.62, 0.55, 0.40],
        texture_path: None,
        walkable: true,
        opaque: false,
        causes_falling: false,
        name: "Floor",
        has_atmosphere: true,
        space_qud_sprite: None
      },
      Tile::Wall => TileProperties {
        glyph: "#",
        color: [0.4, 0.4, 0.4],
        minimap_color: [0.38, 0.38, 0.40],
        texture_path: None,
        walkable: false,
        opaque: true,
        causes_falling: false,
        name: "Wall",
        has_atmosphere: true,
        space_qud_sprite: None
      },
      Tile::CobblestoneWall => TileProperties {
        glyph: "#",
        color: [0.5, 0.5, 0.5],
        minimap_color: [0.38, 0.38, 0.40],
        texture_path: Some("textures/cobblestone_wall.png"),
        walkable: false,
        opaque: true,
        causes_falling: false,
        name: "Cobblestone Wall",
        has_atmosphere: true,
        space_qud_sprite: None
      },
      Tile::BrickWall => TileProperties {
        glyph: "#",
        color: [0.6, 0.3, 0.2],
        minimap_color: [0.38, 0.38, 0.40],
        texture_path: Some("textures/brick_wall.png"),
        walkable: false,
        opaque: true,
        causes_falling: false,
        name: "Brick Wall",
        has_atmosphere: true,
        space_qud_sprite: None
      },
      Tile::Grass => TileProperties {
        glyph: "\"",
        color: [0.2, 0.6, 0.2],
        minimap_color: [0.22, 0.62, 0.30],
        texture_path: None,
        walkable: true,
        opaque: false,
        causes_falling: false,
        name: "Grass",
        has_atmosphere: true,
        space_qud_sprite: Some(("textures/space_qud/grass.png", [0.22, 0.48, 0.18], [0.52, 0.72, 0.28]))
      },
      Tile::Water => TileProperties {
        glyph: "~",
        color: [0.2, 0.3, 0.8],
        minimap_color: [0.12, 0.28, 0.70],
        texture_path: None,
        walkable: false,
        opaque: false,
        causes_falling: false,
        name: "Water",
        has_atmosphere: true,
        space_qud_sprite: None
      },
      Tile::Sand => TileProperties {
        glyph: ",",
        color: [0.8, 0.7, 0.4],
        minimap_color: [0.92, 0.80, 0.52],
        texture_path: None,
        walkable: true,
        opaque: false,
        causes_falling: false,
        name: "Sand",
        has_atmosphere: true,
        space_qud_sprite: Some(("textures/space_qud/wavy.png", [0.72, 0.62, 0.38], [0.92, 0.86, 0.62]))
      },
      Tile::StairsUp => TileProperties {
        glyph: "<",
        color: [0.9, 0.9, 0.2],
        minimap_color: [0.62, 0.55, 0.40],
        texture_path: None,
        walkable: true,
        opaque: false,
        causes_falling: false,
        name: "Stairs Up",
        has_atmosphere: true,
        space_qud_sprite: None
      },
      Tile::StairsDown => TileProperties {
        glyph: ">",
        color: [0.9, 0.9, 0.2],
        minimap_color: [0.62, 0.55, 0.40],
        texture_path: None,
        walkable: true,
        opaque: false,
        causes_falling: false,
        name: "Stairs Down",
        has_atmosphere: true,
        space_qud_sprite: None
      },
      Tile::Door => TileProperties {
        glyph: "+",
        color: [0.6, 0.3, 0.1],
        minimap_color: [0.38, 0.38, 0.40],
        texture_path: None,
        walkable: true,
        opaque: false,
        causes_falling: false,
        name: "Door",
        has_atmosphere: true,
        space_qud_sprite: None
      },
      Tile::TallGrass => TileProperties {
        glyph: "\"",
        color: [0.25, 0.65, 0.25],
        minimap_color: [0.12, 0.48, 0.20],
        texture_path: None,
        walkable: true,
        opaque: false,
        causes_falling: false,
        name: "Tall Grass",
        has_atmosphere: true,
        space_qud_sprite: Some(("textures/space_qud/grass.png", [0.22, 0.48, 0.18], [0.52, 0.72, 0.28]))
      },
      Tile::Bush => TileProperties {
        glyph: "%",
        color: [0.15, 0.45, 0.15],
        minimap_color: [0.10, 0.38, 0.12],
        texture_path: None,
        walkable: false,
        opaque: false,
        causes_falling: false,
        name: "Bush",
        has_atmosphere: true,
        space_qud_sprite: None
      },
      Tile::Ash => TileProperties {
        glyph: ".",
        color: [0.55, 0.53, 0.5],
        minimap_color: [0.92, 0.80, 0.52],
        texture_path: None,
        walkable: true,
        opaque: false,
        causes_falling: false,
        name: "Ash",
        has_atmosphere: true,
        space_qud_sprite: None
      },
      Tile::Lava => TileProperties {
        glyph: "~",
        color: [0.9, 0.3, 0.05],
        minimap_color: [0.95, 0.32, 0.08],
        texture_path: None,
        walkable: false,
        opaque: false,
        causes_falling: false,
        name: "Lava",
        has_atmosphere: true,
        space_qud_sprite: None
      },
      Tile::ShallowWater => TileProperties {
        glyph: "~",
        color: [0.3, 0.5, 0.85],
        minimap_color: [0.22, 0.55, 0.82],
        texture_path: None,
        walkable: true,
        opaque: false,
        causes_falling: false,
        name: "Shallow Water",
        has_atmosphere: true,
        space_qud_sprite: Some(("textures/space_qud/wavy.png", [0.18, 0.42, 0.62], [0.45, 0.68, 0.88]))
      },
      Tile::DeepWater => TileProperties {
        glyph: "≈",
        color: [0.1, 0.15, 0.6],
        minimap_color: [0.05, 0.16, 0.42],
        texture_path: None,
        walkable: false,
        opaque: false,
        causes_falling: false,
        name: "Deep Water",
        has_atmosphere: true,
        space_qud_sprite: None
      },
      Tile::Road => TileProperties {
        glyph: "·",
        color: [0.45, 0.4, 0.35],
        minimap_color: [0.50, 0.46, 0.40],
        texture_path: None,
        walkable: true,
        opaque: false,
        causes_falling: false,
        name: "Road",
        has_atmosphere: true,
        space_qud_sprite: None
      },
      Tile::WoodWall => TileProperties {
        glyph: "#",
        color: [0.45, 0.3, 0.15],
        minimap_color: [0.38, 0.38, 0.40],
        texture_path: None,
        walkable: false,
        opaque: true,
        causes_falling: false,
        name: "Wooden Wall",
        has_atmosphere: true,
        space_qud_sprite: None
      },
      Tile::WoodFloor => TileProperties {
        glyph: ".",
        color: [0.55, 0.4, 0.25],
        minimap_color: [0.55, 0.42, 0.30],
        texture_path: None,
        walkable: true,
        opaque: false,
        causes_falling: false,
        name: "Wooden Floor",
        has_atmosphere: true,
        space_qud_sprite: None
      },
      Tile::Fence => TileProperties {
        glyph: "+",
        color: [0.5, 0.35, 0.2],
        minimap_color: [0.45, 0.55, 0.50],
        texture_path: None,
        walkable: false,
        opaque: false,
        causes_falling: false,
        name: "Fence",
        has_atmosphere: true,
        space_qud_sprite: None
      },
      Tile::CaveWall => TileProperties {
        glyph: "#",
        color: [0.3, 0.28, 0.25],
        minimap_color: [0.38, 0.38, 0.40],
        texture_path: None,
        walkable: false,
        opaque: true,
        causes_falling: false,
        name: "Cave Wall",
        has_atmosphere: true,
        space_qud_sprite: None
      },
      Tile::CaveFloor => TileProperties {
        glyph: ".",
        color: [0.4, 0.38, 0.35],
        minimap_color: [0.62, 0.55, 0.40],
        texture_path: None,
        walkable: true,
        opaque: false,
        causes_falling: false,
        name: "Cave Floor",
        has_atmosphere: true,
        space_qud_sprite: None
      },
      Tile::CrystalFormation => TileProperties {
        glyph: "*",
        color: [0.5, 0.8, 0.95],
        minimap_color: [0.45, 0.55, 0.50],
        texture_path: None,
        walkable: false,
        opaque: false,
        causes_falling: false,
        name: "Crystal Formation",
        has_atmosphere: true,
        space_qud_sprite: None
      },
      // --- Space tiles ---
      Tile::DeckPlate => TileProperties {
        glyph: ".",
        color: [0.55, 0.58, 0.62],
        minimap_color: [0.45, 0.47, 0.5],
        texture_path: None,
        walkable: true,
        opaque: false,
        causes_falling: false,
        name: "Deck Plate",
        has_atmosphere: true,
        space_qud_sprite: Some(("textures/space_qud/floor .png", [0.38, 0.42, 0.48], [0.72, 0.76, 0.82]))
      },
      Tile::Bulkhead => TileProperties {
        glyph: "#",
        color: [0.45, 0.47, 0.50],
        minimap_color: [0.35, 0.37, 0.4],
        texture_path: None,
        walkable: false,
        opaque: true,
        causes_falling: false,
        name: "Bulkhead",
        has_atmosphere: true,
        space_qud_sprite: Some(("textures/space_qud/wall hashtag.png", [0.28, 0.30, 0.34], [0.48, 0.52, 0.56]))
      },
      Tile::Window => TileProperties {
        glyph: "o",
        color: [0.2, 0.25, 0.7],
        minimap_color: [0.15, 0.2, 0.55],
        texture_path: None,
        walkable: false,
        opaque: false,
        causes_falling: false,
        name: "Window",
        has_atmosphere: true,
        space_qud_sprite: Some(("textures/space_qud/window.png", [0.22, 0.32, 0.52], [0.62, 0.76, 0.94]))
      },
      Tile::AirlockDoor => TileProperties {
        glyph: "+",
        color: [0.7, 0.65, 0.3],
        minimap_color: [0.6, 0.55, 0.2],
        texture_path: None,
        walkable: true,
        opaque: false,
        causes_falling: false,
        name: "Airlock Door",
        has_atmosphere: true,
        space_qud_sprite: None
      },
      Tile::StationFloor => TileProperties {
        glyph: ".",
        color: [0.55, 0.58, 0.62],
        minimap_color: [0.45, 0.47, 0.5],
        texture_path: None,
        walkable: true,
        opaque: false,
        causes_falling: false,
        name: "Station Floor",
        has_atmosphere: true,
        space_qud_sprite: Some(("textures/space_qud/grid.png", [0.52, 0.56, 0.62], [0.88, 0.90, 0.94]))
      },
      Tile::StationWall => TileProperties {
        glyph: "#",
        color: [0.5, 0.52, 0.55],
        minimap_color: [0.35, 0.37, 0.4],
        texture_path: None,
        walkable: false,
        opaque: true,
        causes_falling: false,
        name: "Station Wall",
        has_atmosphere: true,
        space_qud_sprite: Some(("textures/space_qud/wall hashtag.png", [0.28, 0.30, 0.34], [0.48, 0.52, 0.56]))
      },
      Tile::DerelictFloor => TileProperties {
        glyph: ".",
        color: [0.35, 0.33, 0.3],
        minimap_color: [0.28, 0.26, 0.22],
        texture_path: None,
        walkable: true,
        opaque: false,
        causes_falling: false,
        name: "Derelict Floor",
        has_atmosphere: true,
        space_qud_sprite: None
      },
      Tile::DerelictWall => TileProperties {
        glyph: "#",
        color: [0.3, 0.28, 0.25],
        minimap_color: [0.28, 0.26, 0.22],
        texture_path: None,
        walkable: false,
        opaque: true,
        causes_falling: false,
        name: "Derelict Wall",
        has_atmosphere: true,
        space_qud_sprite: Some(("textures/space_qud/wall hashtag.png", [0.28, 0.30, 0.34], [0.48, 0.52, 0.56]))
      },
      Tile::Conduit => TileProperties {
        glyph: "=",
        color: [0.6, 0.55, 0.2],
        minimap_color: [0.5, 0.45, 0.15],
        texture_path: None,
        walkable: true,
        opaque: false,
        causes_falling: false,
        name: "Conduit",
        has_atmosphere: true,
        space_qud_sprite: Some(("textures/space_qud/grid.png", [0.40, 0.28, 0.14], [0.88, 0.62, 0.22]))
      },
      Tile::AsteroidRock => TileProperties {
        glyph: "#",
        color: [0.4, 0.35, 0.3],
        minimap_color: [0.42, 0.38, 0.33],
        texture_path: None,
        walkable: false,
        opaque: true,
        causes_falling: false,
        name: "Asteroid Rock",
        has_atmosphere: true,
        space_qud_sprite: Some(("textures/space_qud/wall hashtag.png", [0.42, 0.38, 0.36], [0.58, 0.54, 0.52]))
      },
      Tile::AsteroidFloor => TileProperties {
        glyph: ".",
        color: [0.5, 0.45, 0.4],
        minimap_color: [0.42, 0.38, 0.33],
        texture_path: None,
        walkable: true,
        opaque: false,
        causes_falling: false,
        name: "Asteroid Floor",
        has_atmosphere: true,
        space_qud_sprite: Some(("textures/space_qud/ground.png", [0.48, 0.46, 0.44], [0.72, 0.70, 0.68]))
      },
      Tile::Regolith => TileProperties {
        glyph: ",",
        color: [0.55, 0.5, 0.45],
        minimap_color: [0.6, 0.62, 0.68],
        texture_path: None,
        walkable: true,
        opaque: false,
        causes_falling: false,
        name: "Regolith",
        has_atmosphere: true,
        space_qud_sprite: Some(("textures/space_qud/ground.png", [0.48, 0.46, 0.44], [0.72, 0.70, 0.68]))
      },
      Tile::Vacuum => TileProperties {
        glyph: " ",
        color: [0.0, 0.0, 0.0],
        minimap_color: [0.02, 0.03, 0.06],
        texture_path: None,
        walkable: true,
        opaque: false,
        causes_falling: false,
        name: "Vacuum",
        has_atmosphere: false,
        space_qud_sprite: Some(("textures/space_qud/space background.png", [0.04, 0.06, 0.14], [0.62, 0.72, 0.92]))
      },
      Tile::IceFloor => TileProperties {
        glyph: ",",
        color: [0.7, 0.75, 0.85],
        minimap_color: [0.6, 0.62, 0.68],
        texture_path: None,
        walkable: true,
        opaque: false,
        causes_falling: false,
        name: "Ice Floor",
        has_atmosphere: true,
        space_qud_sprite: None
      },
      Tile::IceWall => TileProperties {
        glyph: "#",
        color: [0.5, 0.55, 0.7],
        minimap_color: [0.45, 0.5, 0.62],
        texture_path: None,
        walkable: false,
        opaque: true,
        causes_falling: false,
        name: "Ice Wall",
        has_atmosphere: true,
        space_qud_sprite: None
      },
      Tile::AlienSoil => TileProperties {
        glyph: ",",
        color: [0.45, 0.35, 0.55],
        minimap_color: [0.35, 0.45, 0.3],
        texture_path: None,
        walkable: true,
        opaque: false,
        causes_falling: false,
        name: "Alien Soil",
        has_atmosphere: true,
        space_qud_sprite: None
      },
      Tile::AlienGrass => TileProperties {
        glyph: "\"",
        color: [0.3, 0.55, 0.3],
        minimap_color: [0.35, 0.45, 0.3],
        texture_path: None,
        walkable: true,
        opaque: false,
        causes_falling: false,
        name: "Alien Grass",
        has_atmosphere: true,
        space_qud_sprite: Some(("textures/space_qud/grass.png", [0.38, 0.16, 0.52], [0.68, 0.52, 0.88]))
      },
      Tile::CrystalGrowth => TileProperties {
        glyph: "*",
        color: [0.5, 0.8, 0.95],
        minimap_color: [0.4, 0.65, 0.8],
        texture_path: None,
        walkable: false,
        opaque: false,
        causes_falling: false,
        name: "Crystal Growth",
        has_atmosphere: true,
        space_qud_sprite: None
      },
      Tile::AlienFluid => TileProperties {
        glyph: "~",
        color: [0.5, 0.3, 0.7],
        minimap_color: [0.4, 0.25, 0.6],
        texture_path: None,
        walkable: true,
        opaque: false,
        causes_falling: false,
        name: "Alien Fluid",
        has_atmosphere: true,
        space_qud_sprite: None
      }
    }
  }

  pub fn glyph(self) -> &'static str { self.properties().glyph }
  pub fn color(self) -> [f32; 3] { self.properties().color }
  pub fn minimap_color(self) -> [f32; 3] { self.properties().minimap_color }
  pub fn texture_path(self) -> Option<&'static str> { self.properties().texture_path }
  pub fn walkable(self) -> bool { self.properties().walkable }
  pub fn opaque(self) -> bool { self.properties().opaque }
  pub fn causes_falling(self) -> bool { self.properties().causes_falling }
  pub fn name(self) -> &'static str { self.properties().name }
  pub fn has_atmosphere(self) -> bool { self.properties().has_atmosphere }
}

#[derive(Clone, Debug)]
pub struct Level {
  pub tiles: Vec<Vec<Tile>>,
  pub items: Vec<Vec<Option<Item>>>,
  pub width: usize,
  pub height: usize
}

impl Level {
  pub fn new(width: usize, height: usize, fill: Tile) -> Self {
    Level {
      tiles: vec![vec![fill; width]; height],
      items: vec![vec![None; width]; height],
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

  pub fn get_item(&self, x: i32, y: i32) -> Option<Item> {
    if x < 0 || y < 0 {
      return None;
    }
    let (ux, uy) = (x as usize, y as usize);
    (ux < self.width && uy < self.height).then(|| self.items[uy][ux]).flatten()
  }

  pub fn set_item(&mut self, x: i32, y: i32, item: Option<Item>) {
    if x >= 0 && y >= 0 && (x as usize) < self.width && (y as usize) < self.height {
      self.items[y as usize][x as usize] = item;
    }
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
    fill_rect(level, x + 1, y + 1, w - 2, h - 2, Tile::Floor);
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
  level.set(dx, dy, Tile::Door);
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
    level.set(cx, cy1, Tile::Floor);
    cx += dx;
  }
  let mut cy = cy1;
  let dy = if cy2 > cy1 { 1 } else { -1 };
  while cy != cy2 {
    level.set(x2, cy, Tile::Floor);
    cy += dy;
  }
  level.set(x2, cy2, Tile::Floor);
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
        level.set(x + dx, y + dy, Tile::Floor);
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
      level.set(cx, cy + offset, Tile::Floor);
      cx += dx;
    }
    // vertical leg
    let mut cy2 = y1;
    let dy = if y2 > y1 { 1 } else { -1 };
    while cy2 != y2 {
      level.set(x2 + offset, cy2, Tile::Floor);
      cy2 += dy;
    }
    level.set(x2 + offset, y2, Tile::Floor);
  }
}

// ---------------------------------------------------------------------------
// World: a stack of levels
// ---------------------------------------------------------------------------

pub const ZONE_WIDTH: usize = 48;
pub const ZONE_HEIGHT: usize = 48;

// ---------------------------------------------------------------------------
// Visibility: perimeter flood-fill
//
// Expand outward chebyshev-ring by chebyshev-ring from the viewer.
// A tile is visible if any of its parent tiles (one step closer to the
// viewer along each axis) is itself visible and not opaque.
// ---------------------------------------------------------------------------

pub struct FovGrid {
  pub visible: Vec<Vec<bool>>,
  pub revealed: Vec<Vec<bool>>,
  pub width: usize,
  pub height: usize
}

impl FovGrid {
  pub fn new(width: usize, height: usize) -> Self {
    FovGrid {
      visible: vec![vec![false; width]; height],
      revealed: vec![vec![false; width]; height],
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
      self.revealed[y][x] = true;
    }
  }

  pub fn is_visible(&self, x: usize, y: usize) -> bool {
    x < self.width && y < self.height && self.visible[y][x]
  }

  pub fn is_revealed(&self, x: usize, y: usize) -> bool {
    x < self.width && y < self.height && self.revealed[y][x]
  }
}

/// Compute FOV from (cx, cy) with the given radius on the given level.
/// Uses perimeter flood-fill: expand outward ring by ring; a tile is visible
/// if any of its parents (one step closer along each axis) are visible and
/// not opaque.
///
/// `blocks_sight` is checked in **level-local** tile coordinates (same as
/// `level.get`): extra per-cell opacity for vision (same role as [`Tile::opaque`]
/// for tiles). The viewer’s own cell is never used as a blocker for *outward*
/// propagation.
pub fn compute_fov(
  fov: &mut FovGrid,
  level: &Level,
  cx: i32,
  cy: i32,
  radius: i32,
  mut blocks_sight: impl FnMut(i32, i32) -> bool
) {
  fov.clear_visible();

  // viewer tile is always visible
  if cx >= 0 && cy >= 0 && (cx as usize) < fov.width && (cy as usize) < fov.height {
    fov.mark_visible(cx as usize, cy as usize);
  }

  // local visibility grid, offset-relative to viewer
  let size = (2 * radius + 1) as usize;
  let mut vis = vec![vec![false; size]; size];
  let r = radius as usize;
  vis[r][r] = true;

  fn sign(n: i32) -> i32 {
    if n > 0 {
      1
    } else if n < 0 {
      -1
    } else {
      0
    }
  }

  for d in 1..=radius {
    for dx in -d..=d {
      for dy in -d..=d {
        if dx.abs().max(dy.abs()) != d {
          continue;
        }
        let (sx, sy) = (sign(dx), sign(dy));
        // All parents are on ring d-1, so iteration order doesn't matter.
        // Corners use only the diagonal parent to ensure a single diagonal
        // wall tile properly occludes. Edge tiles use two inward parents
        // along their dominant axis so they aren't over-blocked.
        let parents: &[(i32, i32)] = if dx == 0 {
          &[(0, -sy)]
        } else if dy == 0 {
          &[(-sx, 0)]
        } else if dx.abs() == dy.abs() {
          // corner: only the diagonal d-1 parent
          &[(-sx, -sy)]
        } else if dx.abs() > dy.abs() {
          // vertical edge: two parents one step inward along x
          &[(-sx, 0), (-sx, -sy)]
        } else {
          // horizontal edge: two parents one step inward along y
          &[(0, -sy), (-sx, -sy)]
        };

        let visible = parents.iter().any(|&(px, py)| {
          let (pj, pi) = ((dx + px) + radius, (dy + py) + radius);
          let (uj, ui) = (pj as usize, pi as usize);
          let (lx, ly) = (cx + dx + px, cy + dy + py);
          // SS13-style: the viewer's own cell never blocks *outward* spread (e.g. standing
          // on a tree, wall, or "telefragged" into a wall still sees the ring around them).
          let parent_blocks = (lx, ly) != (cx, cy)
            && (level.get(lx, ly).is_some_and(|t| t.opaque()) || blocks_sight(lx, ly));
          uj < size && ui < size && vis[ui][uj] && !parent_blocks
        });

        if visible {
          let (j, i) = ((dx + radius) as usize, (dy + radius) as usize);
          vis[i][j] = true;
          let (wx, wy) = (cx + dx, cy + dy);
          if wx >= 0 && wy >= 0 && (wx as usize) < fov.width && (wy as usize) < fov.height
          {
            fov.mark_visible(wx as usize, wy as usize);
          }
        }
      }
    }
  }
}
