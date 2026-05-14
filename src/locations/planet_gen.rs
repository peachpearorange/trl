use noise::{Fbm, MultiFractal, NoiseFn, Perlin};

use crate::{
    galaxy::{Location, LocationId},
    level::{LocationType, Tile},
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

    let (wc, vd, rf) = (
        params.water_coverage as f64,
        params.vegetation_density as f64,
        params.rock_frequency as f64,
    );

    let fill = biome_ground_tile(params.biome);
    let mut loc = Location::new(
        params.name,
        PLANET_SIZE,
        PLANET_SIZE,
        1,
        LocationType::PlanetSurface { breathable: params.breathable },
        fill,
    );
    let level = loc.level_mut(0);

    for y in 0..PLANET_SIZE as i32 {
        for x in 0..PLANET_SIZE as i32 {
            let t = terrain.get([x as f64, y as f64]).clamp(-1.0, 1.0);
            let d = detail.get([x as f64, y as f64]).clamp(-1.0, 1.0);
            level.set(x, y, sample_tile(t, d, params.biome, wc, vd, rf));
        }
    }

    place_ship_dock(level, fill);
    loc
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
