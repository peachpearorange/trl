use noise::{Fbm, MultiFractal, NoiseFn, Perlin};

use crate::{
    entities::Object,
    galaxy::{Location, LocationId},
    level::{LocationType, Tile, carve_blob},
};

pub const ID_ALIEN_JUNGLE: LocationId  = (4, 0, 0);
pub const ID_CRYSTAL_CAVES: LocationId = (5, 0, 0);
pub const ID_ARCTIC_WASTE: LocationId  = (6, 0, 0);
pub const ID_DESERT_WORLD: LocationId  = (1, 1, 0);

/// Generate all proc-gen planets and return (id, location) pairs ready for
/// `galaxy.insert`.
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

pub const PLANET_SIZE: usize = 500;

/// How many cave entrances are scattered across the planet surface (4 cols × 2 rows).
const STAIR_GRID_COLS: i32 = 4;
const STAIR_GRID_ROWS: i32 = 2;

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
    /// 0–1: fraction of surface covered by fluid
    pub water_coverage: f32,
    /// 0–1: density of vegetation / flora tiles
    pub vegetation_density: f32,
    /// 0–1: frequency of rock / wall tiles
    pub rock_frequency: f32,
    pub seed: Option<u64>,
}

impl PlanetParams {
    pub fn grassland(name: &'static str) -> Self {
        Self {
            name,
            biome: PlanetBiome::Grassland,
            breathable: true,
            water_coverage: 0.3,
            vegetation_density: 0.5,
            rock_frequency: 0.1,
            seed: None,
        }
    }

    pub fn alien(name: &'static str) -> Self {
        Self {
            name,
            biome: PlanetBiome::Alien,
            breathable: false,
            water_coverage: 0.25,
            vegetation_density: 0.4,
            rock_frequency: 0.15,
            seed: None,
        }
    }

    pub fn lava(name: &'static str) -> Self {
        Self {
            name,
            biome: PlanetBiome::Lava,
            breathable: false,
            water_coverage: 0.35,
            vegetation_density: 0.0,
            rock_frequency: 0.4,
            seed: None,
        }
    }

    pub fn crystal(name: &'static str) -> Self {
        Self {
            name,
            biome: PlanetBiome::Crystal,
            breathable: false,
            water_coverage: 0.1,
            vegetation_density: 0.3,
            rock_frequency: 0.3,
            seed: None,
        }
    }

    pub fn arctic(name: &'static str) -> Self {
        Self {
            name,
            biome: PlanetBiome::Arctic,
            breathable: false,
            water_coverage: 0.2,
            vegetation_density: 0.0,
            rock_frequency: 0.25,
            seed: None,
        }
    }

    pub fn desert(name: &'static str) -> Self {
        Self {
            name,
            biome: PlanetBiome::Desert,
            breathable: false,
            water_coverage: 0.1,
            vegetation_density: 0.0,
            rock_frequency: 0.3,
            seed: None,
        }
    }

    pub fn with_seed(mut self, seed: u64) -> Self { self.seed = Some(seed); self }
    pub fn with_water(mut self, v: f32) -> Self { self.water_coverage = v; self }
    pub fn with_vegetation(mut self, v: f32) -> Self { self.vegetation_density = v; self }
    pub fn with_rocks(mut self, v: f32) -> Self { self.rock_frequency = v; self }
}

fn is_solid_ground(tile: Tile) -> bool {
    matches!(
        tile,
        Tile::Grass
            | Tile::TallGrass
            | Tile::Ash
            | Tile::CaveFloor
            | Tile::IceFloor
            | Tile::AlienSoil
            | Tile::AlienGrass
    )
}

fn biome_ground_tile(biome: PlanetBiome) -> Tile {
    match biome {
        PlanetBiome::Grassland => Tile::Grass,
        PlanetBiome::Desert    => Tile::Ash,
        PlanetBiome::Crystal   => Tile::CaveFloor,
        PlanetBiome::Alien     => Tile::AlienSoil,
        PlanetBiome::Arctic    => Tile::IceFloor,
        PlanetBiome::Lava      => Tile::Ash,
    }
}

fn biome_cave_wall(biome: PlanetBiome) -> Tile {
    match biome {
        PlanetBiome::Arctic => Tile::IceWall,
        _                   => Tile::CaveWall,
    }
}

fn biome_cave_floor(biome: PlanetBiome) -> Tile {
    match biome {
        PlanetBiome::Arctic => Tile::IceFloor,
        PlanetBiome::Lava   => Tile::Ash,
        _                   => Tile::CaveFloor,
    }
}

