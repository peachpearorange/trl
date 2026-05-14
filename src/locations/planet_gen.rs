use bevy_ghx_proc_gen::proc_gen::{
    generator::{
        builder::GeneratorBuilder,
        model::ModelCollection,
        rules::RulesBuilder,
        socket::{SocketCollection, SocketsCartesian2D},
        RngMode,
    },
    ghx_grid::cartesian::{coordinates::Cartesian2D, grid::CartesianGrid},
};

use crate::{
    entities::Object,
    galaxy::{Location, LocationId},
    level::{LocationType, Tile},
};

pub const ID_ALIEN_JUNGLE: LocationId  = (4, 0, 0);
pub const ID_CRYSTAL_CAVES: LocationId = (5, 0, 0);
pub const ID_ARCTIC_WASTE: LocationId  = (6, 0, 0);
pub const ID_DESERT_WORLD: LocationId  = (1, 1, 0);

pub fn all() -> Vec<(LocationId, Location)> {
    vec![
        (
            ID_ALIEN_JUNGLE,
            generate(&PlanetParams::alien("Xel-Nara IV")
                .with_water(0.35)
                .with_vegetation(0.6)
                .with_seed(0xDEAD_BEEF)),
        ),
        (
            ID_CRYSTAL_CAVES,
            generate(&PlanetParams::crystal("Keth Caverns")
                .with_rocks(0.4)
                .with_vegetation(0.4)
                .with_seed(0x1337_C0DE)),
        ),
        (
            ID_ARCTIC_WASTE,
            generate(&PlanetParams::arctic("Boreas Prime")
                .with_water(0.25)
                .with_rocks(0.3)
                .with_seed(0xFEED_FACE)),
        ),
        (
            ID_DESERT_WORLD,
            generate(&PlanetParams::desert("Khamsin Reach")
                .with_water(0.08)
                .with_rocks(0.35)
                .with_seed(0xCAFE_BABE)),
        ),
    ]
}

pub const PLANET_SIZE: usize = 100;

#[derive(Clone, Copy, Debug)]
pub enum PlanetBiome {
    Grassland,
    Desert,
    Crystal,
    Alien,
    Arctic,
    Lava,
}

pub struct PlanetParams {
    pub name: &'static str,
    pub biome: PlanetBiome,
    pub breathable: bool,
    pub water_coverage: f32,
    pub vegetation_density: f32,
    pub rock_frequency: f32,
    pub seed: Option<u64>,
}

impl PlanetParams {
    pub fn grassland(name: &'static str) -> Self {
        Self { name, biome: PlanetBiome::Grassland, breathable: true,  water_coverage: 0.3,  vegetation_density: 0.5, rock_frequency: 0.1,  seed: None }
    }
    pub fn alien(name: &'static str) -> Self {
        Self { name, biome: PlanetBiome::Alien,     breathable: false, water_coverage: 0.25, vegetation_density: 0.4, rock_frequency: 0.15, seed: None }
    }
    pub fn lava(name: &'static str) -> Self {
        Self { name, biome: PlanetBiome::Lava,      breathable: false, water_coverage: 0.35, vegetation_density: 0.0, rock_frequency: 0.4,  seed: None }
    }
    pub fn crystal(name: &'static str) -> Self {
        Self { name, biome: PlanetBiome::Crystal,   breathable: false, water_coverage: 0.1,  vegetation_density: 0.3, rock_frequency: 0.3,  seed: None }
    }
    pub fn arctic(name: &'static str) -> Self {
        Self { name, biome: PlanetBiome::Arctic,    breathable: false, water_coverage: 0.2,  vegetation_density: 0.0, rock_frequency: 0.25, seed: None }
    }
    pub fn desert(name: &'static str) -> Self {
        Self { name, biome: PlanetBiome::Desert,    breathable: false, water_coverage: 0.1,  vegetation_density: 0.0, rock_frequency: 0.3,  seed: None }
    }

    pub fn with_seed(mut self, seed: u64) -> Self { self.seed = Some(seed); self }
    pub fn with_water(mut self, v: f32) -> Self { self.water_coverage = v; self }
    pub fn with_vegetation(mut self, v: f32) -> Self { self.vegetation_density = v; self }
    pub fn with_rocks(mut self, v: f32) -> Self { self.rock_frequency = v; self }
}

