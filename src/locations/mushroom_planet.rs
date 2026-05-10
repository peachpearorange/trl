use {bevy::prelude::Color, crate::entities::*};
use crate::{galaxy::{Location, LocationId},
            level::{LocationType, Tile},
            prefabs::{prefab, Prefab}};

pub const ID: LocationId = (3, 0, 0);

fn shroom(r: f32, g: f32, b: f32, r2: f32, g2: f32, b2: f32, name: &'static str) -> Object {
  Object::mushroom(Color::srgb(r, g, b), Color::srgb(r2, g2, b2), name)
}

pub fn mushroom_prefab() -> Prefab {
  prefab(
"ssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssss
ssssss1ssssssssssssssssssssssssssssssssssssssssssssssssssssssss2ssssssssssssssss
ssssssssssssssssssssssssBBBBsssssssssssssssssssssssssssssssssssssssssssss3sssssss
ssssssssssssssssssssssBBBBBBBssssssssssssssssssssssssssssssssssssssssssssssssssss
ssss1sssssssssssssssBBBBBBBBBBsssssssssssssssssssssssssssssssssss2sssssssssssssss
sssssssssssssssssssBBBBBBBBBBBssssssssssssss4ssssssssssssssssssssssssssssssssssss
ssssssssssssssssssssBBBBBBBBBsssssssssssssssssssss3sssssssssssssssssssssssssssss
ssssssssssssssssssssssBBBBBssssssssssssssssssssssssssssssssssssssssssssssssssssss
ssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssss
sssss2ssssssssssssssssssssssssssssssssssGGGGsssssssssssssssssssssssssssss1ssssssss
ssssssssssssssssssssssssssssssssssssssGGGGGGGsssssssssssssssssssssssssssssssssssss
ssssssssssssssssssssssssss5sssssssssssGGGGGGGssssssssssssssssssssssssssssssssssss
sssssssssssssssssssssssssssssssssssssssGGGGGssssssssssssssssss4ssssssssssssssssss
ssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssss
ssss3sssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssss5sssssssss
ssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssss
ssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssss
ssssssssssssCCCCssssssssssssssssssssssssssssssssssssss1ssssssssssssssssssssssssss
sssssssssssCCCCCCssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssss
sssssssssssCCCCCCCsssssss2ssssssssssssssssssssssssssssssssssssssssssssssssssssss
sssssssssssCCCCCCCsssssssssssssssssssssssssssssssssssssssssssss3ssssssssssssssss
sssssssssssCCCCCCssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssss
ssssssssssssCCCssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssss
ssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssss
ssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssss
sssssssssssssssssssssssss4sssssssssssssssssssssssssssssssssssssssssssssssssssssss
ssssssssssssssssssssssssssssssssssssAAAAAAAAsssssssssssssssssssssssssssssssssssssss
sssssssssssssssssssssssssssssssssAAAAAAAAAAAAssssssssssssssssssssssssssssssssssssss
ssssssss5sssssssssssssssssssssssAAAAAAAAAAAAsssssssssssssssssssssssssssssssssssssss
ssssssssssssssssssssssssssssssssAAAAAAAAAAAAssssssssssssssssssssssssssssssssssssss
ssssssssssssssssssssssssssssssssAAAAAAAAAAAAsssssssssssssss5sssssssssssssssssssssss
sssssssssssssssssssssssssssssssssAAAAAAAAAAAAssssssssssssssssssssssssssssssssssssss
ssssssssssssssssssssssssssssssssssAAAAAAAAssssssssssssssssssssssssssssssssssssssssss
sssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssss
sssssss1sssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssss
ssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssss
ssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssss
sssssssssssssssssssssssssssrrrsssssssssssssssssssssssssssssssssssssssssssssssssssss
sssssssssssssssssssssssssssrPrssssssssssssssssssssssssssssssssssssssssssssssssssss
sssssssssssssssssssssssssssrrrssssssssssssssssssssssssssssssssssssssssssssssssssss
ssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssss
"
  )
  .assoc('s', (Tile::AlienSoil, []))
  .assoc('g', (Tile::AlienGrass, []))
  .assoc('B', (Tile::BioluminescentPool, []))
  .assoc('G', (Tile::AcidPool, []))
  .assoc('C', (Tile::CrimsonPool, []))
  .assoc('A', (Tile::AmberPool, []))
  .assoc('P', (Tile::ShipDock, []))
  // primary = hat, secondary = stem + spots
  .assoc('1', (Tile::AlienSoil, [shroom(0.52, 0.06, 0.72, 0.94, 0.88, 0.98, "Violet Cap")]))   // deep purple hat, white-lavender stem/spots
  .assoc('2', (Tile::AlienSoil, [shroom(0.08, 0.52, 0.30, 0.78, 0.96, 0.82, "Jade Fungus")])) // dark teal hat, pale mint stem/spots
  .assoc('3', (Tile::AlienSoil, [shroom(0.82, 0.10, 0.06, 0.98, 0.84, 0.52, "Ember Stalk")])) // deep crimson hat, warm cream stem/spots
  .assoc('4', (Tile::AlienSoil, [shroom(0.06, 0.20, 0.85, 0.70, 0.86, 0.98, "Azure Bloom")])) // cobalt hat, ice-blue stem/spots
  .assoc('5', (Tile::AlienSoil, [shroom(0.95, 0.78, 0.04, 0.98, 0.97, 0.84, "Pale Lantern")]))
  .assoc('n', (Tile::AlienSoil, [spore_tender()]))
}

static SPORE_TENDER_DIALOGUE: DialogueTree = tree(&[
  node(
    "root",
    "The mycelium remembers. Every spore that falls here is added to the record. You are new.",
    &[go("What record?", "record"), end("I'll leave you to it.")]
  ),
  node(
    "record",
    "The fruiting bodies are memory. Their colors, their shapes — each one a thought the colony chose to express. We do not speak quickly. We speak permanently.",
    &[end("I see.")]
  )
]);

fn spore_tender() -> Object {
  Object::defined_npc(
    Named {
      name:   "Spore-Tender",
      flavor: "A tall fungal being, its cap a deep violet, its movements slow and deliberate. Spores drift from its gills."
    },
    Stats { hp: 20, max_hp: 20, attack: 2, move_speed: 1.5, attack_speed: 0.4 },
    None,
    None,
    Glyph::palette_sprite(
      "textures/space_qud/mushroom.png",
      'n',
      Color::srgb(0.52, 0.06, 0.72),
      Color::srgb(0.94, 0.88, 0.98)
    ),
    &SPORE_TENDER_DIALOGUE
  )
}

pub fn generate() -> Location {
  Location::from_prefab(
    "Mushroom Planet",
    mushroom_prefab(),
    LocationType::PlanetSurface { breathable: true },
    Tile::AlienSoil
  )
}