/// Maps noise values to a tile for the given biome and parameters.
///
/// `t` is low-frequency terrain elevation in [-1, 1] — controls zone (water/ground/rock).
/// `d` is higher-frequency detail in [-1, 1] — adds variation within the ground zone.
fn sample_tile(t: f64, d: f64, biome: PlanetBiome, wc: f64, vd: f64, rf: f64) -> Tile {
    // Zone thresholds derived from parameters:
    //   wc (water coverage) shifts water/ground boundary up → more area submerged
    //   rf (rock frequency) pulls rock threshold down       → more area is rock
    //   vd (vegetation density) lowers veg threshold        → more vegetation in ground
    let deep_thresh    = -0.65 + wc * 0.9;   // wc=0 → -0.65,  wc=1 → 0.25
    let shallow_thresh = deep_thresh + 0.25;
    let rock_thresh    = 0.55 - rf * 0.60;   // rf=0 → 0.55,   rf=1 → -0.05
    let veg_thresh     = 0.40 - vd * 0.80;   // vd=0 → 0.40,   vd=1 → -0.40

    match biome {
        PlanetBiome::Grassland => {
            if t < deep_thresh         { Tile::DeepWater }
            else if t < shallow_thresh { Tile::ShallowWater }
            else if t > rock_thresh    { Tile::Wall }
            else if d > veg_thresh     { Tile::TallGrass }
            else if d > veg_thresh - 0.3 { Tile::Bush }
            else                       { Tile::Grass }
        }
        PlanetBiome::Desert => {
            if t < deep_thresh         { Tile::AcidPool }
            else if t < shallow_thresh { Tile::AlienFluid }
            else if t > rock_thresh    { Tile::CaveWall }
            else if d > veg_thresh     { Tile::CaveFloor }
            else                       { Tile::Ash }
        }
        PlanetBiome::Crystal => {
            if t < deep_thresh         { Tile::AcidPool }
            else if t < shallow_thresh { Tile::BioluminescentPool }
            else if t > rock_thresh {
                if d > 0.0 { Tile::CrystalFormation } else { Tile::CaveWall }
            }
            else if d > veg_thresh     { Tile::Ash }
            else                       { Tile::CaveFloor }
        }
        PlanetBiome::Alien => {
            if t < deep_thresh         { Tile::BioluminescentPool }
            else if t < shallow_thresh { Tile::AlienFluid }
            else if t > rock_thresh    { Tile::CaveWall }
            else if d > veg_thresh     { Tile::AlienGrass }
            else                       { Tile::AlienSoil }
        }
        PlanetBiome::Arctic => {
            if t < deep_thresh         { Tile::DeepWater }
            else if t < shallow_thresh { Tile::ShallowWater }
            else if t > rock_thresh    { Tile::IceWall }
            else                       { Tile::IceFloor }
        }
        PlanetBiome::Lava => {
            if t < deep_thresh         { Tile::CrimsonPool }
            else if t < shallow_thresh { Tile::Lava }
            else if t > rock_thresh    { Tile::CaveWall }
            else                       { Tile::Ash }
        }
    }
}

/// Maps cave noise to a tile. Returns `None` to leave the cave wall in place.
/// `v` is the primary cave openness value — positive means open passage.
/// `d` is detail noise for biome-specific features inside the open areas.
fn sample_cave_tile(v: f64, d: f64, biome: PlanetBiome) -> Option<Tile> {
    if v <= 0.05 {
        return None; // solid wall
    }
    let floor = biome_cave_floor(biome);
    // Rare feature pockets inside open cave areas (only walkable tiles)
    let feature = match biome {
        PlanetBiome::Crystal if d > 0.55 => Some(Tile::BioluminescentPool),
        PlanetBiome::Alien   if d > 0.60 => Some(Tile::AlienFluid),
        PlanetBiome::Lava    if d > 0.65 => Some(Tile::CrimsonPool),
        PlanetBiome::Desert  if d > 0.70 => Some(Tile::AcidPool),
        PlanetBiome::Arctic  if d > 0.60 => Some(Tile::ShallowWater),
        _                                 => None,
    };
    Some(feature.unwrap_or(floor))
}

