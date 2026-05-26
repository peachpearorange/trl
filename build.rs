use {ghx_proc_gen::{generator::{RngMode,
                                builder::GeneratorBuilder,
                                model::ModelCollection,
                                rules::RulesBuilder,
                                socket::{SocketCollection, SocketsCartesian2D}},
                    ghx_grid::cartesian::{coordinates::Cartesian2D,
                                          grid::CartesianGrid}},
     std::{env, fs, path::Path}};

const PLANET_SIZE: u32 = 300;
const SEED: u64 = 0xDEAD_BEEF;

fn scaled(param: f32, scale: f32) -> f32 { (param * scale).max(0.05) }

fn solve_grid(
  setup: impl FnOnce(&mut SocketCollection, &mut ModelCollection<Cartesian2D>)
) -> Vec<u8> {
  let mut sockets = SocketCollection::new();
  let mut models = ModelCollection::<Cartesian2D>::new();
  setup(&mut sockets, &mut models);

  let rules = RulesBuilder::new_cartesian_2d(models, sockets)
    .build()
    .expect("build.rs: rules build failed");
  let grid = CartesianGrid::new_cartesian_2d(PLANET_SIZE, PLANET_SIZE, false, false);
  let mut generator = GeneratorBuilder::new()
    .with_rules(rules)
    .with_grid(grid)
    .with_rng(RngMode::Seeded(SEED))
    .with_max_retry_count(100)
    .build()
    .expect("build.rs: generator build failed");

  let (_info, grid_data) =
    generator.generate_grid().expect("build.rs: generation failed");

  let mut indices = Vec::with_capacity((PLANET_SIZE * PLANET_SIZE) as usize);
  for y in 0..PLANET_SIZE {
    for x in 0..PLANET_SIZE {
      indices.push(grid_data.get_2d(x, y).model_index as u8);
    }
  }
  indices
}

fn alien(wc: f32, vd: f32, rf: f32) -> Vec<u8> {
  solve_grid(|sockets, models| {
    let ground = sockets.create();
    let feature = sockets.create();
    let shallow = sockets.create();
    let deep = sockets.create();
    let rock = sockets.create();
    sockets.add_connections([
      (ground, vec![ground, feature, shallow, rock]),
      (feature, vec![feature, ground]),
      (shallow, vec![shallow, deep, ground]),
      (deep, vec![deep, shallow]),
      (rock, vec![rock, ground])
    ]);
    models.create(SocketsCartesian2D::Mono(ground)).with_weight(10.0); // 0: AlienSoil
    models.create(SocketsCartesian2D::Mono(feature)).with_weight(scaled(vd, 8.0)); // 1: AlienGrass
    models.create(SocketsCartesian2D::Mono(shallow)).with_weight(scaled(wc, 5.0)); // 2: AlienFluid
    models.create(SocketsCartesian2D::Mono(deep)).with_weight(scaled(wc, 3.0)); // 3: BioluminescentPool
    models.create(SocketsCartesian2D::Mono(rock)).with_weight(scaled(rf, 5.0)); // 4: CaveWall
    models.create(SocketsCartesian2D::Mono(feature)).with_weight(0.4); // 5: alien_runner
    models.create(SocketsCartesian2D::Mono(ground)).with_weight(0.3); // 6: crab_alien
  })
}

fn crystal(wc: f32, vd: f32, rf: f32) -> Vec<u8> {
  solve_grid(|sockets, models| {
    let ground = sockets.create();
    let feature = sockets.create();
    let shallow = sockets.create();
    let deep = sockets.create();
    let rock = sockets.create();
    sockets.add_connections([
      (ground, vec![ground, feature, shallow, rock]),
      (feature, vec![feature, ground]),
      (shallow, vec![shallow, deep, ground]),
      (deep, vec![deep, shallow]),
      (rock, vec![rock, ground])
    ]);
    models.create(SocketsCartesian2D::Mono(ground)).with_weight(8.0);
    models.create(SocketsCartesian2D::Mono(rock)).with_weight(scaled(rf, 8.0));
    models.create(SocketsCartesian2D::Mono(feature)).with_weight(scaled(vd, 6.0));
    models.create(SocketsCartesian2D::Mono(feature)).with_weight(scaled(vd, 3.0));
    models.create(SocketsCartesian2D::Mono(shallow)).with_weight(scaled(wc, 3.0));
    models.create(SocketsCartesian2D::Mono(deep)).with_weight(scaled(wc, 2.0));
    models.create(SocketsCartesian2D::Mono(feature)).with_weight(0.35);
  })
}

