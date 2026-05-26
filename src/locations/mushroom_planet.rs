use {crate::{entities::*,
             galaxy::{Location, LocationId},
             level::{LocationType, Tile},
             prefabs::{Prefab, prefab}},
     bevy::prelude::Color};

pub const ID: LocationId = (3, 0, 0);

fn shroom(
  r: f32,
  g: f32,
  b: f32,
  r2: f32,
  g2: f32,
  b2: f32,
  name: &'static str
) -> Object {
  Object::mushroom(Color::srgb(r, g, b), Color::srgb(r2, g2, b2), name)
}

pub fn mushroom_prefab() -> Prefab {
  prefab(
    "ssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssss
ssssss1sssssssssssssssmsss111sssssssssssssssssssssssssssssssssss2ssssssssssssssss
ssssssssssssssssssssss11BBBB1ssssssssssssssssssssssssssssssssssssssssssss3sssssss
ssssssssssssssssssss11BBBBBBBssssssssssssssssssssss4ssssss4sssss4sss4ssssssssssss
ssss1ssssssssssssss1BBBBBBBBBBssssssssssssss4sssssssss4sss4s4ssss4sssssssssssssss
ssssssssssssssssss1BBBBBBBBBBBssssssssssssss4s4ss4ss4ssssssss4ssss4s4ssssggssssss
ssssssssssss5ss5sss1BBBBBBBBBssssssssss4ss4ss4ssss34ssss4sss4sssssssssssggggssss
ssssssssss5sssssssss11BBBBBssssssssss4sss4ssssssssssssssss4ssssssssssssggggssssss
ssssss5ssssss5ssssssssssssssssssssss4s4sss4ssss4sss4sssssssssssssssssssgggssssssss
sssss2s5ss5sssss5sssssssssssssss4ss4ssssGGGGssssssssssssssssssssssssssggggssssssss
sssssss5sss5ssssssssssssssssssssssssssGGGGGGGsssssssssggggggggggssssssgggsssssssss
ssssssssssssss5sssssssssss5sssss4sssssGGGGGGGsssssssggggggggggggggggsggggssssssss
ssssssss5sssssssss5ssssssssssssssssssssGGGGGsssssssgggggsggggg4ssssgggsggssssssss
sssssssssss5sssssssssssssmssssssssssssssssssssssssssggggggggsssssssssggggssssssssss
ssss3sss5ssssss5sssssssssssssssssssssssssssssssssssgggggggssssssssesgggg5sssssssss
sssssssCCCCC5CCssss5ssssssssssssssssssssssssssssssssgggggsssssssssssgggsssssssssss
sssssssCC5CCCCCC5sssssssssssssssssssssssssssssssssssssssssssssssssssgggsssssssssss
sssssssCCCCCC5CCs5s5sssssssssssssssssss2ssssssssssssss1sssssssssssssgggssssssssss
sssssssCCC5CC5CCCsss5ssssssssssssssss2sssss2ssssssssssssssssssssssssggggsssssssss
sssssssCCCCCCC5CCCsssssss2sssss2sssss2ssssss2sssssssssssssssssssssssgggggsssssss
ssssssssCC5CCCCCCCssssssssssssssss2ssssssssssssssssssssssssssss3ssssggsggggsssss
sssssssssCCCCCCCCssssssssss2ssssssss2sssssssssssssssssssssssssssssssggssggggsssss
ssssssssssCCCCCCssssssssssmsssssssss2ssssssssssssssssssssssssssssssssggsssggggssss
ssssssssssssssssssssssss2ss2sssssssssssssssssssssssssssssssssssssssgggssssgggsssss
ssssssssssssssssssssssssssssss2ssssssssssssssssssssssssssssssssssssggssssssgggssss
sssssssssssssssssssssss2s4s2ssssssssssssssssssssssssssssssssssssssgggsssssssggsss
sssssssssssssssssss2ssssssssssssssssAAAAAAAAssssssssssssssssssssssgggsssssssgggssss
sssssssssssssssssssssssss2sssssssAAAAAAAAAAAAsssssssssssssssssssssgggssssssssggssss
ssssssss5sssssssssss2ss2ssssssssAAAAAAAAAAAAssssssssssssssssss3ssgggsssssssssggssss
sssssssssssssssss2sss2ssssssssssAAAAAAAAAAAAsssssssssssssssssssssgsgssssssssssgsss
ssssssssssssssssssssssssssssssssAAAAAAAAAAAAsssssssssssssss5ssssssssssssssssssggsss
ssssssssssssssss22sssssssssssssssAAAAAAAAAAAAsssssssssssssssssss3ssssssssssssssssss
sssssssss2ss2s2sssssssssssssssssssAAAAAAAAsssssssssssssssssssss3s3ssssssssssssssssss
ssssssssssssss2s2ssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssss
sssssss1sssssssssssssssssssssssssssssssssssssssssssssssssssssssss3s33ssssssssssssss
ssssssssss2sssssssssssssssssssssssssssssssssssssssssssssssssssssssm3ssssssssssssss
ssssssssss2ssssssssssssssssssssssssssssssssssssssssssssssssssssssss3ssss3ssssssssss
sssssssssssssssssssssssssssrrrsssssssssssssssssssssssss1BBBBBBssssss33sssssssssssss
sssssssssssssssssssssssssssrPrssssssssssssssssssssssss1BBBBBBBBBBsss3sssssssssssss
sssssssssssssssssssssssssssrrrssssssssssssssssssssssss1BBBBBBBBBBssss3ssssssssssss
sssssssssssssssssssssssssssssssssssssssssssssssssssssss1BBBBBBBB1ssss3sss3ssssssss
sssssssssssssssssssssssssssssssssssssssssssssssssssssss1BBBBBBB1ssssss3s3sssssssss
ssssssssssssssssssssssssssssssssssssssssssssssssssssssss1BBBB1sssssssss3ssssssssss
sssssssssssssssssssssssssssssssssssssssssssssssssssssssss1s1ssssssssssssssssssssss
sssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssss33s3sssss
sssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssss3ssssss
sssssgggssssssssssssssssssgggggsssssssssssssssssssssssssssssssggggsssssssss33sssss
ssssggggggggggsssssssssssggggggssssssssssssssssssssssssssssssggggggsssssmssss3ssss
ssssgggggggggggggssssssssgggggggsssssssssssssssssssssssssssgggggggssssssssss3sssss
ssssgggggggggggggsggggggggggggggsssssssssssssssssssssssssgggggggggssssssssssss3sss
ssssggggggggggggggggggggggggggsgssssssssssssssssssssssssggggggssssssssssssssssssss
ssssgggggggggggggggggggggggggggggssssssssssssssssssssggggggggssssssssssssssmsssss
sssssssggggggggggssssssssssgggsgsgssssssssssssssssggggggggssssssssssssssssssssssss
ssssssssssggggggssssssssssssggggggssssssssssssgggggggggsssssssssssssssssssssssssss
ssssssssssssssssssssssssssssssggggggssssgggggggggggggsssssssssssssssssssssssssssss
sssssssssssssssssssssssssssssssggggggggggggggggggggsssssssssssssssssssCCCsssssssss
ssssssssssssssssssssssssssssssssgsgggggggggggggggssssssssssssssssssssCCCCCCsssssss
ssssssssssssssssssGGGGsssssssssssgsggggggggggsssssssssssssssssssssssCCCCCCCCssssss
ssssssssssssssssGGGGGGGssssssssssssggsgsgggggggssssssssmssssssssssssCCCCCCCCssssss
sssssssssssssssGGGGGGGGssssssssssssssssssssgggggggssssssssssssssssssCCCCCCCsssssss
ssssssssssssssGGGGGGGGGGGsssssssssssssssssssggggggggssssssssssssssssssCCCCssssssss
sssssssssssssGGGGGGGGGGsGssssssssssssssssssssssgggggggssssssssssssssssssssssssssss
sssssssssssssGGGGGGGGGGGsssssssssssssssssssssssssgggggggssssssssssssssssssssssssss
ssssssssssssssGGGGGGGssssssssssssssssssssssssssssssggggggsssssssssssssssssssssssss
ssssssssssssssssssssssssssssssssssssssssssssssssssssggggggssssssssssssssssssssssss
ssssssssssssssssssssssssssssssssssssssssssssssssssssssggggggssssssssssssssssssssss
sssssssssssssssssssssssssssssssssssssssssssssssssssssssssggsssssssssssssssssssssss
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
  .assoc(
    '1',
    (Tile::AlienSoil, [shroom(0.52, 0.06, 0.72, 0.94, 0.88, 0.98, "Violet Cap")])
  ) // deep purple hat, white-lavender stem/spots
  .assoc(
    '2',
    (Tile::AlienSoil, [shroom(0.08, 0.52, 0.30, 0.78, 0.96, 0.82, "Jade Fungus")])
  ) // dark teal hat, pale mint stem/spots
  .assoc(
    '3',
    (Tile::AlienSoil, [shroom(0.82, 0.10, 0.06, 0.98, 0.84, 0.52, "Ember Stalk")])
  ) // deep crimson hat, warm cream stem/spots
  .assoc(
    '4',
    (Tile::AlienSoil, [shroom(0.06, 0.20, 0.85, 0.70, 0.86, 0.98, "Azure Bloom")])
  ) // cobalt hat, ice-blue stem/spots
  .assoc(
    '5',
    (Tile::AlienSoil, [shroom(0.95, 0.78, 0.04, 0.98, 0.97, 0.84, "Pale Lantern")])
  )
  .assoc('n', (Tile::AlienSoil, [spore_tender()]))
  .assoc('m', (Tile::AlienSoil, [Object::MUSHROOM_CREATURE.clone()]))
  .assoc('e', (Tile::AlienSoil, [Object::GRENADE_THROWER.clone()]))
}

static SPORE_TENDER_DIALOGUE: DialogueTree = dialogue_tree(&[
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
      name: "Spore-Tender",
      flavor: "A tall fungal being, its cap a deep violet, its movements slow and deliberate. Spores drift from its gills."
    },
    Stats { hp: 20, max_hp: 20, attack: 2, move_speed: 1.5, attack_speed: 0.4 },
    Loadout::default(),
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
