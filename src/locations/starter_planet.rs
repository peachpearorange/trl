use crate::{entities::Object,
          galaxy::{Location, LocationId},
          level::{LocationType, Tile},
          npcs,
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
ggggggggggggggggggrrrrrrrrrrrrgggggggggggggggggg
ggggggggggggggggggrrrrrrPrrrrrgggggggggggggggggg
ggggggggggggggggggrrrrrrrrrrrrgggggggggggggggggg
ggggggggggggggggggrrrrrrrrrrrrgggggggggggggggggg
gggggggggggggwwwwwwwwwwwDwwwwwwwwwwggggggggggggg
gggggggggggggwffffwfffffffffffffffwggggggggggggg
gggggggggggggwffff.fffffffffffffffwggggggggggggg
gggggggggggggw@fffwffffffffRffffffwggggggggggggg
gggggggggggggwffffwffffffKffffffffwggggggggggggg
gggggggggggggwwwwwwwwwwwwwwwwwwwwwwggggggggggggg
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
    .assoc('r', (Tile::Road, []))
    .assoc('c', (Tile::CrystalGrowth, []))
    .assoc('~', (Tile::AlienFluid, []))
    .assoc('t', (Tile::AlienGrass, [Object::tree()]))
    .assoc('P', (Tile::ShipDock, []))
    .assoc('@', (Tile::WoodTile, [Object::bed()]))
    .assoc('R', (Tile::StationFloor, [npcs::tutorial::ori1()]))
    .assoc('K', (Tile::StationFloor, [Object::crafting_table()]))
    .assoc('D', (Tile::StationFloor, [Object::door()]))
    .assoc('.', (Tile::StationFloor, []))
}

pub fn generate() -> Location {
  Location::from_prefab(
    "Origin Planet",
    surface_prefab(),
    LocationType::PlanetSurface { breathable: true },
    Tile::AlienGrass
  )
}