pub fn generate(params: &PlanetParams) -> Location {
    let seed = params.seed.unwrap_or_else(rand::random::<u64>);
    // Fold 64-bit seed to 32-bit for the noise crate.
    let seed32 = (seed ^ (seed >> 32)) as u32;

    // Low-frequency terrain noise — large, chunky zones (features span ~25 tiles).
    let terrain: Fbm<Perlin> = Fbm::new(seed32)
        .set_octaves(4)
        .set_frequency(0.04)
        .set_lacunarity(2.0)
        .set_persistence(0.5);

    // Higher-frequency detail noise for surface variation (~7-tile features).
    let detail: Fbm<Perlin> = Fbm::new(seed32.wrapping_add(0x9e37_79b9))
        .set_octaves(3)
        .set_frequency(0.15);

    // Cave noise — different seeds so underground never mirrors the surface.
    let cave_terrain: Fbm<Perlin> = Fbm::new(seed32.wrapping_add(0x4b7a_c9f2))
        .set_octaves(4)
        .set_frequency(0.04)
        .set_lacunarity(2.0)
        .set_persistence(0.5);

    let cave_detail: Fbm<Perlin> = Fbm::new(seed32.wrapping_add(0xd3e8_2a1c))
        .set_octaves(3)
        .set_frequency(0.15);

    let (wc, vd, rf) = (
        params.water_coverage as f64,
        params.vegetation_density as f64,
        params.rock_frequency as f64,
    );

    let fill = biome_ground_tile(params.biome);
    let cave_wall = biome_cave_wall(params.biome);

    let mut loc = Location::new(
        params.name,
        PLANET_SIZE,
        PLANET_SIZE,
        2,
        LocationType::PlanetSurface { breathable: params.breathable },
        fill,
    );

    // Overwrite cave level (z=1) with solid cave walls before carving it open.
    {
        let cave = loc.level_mut(1);
        for y in 0..PLANET_SIZE as i32 {
            for x in 0..PLANET_SIZE as i32 {
                cave.set(x, y, cave_wall);
            }
        }
    }

    // Generate surface (z=0).
    {
        let level = loc.level_mut(0);
        for y in 0..PLANET_SIZE as i32 {
            for x in 0..PLANET_SIZE as i32 {
                let t = terrain.get([x as f64, y as f64]).clamp(-1.0, 1.0);
                let d = detail.get([x as f64, y as f64]).clamp(-1.0, 1.0);
                level.set(x, y, sample_tile(t, d, params.biome, wc, vd, rf));
            }
        }
    }

    // Generate cave level (z=1): carve organic passages from solid rock.
    {
        let cave = loc.level_mut(1);
        for y in 0..PLANET_SIZE as i32 {
            for x in 0..PLANET_SIZE as i32 {
                let v = cave_terrain.get([x as f64, y as f64]).clamp(-1.0, 1.0);
                let d = cave_detail.get([x as f64, y as f64]).clamp(-1.0, 1.0);
                if let Some(tile) = sample_cave_tile(v, d, params.biome) {
                    cave.set(x, y, tile);
                }
            }
        }
    }

    place_ship_dock(loc.level_mut(0), fill);
    place_cave_stairs(&mut loc, params.biome, seed32);
    if matches!(params.biome, PlanetBiome::Alien) {
        scatter_hostiles(&mut loc, seed32);
    }
    loc
}

/// Scatter cave entrances across the surface in a STAIR_GRID_COLS × STAIR_GRID_ROWS grid.
/// Each entrance is a `StairsDown` tile on the surface paired with a `StairsUp` tile
/// underground, and an Elevator entity on both z-levels so the player can travel between them.
fn place_cave_stairs(loc: &mut Location, biome: PlanetBiome, seed32: u32) {
    let ps = PLANET_SIZE as i32;
    let cell_w = ps / STAIR_GRID_COLS;
    let cell_h = ps / STAIR_GRID_ROWS;
    let cave_floor = biome_cave_floor(biome);

    // Simple LCG for deterministic jitter within each grid cell.
    let mut rng = seed32.wrapping_mul(0x6c62272e).wrapping_add(0x07bb0142);

    let mut stair_positions: Vec<(i32, i32)> = Vec::new();

    for row in 0..STAIR_GRID_ROWS {
        for col in 0..STAIR_GRID_COLS {
            rng = rng.wrapping_mul(0x5851f42d).wrapping_add(0xc4ceb9fe);
            let jx = ((rng >> 8) & 0xff) as i32 - 128;
            rng = rng.wrapping_mul(0x5851f42d).wrapping_add(0xc4ceb9fe);
            let jy = ((rng >> 8) & 0xff) as i32 - 128;

            let cx = (cell_w / 2 + col * cell_w + jx.clamp(-(cell_w / 4), cell_w / 4))
                .clamp(5, ps - 5);
            let cy = (cell_h / 2 + row * cell_h + jy.clamp(-(cell_h / 4), cell_h / 4))
                .clamp(5, ps - 5);

            // Find walkable solid-ground near this grid center on the surface.
            let pos = find_solid_ground(loc.level(0), cx, cy, 40);
            if let Some((sx, sy)) = pos {
                // StairsDown visible on surface; StairsUp emerges from underground.
                loc.level_mut(0).set(sx, sy, Tile::StairsDown);

                // Guarantee open floor around the underground arrival point.
                carve_blob(loc.level_mut(1), sx, sy, 6, cave_floor);
                loc.level_mut(1).set(sx, sy, Tile::StairsUp);

                stair_positions.push((sx, sy));
            }
        }
    }

    // Spawn one Elevator entity per z-level per stair so players can go both ways.
    // Each elevator knows both floors: surface (z=0) and caves (z=1) at the same (x,y).
    for &(sx, sy) in &stair_positions {
        let floors = vec![(0usize, sx, sy), (1usize, sx, sy)];
        loc.spawn_objects.push((sx, sy, 0, Object::elevator(0, floors.clone())));
        loc.spawn_objects.push((sx, sy, 1, Object::elevator(1, floors)));
    }
}

/// Scatter alien_runner hostiles across the surface in a grid, one per cell with jitter.
fn scatter_hostiles(loc: &mut Location, seed32: u32) {
    const COLS: i32 = 10;
    const ROWS: i32 = 10;
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

            if let Some((sx, sy)) = find_solid_ground(loc.level(0), cx, cy, 30) {
                loc.spawn_objects.push((sx, sy, 0, Object::alien_runner()));
            }
        }
    }
}

/// Spiral outward from (cx, cy) to find the nearest tile that passes `is_solid_ground`.
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
