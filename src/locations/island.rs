use crate::{entities::*,
            galaxy::{Location, LocationId},
            level::{LocationType, Tile}};

pub const ID: LocationId = (12, 0, 0);

const ISLAND_DATA: &str = include_str!("../../editor_saves/island.txt");

fn object_from_save(name: &str) -> Option<Object> {
  Some(match name {
    "Tree" => Object::TREE,
    "Tree2" => Object::TREE2,
    "Boulder" => Object::BOULDER,
    "Door" => Object::DOOR,
    "AirlockDoor" => Object::AIRLOCK_DOOR,
    "Bed" => Object::BED,
    "Table" => Object::TABLE,
    "Chair" => Object::CHAIR,
    "CraftingTable" => Object::CRAFTING_TABLE,
    "Locker" => Object::LOCKER,
    "Crate" => Object::CRATE_OBJ,
    "LootChest" => Object::LOOT_CHEST,
    "FlightConsole" => Object::FLIGHT_CONSOLE,
    "LoadoutConsole" => Object::LOADOUT_CONSOLE,
    "SpaceCat" => Object::SPACE_CAT,
    "Thruster" => Object::THRUSTER,
    "RatSoldier" => Object::RAT_SOLDIER,
    "ArmoredRatSoldier" => Object::ARMORED_RAT_SOLDIER,
    "Robot" => Object::ROBOT,
    "WackRobot" => Object::WACK_ROBOT,
    "AlienRunner" => Object::ALIEN_RUNNER,
    "LavaCrab" => Object::LAVA_CRAB,
    "MantisAlien" => Object::MANTIS_ALIEN,
    "CrabAlien" => Object::CRAB_ALIEN,
    "MushroomCreature" => Object::MUSHROOM_CREATURE,
    "GrenadeThrower" => Object::GRENADE_THROWER,
    "Gunman" => Object::GUNMAN,
    "LaserSword" => Object::LASER_SWORD,
    "RobotDog" => Object::ROBOT_DOG,
    "Turret" => Object::TURRET,
    _ => return None
  })
}

struct ParsedLevel {
  width: usize,
  height: usize,
  tiles: Vec<Vec<Tile>>,
  objects: Vec<(i32, i32, Object)>
}

fn parse_save(data: &str, target_z: i32) -> ParsedLevel {
  let (grid, _markers) = data.split_once("MARKERS\n").unwrap_or((data, ""));
  let mut lines = grid.lines();
  let header = lines.next().unwrap_or("");
  let mut hi = header.split_whitespace();
  let w: usize = hi.next().and_then(|s| s.parse().ok()).unwrap_or(0);
  let h: usize = hi.next().and_then(|s| s.parse().ok()).unwrap_or(0);
  let _ox: i32 = hi.next().and_then(|s| s.parse().ok()).unwrap_or(0);
  let _oy: i32 = hi.next().and_then(|s| s.parse().ok()).unwrap_or(0);
  let multi_level = hi.next() == Some("ZRANGE");
  let mut tiles = vec![vec![Tile::Grass; w]; h];
  let mut objects = Vec::new();
  let read_row = |line: &str, y: usize, tiles: &mut Vec<Vec<Tile>>, objects: &mut Vec<(i32, i32, Object)>| {
    let mut toks = line.split_whitespace();
    for x in 0..w {
      let tile_tok = toks.next().unwrap_or("Grass");
      let obj_tok = toks.next().unwrap_or("-");
      tiles[y][x] = Tile::from_save(tile_tok).unwrap_or(Tile::Grass);
      if obj_tok != "-" && let Some(obj) = object_from_save(obj_tok) {
        objects.push((x as i32, y as i32, obj));
      }
    }
  };
  if multi_level {
    let mut cur_z: Option<i32> = None;
    let mut cur_y = 0usize;
    for line in lines {
      if let Some(rest) = line.strip_prefix("LEVEL ") {
        cur_z = rest.trim().parse().ok();
        cur_y = 0;
      } else if cur_z == Some(target_z) && cur_y < h {
        read_row(line, cur_y, &mut tiles, &mut objects);
        cur_y += 1;
      }
    }
  } else if target_z == 0 {
    let body: String = lines.collect::<Vec<_>>().join("\n");
    let mut toks = body.split_whitespace();
    for y in 0..h {
      for x in 0..w {
        let tile_tok = toks.next().unwrap_or("Grass");
        let obj_tok = toks.next().unwrap_or("-");
        tiles[y][x] = Tile::from_save(tile_tok).unwrap_or(Tile::Grass);
        if obj_tok != "-" && let Some(obj) = object_from_save(obj_tok) {
          objects.push((x as i32, y as i32, obj));
        }
      }
    }
  }
  ParsedLevel { width: w, height: h, tiles, objects }
}

pub fn generate() -> Location {
  let surface = parse_save(ISLAND_DATA, 0);
  let underground = parse_save(ISLAND_DATA, -1);
  let w = surface.width.max(underground.width);
  let h = surface.height.max(underground.height);

  let mut loc = Location::new(
    "Island",
    w, h, 2,
    LocationType::PlanetSurface { breathable: true },
    Tile::Water
  );

  let surface_level = loc.level_mut(0);
  for y in 0..surface.height {
    for x in 0..surface.width {
      surface_level.set(x as i32, y as i32, surface.tiles[y][x]);
    }
  }
  let under_level = loc.level_mut(1);
  for y in 0..h {
    for x in 0..w {
      under_level.set(x as i32, y as i32, Tile::CaveWall);
    }
  }
  for y in 0..underground.height {
    for x in 0..underground.width {
      under_level.set(x as i32, y as i32, underground.tiles[y][x]);
    }
  }

  let mut spawns: Vec<(i32, i32, usize, Object)> =
    surface.objects.into_iter().map(|(x, y, o)| (x, y, 0, o)).collect();
  spawns.extend(underground.objects.into_iter().map(|(x, y, o)| (x, y, 1, o)));

  let surface_stairs = (0..surface.height as i32)
    .flat_map(|y| (0..surface.width as i32).map(move |x| (x, y)))
    .find(|&(x, y)| surface.tiles[y as usize][x as usize] == Tile::StairsDown);
  let under_stairs = (0..underground.height as i32)
    .flat_map(|y| (0..underground.width as i32).map(move |x| (x, y)))
    .find(|&(x, y)| underground.tiles[y as usize][x as usize] == Tile::StairsUp);

  if let (Some((sx, sy)), Some((ux, uy))) = (surface_stairs, under_stairs) {
    spawns.push((sx, sy, 0, Object::cave_entrance(sx, sy, ux, uy)));
    spawns.push((ux, uy, 1, Object::cave_exit(sx, sy, ux, uy)));
  }

  loc.spawn_objects = spawns;
  loc
}