fn arctic(wc: f32, rf: f32) -> Vec<u8> {
  solve_grid(|sockets, models| {
    let ground = sockets.create();
    let feature = sockets.create();
    let shallow = sockets.create();
    let deep = sockets.create();
    let rock = sockets.create();
    sockets.add_connections([
      (ground, vec![ground, feature, shallow, rock]),
      (feature, vec![feature, ground]),
      (shallow, vec![shallow, deep, ground]),
      (deep, vec![deep, shallow]),
      (rock, vec![rock, ground])
    ]);
    models.create(SocketsCartesian2D::Mono(ground)).with_weight(10.0);
    models.create(SocketsCartesian2D::Mono(rock)).with_weight(scaled(rf, 8.0));
    models.create(SocketsCartesian2D::Mono(shallow)).with_weight(scaled(wc, 6.0));
    models.create(SocketsCartesian2D::Mono(deep)).with_weight(scaled(wc, 3.0));
  })
}

fn desert(wc: f32, rf: f32) -> Vec<u8> {
  solve_grid(|sockets, models| {
    let ground = sockets.create();
    let feature = sockets.create();
    let shallow = sockets.create();
    let deep = sockets.create();
    let rock = sockets.create();
    sockets.add_connections([
      (ground, vec![ground, feature, shallow, rock]),
      (feature, vec![feature, ground]),
      (shallow, vec![shallow, deep, ground]),
      (deep, vec![deep, shallow]),
      (rock, vec![rock, ground])
    ]);
    models.create(SocketsCartesian2D::Mono(ground)).with_weight(10.0);
    models.create(SocketsCartesian2D::Mono(feature)).with_weight(scaled(rf, 5.0));
    models.create(SocketsCartesian2D::Mono(rock)).with_weight(scaled(rf, 8.0));
    models.create(SocketsCartesian2D::Mono(shallow)).with_weight(scaled(wc, 4.0));
    models.create(SocketsCartesian2D::Mono(deep)).with_weight(scaled(wc, 2.0));
  })
}

fn bright(wc: f32, rf: f32) -> Vec<u8> {
  solve_grid(|sockets, models| {
    let ground = sockets.create();
    let feature = sockets.create();
    let shallow = sockets.create();
    let deep = sockets.create();
    let rock = sockets.create();
    sockets.add_connections([
      (ground, vec![ground, feature, shallow, rock]),
      (feature, vec![feature, ground]),
      (shallow, vec![shallow, deep, ground]),
      (deep, vec![deep, shallow]),
      (rock, vec![rock, ground])
    ]);
    models.create(SocketsCartesian2D::Mono(ground)).with_weight(10.0);
    models.create(SocketsCartesian2D::Mono(rock)).with_weight(scaled(rf, 8.0));
    models.create(SocketsCartesian2D::Mono(shallow)).with_weight(scaled(wc, 5.0));
    models.create(SocketsCartesian2D::Mono(deep)).with_weight(scaled(wc, 2.0));
    models.create(SocketsCartesian2D::Mono(ground)).with_weight(0.4);
    models.create(SocketsCartesian2D::Mono(ground)).with_weight(0.1);
  })
}

fn lava(rf: f32) -> Vec<u8> {
  solve_grid(|sockets, models| {
    let s_wall = sockets.create();
    let s_edge = sockets.create();
    let s_run = sockets.create();
    sockets.add_connections([
      (s_wall, vec![s_wall, s_edge]),
      (s_edge, vec![s_wall, s_edge, s_run]),
      (s_run, vec![s_run, s_edge])
    ]);
    // Rock
    models.create(SocketsCartesian2D::Mono(s_wall)).with_weight(3.0);
    // Edge
    models.create(SocketsCartesian2D::Mono(s_edge)).with_weight(1.5);
    // Straight corridor
    models
      .create(SocketsCartesian2D::Simple {
        x_pos: s_run,
        x_neg: s_run,
        y_pos: s_edge,
        y_neg: s_edge
      })
      .with_weight(5.0);
    // Corner
    models
      .create(SocketsCartesian2D::Simple {
        x_pos: s_run,
        x_neg: s_edge,
        y_pos: s_run,
        y_neg: s_edge
      })
      .with_weight(0.1)
      .with_all_rotations();
    // T-junction
    models
      .create(SocketsCartesian2D::Simple {
        x_pos: s_run,
        x_neg: s_run,
        y_pos: s_run,
        y_neg: s_edge
      })
      .with_weight(0.5)
      .with_all_rotations();
    // Cross
    models.create(SocketsCartesian2D::Mono(s_run)).with_weight(0.5);
    // Wall patches
    models.create(SocketsCartesian2D::Mono(s_wall)).with_weight(scaled(rf, 4.0));
  })
}

fn main() {
  let out = env::var("OUT_DIR").unwrap();
  let out = Path::new(&out);

  let grids: &[(&str, Vec<u8>)] = &[
    ("alien", alien(0.35, 0.6, 0.15)),
    ("crystal", crystal(0.1, 0.4, 0.4)),
    ("arctic", arctic(0.25, 0.3)),
    ("desert", desert(0.08, 0.35)),
    ("bright", bright(0.15, 0.3)),
    ("lava", lava(0.5))
  ];

  for (name, data) in grids {
    fs::write(out.join(format!("planet_{name}.bin")), data).unwrap();
  }
}