fn is_solid_ground(tile: Tile) -> bool {
    matches!(
        tile,
        Tile::Grass | Tile::TallGrass | Tile::Ash | Tile::CaveFloor
            | Tile::IceFloor | Tile::AlienSoil | Tile::AlienGrass
    )
}

/// Scale `param` by `scale`; WFC requires weight > 0 so floor at 0.05.
fn scaled(param: f32, scale: f32) -> f32 { (param * scale).max(0.05) }

pub fn generate(params: &PlanetParams) -> Location {
    let mut sockets = SocketCollection::new();
    let ground  = sockets.create();
    let shallow = sockets.create();
    let deep    = sockets.create();
    let rock    = sockets.create();

    sockets.add_connections([
        (ground,  vec![ground, shallow, rock]),
        (shallow, vec![shallow, deep, ground]),
        (deep,    vec![deep, shallow]),
        (rock,    vec![rock, ground]),
    ]);

    let mut models = ModelCollection::<Cartesian2D>::new();
    let mut tile_map: Vec<Tile> = Vec::new();

    macro_rules! tile {
        ($sock:expr, $weight:expr, $t:expr) => {{
            models.create(SocketsCartesian2D::Mono($sock)).with_weight($weight);
            tile_map.push($t);
        }};
    }

    let (wc, vd, rf) = (params.water_coverage, params.vegetation_density, params.rock_frequency);

    match params.biome {
        PlanetBiome::Grassland => {
            tile!(ground,  10.0,             Tile::Grass);
            tile!(ground,  scaled(vd, 5.0),  Tile::TallGrass);
            tile!(ground,  scaled(vd, 3.0),  Tile::Bush);
            tile!(shallow, scaled(wc, 8.0),  Tile::ShallowWater);
            tile!(deep,    scaled(wc, 4.0),  Tile::DeepWater);
            tile!(rock,    scaled(rf, 6.0),  Tile::Wall);
        }
        PlanetBiome::Desert => {
            tile!(ground,  10.0,             Tile::Ash);
            tile!(ground,  scaled(rf, 4.0),  Tile::CaveFloor);
            tile!(rock,    scaled(rf, 8.0),  Tile::CaveWall);
            tile!(shallow, scaled(wc, 4.0),  Tile::AlienFluid);
            tile!(deep,    scaled(wc, 2.0),  Tile::AcidPool);
        }
        PlanetBiome::Crystal => {
            tile!(ground,  8.0,              Tile::CaveFloor);
            tile!(rock,    scaled(rf, 8.0),  Tile::CaveWall);
            tile!(rock,    scaled(vd, 5.0),  Tile::CrystalFormation);
            tile!(ground,  scaled(vd, 4.0),  Tile::Ash);
            tile!(shallow, scaled(wc, 3.0),  Tile::BioluminescentPool);
            tile!(deep,    scaled(wc, 2.0),  Tile::AcidPool);
        }
        PlanetBiome::Alien => {
            tile!(ground,  8.0,              Tile::AlienSoil);
            tile!(ground,  scaled(vd, 6.0),  Tile::AlienGrass);
            tile!(shallow, scaled(wc, 5.0),  Tile::AlienFluid);
            tile!(deep,    scaled(wc, 3.0),  Tile::BioluminescentPool);
            tile!(rock,    scaled(rf, 5.0),  Tile::CaveWall);
        }
        PlanetBiome::Arctic => {
            tile!(ground,  10.0,             Tile::IceFloor);
            tile!(rock,    scaled(rf, 8.0),  Tile::IceWall);
            tile!(shallow, scaled(wc, 6.0),  Tile::ShallowWater);
            tile!(deep,    scaled(wc, 3.0),  Tile::DeepWater);
        }
        PlanetBiome::Lava => {
            tile!(ground,  8.0,              Tile::Ash);
            tile!(rock,    scaled(rf, 10.0), Tile::CaveWall);
            tile!(shallow, 8.0,              Tile::Lava);
            tile!(deep,    scaled(wc, 6.0),  Tile::CrimsonPool);
        }
    }

    let rules = RulesBuilder::new_cartesian_2d(models, sockets)
        .build()
        .expect("planet_gen: rules build failed");
    let grid = CartesianGrid::new_cartesian_2d(
        PLANET_SIZE as u32, PLANET_SIZE as u32, false, false,
    );
    let rng = params.seed.map(RngMode::Seeded).unwrap_or(RngMode::RandomSeed);

    let mut generator = GeneratorBuilder::new()
        .with_rules(rules)
        .with_grid(grid)
        .with_rng(rng)
        .with_max_retry_count(100)
        .build()
        .expect("planet_gen: generator build failed");

    let (_info, grid_data) = generator.generate_grid().expect("planet_gen: generation failed");

    let fill = tile_map[0];
    let mut loc = Location::new(
        params.name,
        PLANET_SIZE,
        PLANET_SIZE,
        1,
        LocationType::PlanetSurface { breathable: params.breathable },
        fill,
    );
    let level = loc.level_mut(0);

    for y in 0..PLANET_SIZE as u32 {
        for x in 0..PLANET_SIZE as u32 {
            level.set(x as i32, y as i32, tile_map[grid_data.get_2d(x, y).model_index]);
        }
    }

    place_ship_dock(level, fill);

    if matches!(params.biome, PlanetBiome::Alien) {
        // Use the WFC seed (or a fixed fallback) to deterministically place hostiles.
        let seed32 = params.seed
            .map(|s| (s ^ (s >> 32)) as u32)
            .unwrap_or(0xa3c1_e5f7);
        scatter_hostiles(&mut loc, seed32);
    }

    loc
}

