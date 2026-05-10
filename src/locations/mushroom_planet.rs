use bevy::prelude::Color;
use crate::{entities::Object,
            galaxy::{Location, LocationId},
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
  .assoc('r', (Tile::AlienSoil, []))
  .assoc('B', (Tile::BioluminescentPool, []))
  .assoc('G', (Tile::AcidPool, []))
  .assoc('C', (Tile::CrimsonPool, []))
  .assoc('A', (Tile::AmberPool, []))
  .assoc('P', (Tile::ShipDock, []))
  .assoc('1', (Tile::AlienSoil, [shroom(0.72, 0.18, 0.62, 0.92, 0.55, 0.85, "Violet Cap")]))
  .assoc('2', (Tile::AlienSoil, [shroom(0.18, 0.72, 0.45, 0.55, 0.95, 0.72, "Jade Fungus")]))
  .assoc('3', (Tile::AlienSoil, [shroom(0.85, 0.35, 0.12, 0.98, 0.68, 0.35, "Ember Stalk")]))
  .assoc('4', (Tile::AlienSoil, [shroom(0.15, 0.52, 0.88, 0.45, 0.78, 0.98, "Azure Bloom")]))
  .assoc('5', (Tile::AlienSoil, [shroom(0.92, 0.88, 0.15, 0.98, 0.98, 0.62, "Pale Lantern")]))
}

pub fn generate() -> Location {
  Location::from_prefab(
    "Mushroom Planet",
    mushroom_prefab(),
    LocationType::PlanetSurface { breathable: true },
    Tile::AlienSoil
  )
}
