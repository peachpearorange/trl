use crate::{entities::Object,
          galaxy::{Location, LocationId},
          level::{LocationType, Tile},
          prefabs::{prefab, Prefab}};

pub const ID: LocationId = (0, 0, 0);

pub fn surface_prefab() -> Prefab {
  prefab(
"gggggggggggggggggggggggggggggggggggggggggggggggg
gggggggggggggggggggggggggggggggggggggggggggggggg
gggggggggggggggggggggggggggggggggggggggggggggggg
gggtgggggggggggggggggggggggggggggggggggggggggtgg
gggggggggggggggggggggggggggggggggggggggggggggggg
gggggcgggggggggggggggggggggggggggggggggggggggggg
gggggggggggggggggggggggggggggggggggggggggggggggg
gggggggggggggggggggggggggggggggggggggggggggggggg
ggggggggggggggggggggggggggggggggggggggggcggggggg
gggggggggggggggggggggggggggggggggggggggggggggggg
gggggggggggggggggggggggggggggggggggggggggggggggg
gggggggggggggggggggggggggggggggggggggggggggggggg
ggggggggcggggggggggggggggggggggggggggggggggggggg
gggggggggggggggggggggggggggggggggggggggggggggggg
gggggggggggggggggggggggggggggggggggggggggggggggg
gggggggggggggggggggggggggggggggggggggggggggggggg
gggggggggggggggggggggggggggggggggggggggggggggggg
gggggggggggggggggggggggggggggggggggggggggggggggg
gggggggggggggggggggggggggggggggggggggggggggggggg
gggggggggggggggggggggggggggggggggggggggggggggggg
ggggggggggggggggggwwwwwwwwwwwwgggggggggggggggggg
ggggggggggggggggggwffffffffffwgggggggggggggggggg
ggggggggggggggggggwffffffffffwgggggggggggggggggg
ggggggggggggggggggwffffffffffwgggggggggggggggggg
ggggggggggggggggggwffffffffffwgggggggggggggggggg
ggggggggggggggggggwffffffffffwgggggggggggggggggg
ggggggggggggggggggwffffffffffwgggggggggggggggggg
ggggggggggggggggggwwwwwwwwwwwwgggggggggggggggggg
ggggggggggggggggggrrrrrrrrrrrrgggggggggggggggggg
ggggggggggggggggggrrrrrrPrrrrrgggggggggggggggggg
ggggggggggggggggggrrrrrrrrrrrrggggggggcggggggggg
ggggggggggggggggggrrrrrrrrrrrrgggggggggggggggggg
gggggggggggggggggggggggggggggggggggggggggggggggg
gggggggggggggggggggggggggggggggggggggggggggggggg
gggggggggggggggggggggggggggggggggggggggggggggggg
gggggggggg~~gggggggggggggggggggggggggggggggggggg
gggggggggg~ggggggggggggggggggggggggggggggggggggg
gggggggggggggggggggggggggggggggggggggggggggggggg
gggggggggggggggggggggggggggggggggggggggggggggggg
gggggggggggggggggggggggggggggggggggggggggggggggg
gggggggggggggggggggggggggggggggggggggggggggggggg
gggggggggggggggggggggggggggggggggggggggggggggggg
gggggggggggggggggggggggggggggggggggggggggggggggg
gggggggggggggggggggggggggggggggggggggggggggggggg
gggggggggggggggggggggggggggggggggggggggggggggggg
gggtgggggggggggggggggggggggggggggggggggggggggtgg
gggggggggggggggggggggggggggggggggggggggggggggggg
gggggggggggggggggggggggggggggggggggggggggggggggg
")
    .assoc('g', (Tile::AlienGrass, []))
    .assoc('w', (Tile::StationWall, []))
    .assoc('f', (Tile::StationFloor, []))
    .assoc('d', (Tile::Door, [Object::door()]))
    .assoc('r', (Tile::Road, []))
    .assoc('c', (Tile::CrystalGrowth, []))
    .assoc('~', (Tile::AlienFluid, []))
    .assoc('t', (Tile::AlienGrass, [Object::tree()]))
    .assoc('P', (Tile::ShipDock, []))
}

pub fn generate() -> Location {
  Location::from_prefab(
    surface_prefab(),
    LocationType::PlanetSurface { breathable: true },
    Tile::AlienGrass
  )
}