/// Scatter alien_runner hostiles across the surface in a 5×5 grid with per-cell jitter.
fn scatter_hostiles(loc: &mut Location, seed32: u32) {
    const COLS: i32 = 5;
    const ROWS: i32 = 5;
    let ps = PLANET_SIZE as i32;
    let cell_w = ps / COLS;
    let cell_h = ps / ROWS;

    let mut rng = seed32.wrapping_mul(0xa3c1_e5f7).wrapping_add(0x3b4d_9a21);

    for row in 0..ROWS {
        for col in 0..COLS {
            rng = rng.wrapping_mul(0x5851f42d).wrapping_add(0xc4ceb9fe);
            let jx = ((rng >> 8) & 0xff) as i32 - 128;
            rng = rng.wrapping_mul(0x5851f42d).wrapping_add(0xc4ceb9fe);
            let jy = ((rng >> 8) & 0xff) as i32 - 128;

            let cx = (cell_w / 2 + col * cell_w + jx.clamp(-(cell_w / 4), cell_w / 4))
                .clamp(5, ps - 5);
            let cy = (cell_h / 2 + row * cell_h + jy.clamp(-(cell_h / 4), cell_h / 4))
                .clamp(5, ps - 5);

            if let Some((sx, sy)) = find_solid_ground(loc.level(0), cx, cy, 20) {
                loc.spawn_objects.push((sx, sy, 0, Object::alien_runner()));
            }
        }
    }
}

/// Spiral outward from (cx, cy) to find the nearest walkable solid-ground tile.
fn find_solid_ground(level: &crate::level::Level, cx: i32, cy: i32, max_r: i32) -> Option<(i32, i32)> {
    (0..max_r).find_map(|r| {
        (-r..=r).find_map(|dy| {
            (-r..=r).find_map(|dx| {
                (dx.abs().max(dy.abs()) == r
                    && level.get(cx + dx, cy + dy).is_some_and(is_solid_ground))
                .then_some((cx + dx, cy + dy))
            })
        })
    })
}

/// Spiral outward from center to find solid walkable ground, clear a 3×3
/// landing pad, and stamp a ShipDock tile.
fn place_ship_dock(level: &mut crate::level::Level, fill: Tile) {
    let (cx, cy) = (PLANET_SIZE as i32 / 2, PLANET_SIZE as i32 / 2);
    let max = PLANET_SIZE as i32 - 1;

    'dock: for r in 0..50_i32 {
        for dy in -r..=r {
            for dx in -r..=r {
                if dx.abs().max(dy.abs()) == r {
                    let (sx, sy) = (cx + dx, cy + dy);
                    if level.walkable(sx, sy) && level.get(sx, sy).is_some_and(is_solid_ground) {
                        for py in (sy - 1).max(0)..=(sy + 1).min(max) {
                            for px in (sx - 1).max(0)..=(sx + 1).min(max) {
                                if !level.walkable(px, py) {
                                    level.set(px, py, fill);
                                }
                            }
                        }
                        level.set(sx, sy, Tile::ShipDock);
                        break 'dock;
                    }
                }
            }
        }
    }
}
