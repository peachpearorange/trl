//! Visual particle effects for gunshots and grenade explosions using bevy_hanabi.

use {crate::{TILE_SIZE, entities::Location, CurrentZone},
     bevy::prelude::*,
     bevy_hanabi::prelude::*};

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// Cached effect-asset handles created at startup.
#[derive(Resource)]
pub struct ParticleEffects {
  /// Single particle that travels from the gun muzzle to the aim point.
  pub gun_bullet: Handle<EffectAsset>,
  /// Single bright dot: one spawned per tile along the bullet path.
  pub bullet_tracer: Handle<EffectAsset>,
  /// Impact spark burst at the hit point.
  pub bullet_spark: Handle<EffectAsset>,
  /// Large orange-red explosion burst.
  pub explosion: Handle<EffectAsset>,
  /// Tight cyan glow: one spawned per tile along the laser beam.
  pub laser_beam: Handle<EffectAsset>,
  /// Bursty green plasma bolt.
  pub plasma_bolt: Handle<EffectAsset>,
  /// Small red-orange scatter pellet.
  pub scatter_pellet: Handle<EffectAsset>,
  /// Heavy purple pulse beam segment.
  pub pulse_beam: Handle<EffectAsset>
}

/// Despawn this entity once the timer expires.
#[derive(Component)]
pub struct EffectLifetime(pub Timer);

/// Sim-step driven projectile. Hanabi emitter is a separate entity (see `GunBulletVisual`)
/// spawned after the first advance so it starts away from the shooter tile.
#[derive(Component)]
pub struct GunBullet {
  pub dir: Vec2,
  pub pos: Vec2,
  pub target: Vec2,
  pub tiles_per_turn: f32,
  pub damage: i32,
  pub is_player: bool,
  pub z: usize,
  pub emitter: Option<Entity>
}

#[derive(Component)]
pub struct GunBulletVisual {
  pub dest: Vec3,
  pub speed: f32
}

#[derive(Component)]
pub struct PendingImpact {
  pub effect: Handle<EffectAsset>,
  pub pos: Vec3
}

pub const BULLET_TILES_PER_TURN: f32 = 8.0;

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

pub struct ParticlesPlugin;

impl Plugin for ParticlesPlugin {
  fn build(&self, app: &mut App) {
    app
      .add_plugins(HanabiPlugin)
      .add_systems(Startup, setup_particle_effects)
;
  }
}

// ---------------------------------------------------------------------------
// Coordinate helper
// ---------------------------------------------------------------------------

/// Convert a grid cell to world-space Vec3 using the *actual* level dimensions.
/// Mirrors `tile_screen_pos` — must use the same `w`/`h` that the renderer uses,
/// not hardcoded zone-size constants (which differ per location).
pub fn grid_world(x: i32, y: i32, w: usize, h: usize) -> Vec3 {
  Vec3::new(
    (x as f32 - w as f32 / 2.0) * TILE_SIZE,
    (h as f32 / 2.0 - y as f32) * TILE_SIZE,
    5.0
  )
}

/// Like `grid_world` but accepts continuous tile-space coordinates, e.g. (3.7, 5.1).
pub fn tile_to_world(x: f32, y: f32, w: usize, h: usize) -> Vec3 {
  Vec3::new(
    (x - 0.5 - w as f32 / 2.0) * TILE_SIZE,
    (h as f32 / 2.0 - y + 0.5) * TILE_SIZE,
    5.0
  )
}

// ---------------------------------------------------------------------------
// Startup: build effect assets
// ---------------------------------------------------------------------------

fn setup_particle_effects(
  mut commands: Commands,
  mut effects: ResMut<Assets<EffectAsset>>
) {
  // --- Gun bullet ---
  // Moving emitter: a GunBullet entity travels muzzle→aim (see move_gun_bullets).
  // Particles flash briefly in place as the emitter sweeps through — no per-particle velocity.
  let writer = ExprWriter::new();
  let age = writer.lit(0.0_f32).expr();
  let lifetime = writer.lit(0.08_f32).uniform(writer.lit(0.18_f32)).expr();
  let p_center = writer.lit(Vec3::ZERO).expr();
  let p_radius = writer.lit(0.0_f32).expr();
  let module = writer.finish();

  let mut cg: bevy_hanabi::Gradient<Vec4> = bevy_hanabi::Gradient::new();
  cg.add_key(0.0, Vec4::new(1.0, 1.0, 0.9, 1.0));
  cg.add_key(0.4, Vec4::new(1.0, 0.9, 0.3, 1.0));
  cg.add_key(1.0, Vec4::new(1.0, 0.5, 0.0, 0.0));

  let mut sg: bevy_hanabi::Gradient<Vec3> = bevy_hanabi::Gradient::new();
  sg.add_key(0.0, Vec3::splat(40.0));
  sg.add_key(1.0, Vec3::splat(15.0));

  let gun_bullet = effects.add(
    EffectAsset::new(16, SpawnerSettings::rate(60.0_f32.into()), module)
      .with_name("gun_bullet")
      .with_motion_integration(MotionIntegration::None)
      .init(SetAttributeModifier::new(Attribute::AGE, age))
      .init(SetAttributeModifier::new(Attribute::LIFETIME, lifetime))
      .init(SetPositionSphereModifier { center: p_center, radius: p_radius, dimension: ShapeDimension::Volume })
      .render(ColorOverLifetimeModifier::new(cg))
      .render(SizeOverLifetimeModifier { gradient: sg, screen_space_size: false })
  );

  // --- Bullet tracer dot ---
  // Spawned once per tile on the bullet path. Very bright white flash.
  // Particles start on a tiny seed sphere so the velocity sphere direction is defined.
  let writer = ExprWriter::new();
  let age = writer.lit(0.0_f32).expr();
  let lifetime = writer.lit(0.08_f32).uniform(writer.lit(0.14_f32)).expr();
  let p_center = writer.lit(Vec3::ZERO).expr();
  let p_radius = writer.lit(1.0_f32).expr();
  let v_center = writer.lit(Vec3::ZERO).expr();
  let speed = writer.lit(10.0_f32).uniform(writer.lit(50.0_f32)).expr();

  let mut cg: bevy_hanabi::Gradient<Vec4> = bevy_hanabi::Gradient::new();
  cg.add_key(0.0, Vec4::new(1.0, 1.0, 0.9, 1.0));
  cg.add_key(0.6, Vec4::new(1.0, 0.9, 0.3, 0.9));
  cg.add_key(1.0, Vec4::new(0.8, 0.5, 0.0, 0.0));

  let mut sg: bevy_hanabi::Gradient<Vec3> = bevy_hanabi::Gradient::new();
  sg.add_key(0.0, Vec3::splat(12.0));
  sg.add_key(0.5, Vec3::splat(8.0));
  sg.add_key(1.0, Vec3::ZERO);

  let bullet_tracer = effects.add(
    EffectAsset::new(32, SpawnerSettings::once(4.0_f32.into()), writer.finish())
      .with_name("bullet_tracer")
      .init(SetAttributeModifier::new(Attribute::AGE, age))
      .init(SetAttributeModifier::new(Attribute::LIFETIME, lifetime))
      .init(SetPositionSphereModifier {
        center: p_center,
        radius: p_radius,
        dimension: ShapeDimension::Surface
      })
      .init(SetVelocitySphereModifier { center: v_center, speed })
      .render(ColorOverLifetimeModifier::new(cg))
      .render(SizeOverLifetimeModifier { gradient: sg, screen_space_size: false })
  );

  // --- Bullet impact spark ---
  // Larger burst at the hit tile.
  let writer = ExprWriter::new();
  let age = writer.lit(0.0_f32).expr();
  let lifetime = writer.lit(0.15_f32).uniform(writer.lit(0.30_f32)).expr();
  let p_center = writer.lit(Vec3::ZERO).expr();
  let p_radius = writer.lit(2.0_f32).expr();
  let v_center = writer.lit(Vec3::ZERO).expr();
  let speed = writer.lit(120.0_f32).uniform(writer.lit(300.0_f32)).expr();

  let mut cg: bevy_hanabi::Gradient<Vec4> = bevy_hanabi::Gradient::new();
  cg.add_key(0.0, Vec4::new(1.0, 1.0, 0.8, 1.0));
  cg.add_key(0.3, Vec4::new(1.0, 0.6, 0.0, 1.0));
  cg.add_key(1.0, Vec4::new(0.5, 0.1, 0.0, 0.0));

  let mut sg: bevy_hanabi::Gradient<Vec3> = bevy_hanabi::Gradient::new();
  sg.add_key(0.0, Vec3::splat(14.0));
  sg.add_key(0.4, Vec3::splat(8.0));
  sg.add_key(1.0, Vec3::ZERO);

  let bullet_spark = effects.add(
    EffectAsset::new(64, SpawnerSettings::once(28.0_f32.into()), writer.finish())
      .with_name("bullet_spark")
      .init(SetAttributeModifier::new(Attribute::AGE, age))
      .init(SetAttributeModifier::new(Attribute::LIFETIME, lifetime))
      .init(SetPositionSphereModifier {
        center: p_center,
        radius: p_radius,
        dimension: ShapeDimension::Surface
      })
      .init(SetVelocitySphereModifier { center: v_center, speed })
      .render(ColorOverLifetimeModifier::new(cg))
      .render(SizeOverLifetimeModifier { gradient: sg, screen_space_size: false })
  );

  // --- Grenade explosion ---
  // Massive burst of orange-red particles spread across the blast area.
  let writer = ExprWriter::new();
  let age = writer.lit(0.0_f32).expr();
  let lifetime = writer.lit(0.35_f32).uniform(writer.lit(0.80_f32)).expr();
  let p_center = writer.lit(Vec3::ZERO).expr();
  let p_radius = writer.lit(TILE_SIZE * 2.0_f32).expr();
  let v_center = writer.lit(Vec3::ZERO).expr();
  let speed = writer.lit(80.0_f32).uniform(writer.lit(500.0_f32)).expr();

  let mut cg: bevy_hanabi::Gradient<Vec4> = bevy_hanabi::Gradient::new();
  cg.add_key(0.0, Vec4::new(1.0, 1.0, 0.5, 1.0)); // bright flash
  cg.add_key(0.12, Vec4::new(1.0, 0.6, 0.0, 1.0)); // orange
  cg.add_key(0.4, Vec4::new(0.9, 0.15, 0.0, 0.9)); // red
  cg.add_key(0.75, Vec4::new(0.4, 0.05, 0.0, 0.5)); // dark red
  cg.add_key(1.0, Vec4::new(0.1, 0.0, 0.0, 0.0)); // fade out

  let mut sg: bevy_hanabi::Gradient<Vec3> = bevy_hanabi::Gradient::new();
  sg.add_key(0.0, Vec3::splat(22.0)); // start big
  sg.add_key(0.3, Vec3::splat(16.0));
  sg.add_key(0.7, Vec3::splat(8.0));
  sg.add_key(1.0, Vec3::ZERO);

  let explosion = effects.add(
    EffectAsset::new(1024, SpawnerSettings::once(300.0_f32.into()), writer.finish())
      .with_name("explosion")
      .init(SetAttributeModifier::new(Attribute::AGE, age))
      .init(SetAttributeModifier::new(Attribute::LIFETIME, lifetime))
      .init(SetPositionSphereModifier {
        center: p_center,
        radius: p_radius,
        dimension: ShapeDimension::Volume
      })
      .init(SetVelocitySphereModifier { center: v_center, speed })
      .render(ColorOverLifetimeModifier::new(cg))
      .render(SizeOverLifetimeModifier { gradient: sg, screen_space_size: false })
  );

  // --- Laser beam glow ---
  // Tight cyan/white cluster per tile — very low drift so it looks like a solid beam.
  let writer = ExprWriter::new();
  let age = writer.lit(0.0_f32).expr();
  let lifetime = writer.lit(0.15_f32).uniform(writer.lit(0.35_f32)).expr();
  let p_center = writer.lit(Vec3::ZERO).expr();
  let p_radius = writer.lit(2.0_f32).expr();
  let v_center = writer.lit(Vec3::ZERO).expr();
  let speed = writer.lit(5.0_f32).uniform(writer.lit(25.0_f32)).expr();

  let mut cg: bevy_hanabi::Gradient<Vec4> = bevy_hanabi::Gradient::new();
  cg.add_key(0.0, Vec4::new(0.8, 1.0, 1.0, 1.0));
  cg.add_key(0.3, Vec4::new(0.0, 0.9, 1.0, 1.0));
  cg.add_key(0.7, Vec4::new(0.0, 0.5, 0.8, 0.6));
  cg.add_key(1.0, Vec4::new(0.0, 0.2, 0.5, 0.0));

  let mut sg: bevy_hanabi::Gradient<Vec3> = bevy_hanabi::Gradient::new();
  sg.add_key(0.0, Vec3::splat(18.0));
  sg.add_key(0.4, Vec3::splat(12.0));
  sg.add_key(1.0, Vec3::ZERO);

  let laser_beam = effects.add(
    EffectAsset::new(64, SpawnerSettings::once(16.0_f32.into()), writer.finish())
      .with_name("laser_beam")
      .init(SetAttributeModifier::new(Attribute::AGE, age))
      .init(SetAttributeModifier::new(Attribute::LIFETIME, lifetime))
      .init(SetPositionSphereModifier {
        center: p_center,
        radius: p_radius,
        dimension: ShapeDimension::Surface
      })
      .init(SetVelocitySphereModifier { center: v_center, speed })
      .render(ColorOverLifetimeModifier::new(cg))
      .render(SizeOverLifetimeModifier { gradient: sg, screen_space_size: false })
  );

  // --- Plasma bolt (green, bursty) ---
  let writer = ExprWriter::new();
  let age = writer.lit(0.0_f32).expr();
  let lifetime = writer.lit(0.1_f32).uniform(writer.lit(0.25_f32)).expr();
  let p_center = writer.lit(Vec3::ZERO).expr();
  let p_radius = writer.lit(3.0_f32).expr();
  let v_center = writer.lit(Vec3::ZERO).expr();
  let speed = writer.lit(30.0_f32).uniform(writer.lit(80.0_f32)).expr();

  let mut cg: bevy_hanabi::Gradient<Vec4> = bevy_hanabi::Gradient::new();
  cg.add_key(0.0, Vec4::new(0.7, 1.0, 0.8, 1.0));
  cg.add_key(0.3, Vec4::new(0.2, 1.0, 0.3, 1.0));
  cg.add_key(0.7, Vec4::new(0.0, 0.6, 0.1, 0.6));
  cg.add_key(1.0, Vec4::new(0.0, 0.3, 0.0, 0.0));

  let mut sg: bevy_hanabi::Gradient<Vec3> = bevy_hanabi::Gradient::new();
  sg.add_key(0.0, Vec3::splat(16.0));
  sg.add_key(0.3, Vec3::splat(12.0));
  sg.add_key(1.0, Vec3::ZERO);

  let plasma_bolt = effects.add(
    EffectAsset::new(64, SpawnerSettings::once(8.0_f32.into()), writer.finish())
      .with_name("plasma_bolt")
      .init(SetAttributeModifier::new(Attribute::AGE, age))
      .init(SetAttributeModifier::new(Attribute::LIFETIME, lifetime))
      .init(SetPositionSphereModifier {
        center: p_center,
        radius: p_radius,
        dimension: ShapeDimension::Surface
      })
      .init(SetVelocitySphereModifier { center: v_center, speed })
      .render(ColorOverLifetimeModifier::new(cg))
      .render(SizeOverLifetimeModifier { gradient: sg, screen_space_size: false })
  );

  // --- Scatter pellet (red-orange, small fast) ---
  let writer = ExprWriter::new();
  let age = writer.lit(0.0_f32).expr();
  let lifetime = writer.lit(0.06_f32).uniform(writer.lit(0.12_f32)).expr();
  let p_center = writer.lit(Vec3::ZERO).expr();
  let p_radius = writer.lit(1.0_f32).expr();
  let v_center = writer.lit(Vec3::ZERO).expr();
  let speed = writer.lit(15.0_f32).uniform(writer.lit(60.0_f32)).expr();

  let mut cg: bevy_hanabi::Gradient<Vec4> = bevy_hanabi::Gradient::new();
  cg.add_key(0.0, Vec4::new(1.0, 0.8, 0.6, 1.0));
  cg.add_key(0.5, Vec4::new(1.0, 0.3, 0.1, 0.9));
  cg.add_key(1.0, Vec4::new(0.5, 0.1, 0.0, 0.0));

  let mut sg: bevy_hanabi::Gradient<Vec3> = bevy_hanabi::Gradient::new();
  sg.add_key(0.0, Vec3::splat(8.0));
  sg.add_key(0.5, Vec3::splat(5.0));
  sg.add_key(1.0, Vec3::ZERO);

  let scatter_pellet = effects.add(
    EffectAsset::new(32, SpawnerSettings::once(3.0_f32.into()), writer.finish())
      .with_name("scatter_pellet")
      .init(SetAttributeModifier::new(Attribute::AGE, age))
      .init(SetAttributeModifier::new(Attribute::LIFETIME, lifetime))
      .init(SetPositionSphereModifier {
        center: p_center,
        radius: p_radius,
        dimension: ShapeDimension::Surface
      })
      .init(SetVelocitySphereModifier { center: v_center, speed })
      .render(ColorOverLifetimeModifier::new(cg))
      .render(SizeOverLifetimeModifier { gradient: sg, screen_space_size: false })
  );

  // --- Pulse beam (purple, heavy) ---
  let writer = ExprWriter::new();
  let age = writer.lit(0.0_f32).expr();
  let lifetime = writer.lit(0.2_f32).uniform(writer.lit(0.5_f32)).expr();
  let p_center = writer.lit(Vec3::ZERO).expr();
  let p_radius = writer.lit(3.0_f32).expr();
  let v_center = writer.lit(Vec3::ZERO).expr();
  let speed = writer.lit(5.0_f32).uniform(writer.lit(20.0_f32)).expr();

  let mut cg: bevy_hanabi::Gradient<Vec4> = bevy_hanabi::Gradient::new();
  cg.add_key(0.0, Vec4::new(0.9, 0.7, 1.0, 1.0));
  cg.add_key(0.3, Vec4::new(0.7, 0.2, 1.0, 1.0));
  cg.add_key(0.7, Vec4::new(0.4, 0.0, 0.7, 0.6));
  cg.add_key(1.0, Vec4::new(0.2, 0.0, 0.4, 0.0));

  let mut sg: bevy_hanabi::Gradient<Vec3> = bevy_hanabi::Gradient::new();
  sg.add_key(0.0, Vec3::splat(22.0));
  sg.add_key(0.3, Vec3::splat(16.0));
  sg.add_key(1.0, Vec3::ZERO);

  let pulse_beam = effects.add(
    EffectAsset::new(64, SpawnerSettings::once(16.0_f32.into()), writer.finish())
      .with_name("pulse_beam")
      .init(SetAttributeModifier::new(Attribute::AGE, age))
      .init(SetAttributeModifier::new(Attribute::LIFETIME, lifetime))
      .init(SetPositionSphereModifier {
        center: p_center,
        radius: p_radius,
        dimension: ShapeDimension::Surface
      })
      .init(SetVelocitySphereModifier { center: v_center, speed })
      .render(ColorOverLifetimeModifier::new(cg))
      .render(SizeOverLifetimeModifier { gradient: sg, screen_space_size: false })
  );

  commands.insert_resource(ParticleEffects {
    gun_bullet,
    bullet_tracer,
    bullet_spark,
    explosion,
    laser_beam,
    plasma_bolt,
    scatter_pellet,
    pulse_beam
  });
}

// ---------------------------------------------------------------------------
// Public helpers — called directly from ability / combat systems
// ---------------------------------------------------------------------------

/// Spawn a visible bullet trail along the bresenham path (skipping the shooter tile)
/// plus an impact spark at the end.
/// `level_w`/`level_h` must be the actual dimensions of the current level.
pub fn spawn_bullet_trail(
  commands: &mut Commands,
  effects: &ParticleEffects,
  path: &[(i32, i32)],
  level_w: usize,
  level_h: usize
) {
  for &(x, y) in path.iter().skip(1) {
    commands.spawn((
      ParticleEffect::new(effects.bullet_tracer.clone()),
      Transform::from_translation(grid_world(x, y, level_w, level_h)),
      EffectLifetime(Timer::from_seconds(0.3, TimerMode::Once))
    ));
  }
  if let Some(&(x, y)) = path.last() {
    commands.spawn((
      ParticleEffect::new(effects.bullet_spark.clone()),
      Transform::from_translation(grid_world(x, y, level_w, level_h)),
      EffectLifetime(Timer::from_seconds(0.6, TimerMode::Once))
    ));
  }
}

pub fn spawn_gun_bullet(
  commands: &mut Commands,
  effects: &ParticleEffects,
  ex: i32,
  ey: i32,
  aim_x: f32,
  aim_y: f32,
  damage: i32,
  is_player: bool,
  z: usize,
  level_w: usize,
  level_h: usize
) {
  let pos = Vec2::new(ex as f32 + 0.5, ey as f32 + 0.5);
  let target = Vec2::new(aim_x, aim_y);
  let diff = target - pos;
  if diff.length() < 0.1 { return; }
  let start_world = tile_to_world(pos.x, pos.y, level_w, level_h);
  let emitter = commands.spawn((
    ParticleEffect::new(effects.gun_bullet.clone()),
    GunBulletVisual { dest: start_world, speed: VISUAL_SPEED },
    Transform::from_translation(start_world)
  )).id();
  commands.spawn(GunBullet {
    dir: diff.normalize(),
    pos,
    target,
    tiles_per_turn: BULLET_TILES_PER_TURN,
    damage,
    is_player,
    z,
    emitter: Some(emitter)
  });
}

const VISUAL_SPEED: f32 = 500.0;

/// Spawn a cyan laser beam flash as a straight Euclidean line from `start` to `end`
/// (world-space positions). Emitters are placed every half-tile along the line.
pub fn spawn_laser_beam(
  commands: &mut Commands,
  effects: &ParticleEffects,
  start: Vec3,
  end: Vec3
) {
  let diff = end - start;
  let length = diff.truncate().length();
  let steps = ((length / (TILE_SIZE * 0.5)).ceil() as usize).max(1);
  for i in 0..=steps {
    let t = i as f32 / steps as f32;
    let pos = start.lerp(end, t);
    commands.spawn((
      ParticleEffect::new(effects.laser_beam.clone()),
      Transform::from_translation(pos),
      EffectLifetime(Timer::from_seconds(0.5, TimerMode::Once))
    ));
  }
}

/// Spawn green plasma bolt trail along a bresenham path, with a triple-burst at the endpoint.
pub fn spawn_plasma_burst(
  commands: &mut Commands,
  effects: &ParticleEffects,
  path: &[(i32, i32)],
  level_w: usize,
  level_h: usize
) {
  for &(x, y) in path.iter().skip(1) {
    commands.spawn((
      ParticleEffect::new(effects.plasma_bolt.clone()),
      Transform::from_translation(grid_world(x, y, level_w, level_h)),
      EffectLifetime(Timer::from_seconds(0.2, TimerMode::Once))
    ));
  }
  if let Some(&(x, y)) = path.last() {
    for _ in 0..3 {
      commands.spawn((
        ParticleEffect::new(effects.plasma_bolt.clone()),
        Transform::from_translation(grid_world(x, y, level_w, level_h)),
        EffectLifetime(Timer::from_seconds(0.5, TimerMode::Once))
      ));
    }
  }
}

/// Spawn red-orange scatter pellet trails along multiple spread paths.
pub fn spawn_scatter_trails(
  commands: &mut Commands,
  effects: &ParticleEffects,
  paths: &[Vec<(i32, i32)>],
  level_w: usize,
  level_h: usize
) {
  for path in paths {
    for &(x, y) in path.iter().skip(1) {
      commands.spawn((
        ParticleEffect::new(effects.scatter_pellet.clone()),
        Transform::from_translation(grid_world(x, y, level_w, level_h)),
        EffectLifetime(Timer::from_seconds(0.25, TimerMode::Once))
      ));
    }
    if let Some(&(x, y)) = path.last() {
      commands.spawn((
        ParticleEffect::new(effects.bullet_spark.clone()),
        Transform::from_translation(grid_world(x, y, level_w, level_h)),
        EffectLifetime(Timer::from_seconds(0.4, TimerMode::Once))
      ));
    }
  }
}

/// Spawn a heavy purple pulse beam as a straight Euclidean line from `start` to `end`.
pub fn spawn_pulse_beam(
  commands: &mut Commands,
  effects: &ParticleEffects,
  start: Vec3,
  end: Vec3
) {
  let diff = end - start;
  let length = diff.truncate().length();
  let steps = ((length / (TILE_SIZE * 0.5)).ceil() as usize).max(1);
  for i in 0..=steps {
    let t = i as f32 / steps as f32;
    let pos = start.lerp(end, t);
    commands.spawn((
      ParticleEffect::new(effects.pulse_beam.clone()),
      Transform::from_translation(pos),
      EffectLifetime(Timer::from_seconds(0.7, TimerMode::Once))
    ));
  }
}

/// Spawn a large explosion burst centered on the given grid tile.
/// `level_w`/`level_h` must be the actual dimensions of the current level.
pub fn spawn_explosion_burst(
  commands: &mut Commands,
  effects: &ParticleEffects,
  at: (i32, i32),
  level_w: usize,
  level_h: usize
) {
  commands.spawn((
    ParticleEffect::new(effects.explosion.clone()),
    Transform::from_translation(grid_world(at.0, at.1, level_w, level_h)),
    EffectLifetime(Timer::from_seconds(2.0, TimerMode::Once))
  ));
}

pub fn spawn_liquid_splash(
  commands: &mut Commands,
  effects: &mut Assets<EffectAsset>,
  x: i32,
  y: i32,
  level_w: usize,
  level_h: usize,
  primary: [f32; 3],
  secondary: [f32; 3]
) {
  let writer = ExprWriter::new();
  let age = writer.lit(0.0_f32).expr();
  let lifetime = writer.lit(0.2_f32).uniform(writer.lit(0.5_f32)).expr();
  let p_center = writer.lit(Vec3::ZERO).expr();
  let p_radius = writer.lit(TILE_SIZE * 0.4).expr();
  let v_center = writer.lit(Vec3::ZERO).expr();
  let speed = writer.lit(40.0_f32).uniform(writer.lit(120.0_f32)).expr();

  let [pr, pg, pb] = primary;
  let [sr, sg_, sb] = secondary;
  let mut cg: bevy_hanabi::Gradient<Vec4> = bevy_hanabi::Gradient::new();
  cg.add_key(0.0, Vec4::new(sr, sg_, sb, 0.9));
  cg.add_key(0.4, Vec4::new(pr, pg, pb, 0.7));
  cg.add_key(1.0, Vec4::new(pr * 0.5, pg * 0.5, pb * 0.5, 0.0));

  let mut sz: bevy_hanabi::Gradient<Vec3> = bevy_hanabi::Gradient::new();
  sz.add_key(0.0, Vec3::splat(10.0));
  sz.add_key(0.5, Vec3::splat(6.0));
  sz.add_key(1.0, Vec3::ZERO);

  let handle = effects.add(
    EffectAsset::new(64, SpawnerSettings::once(16.0_f32.into()), writer.finish())
      .with_name("liquid_splash")
      .init(SetAttributeModifier::new(Attribute::AGE, age))
      .init(SetAttributeModifier::new(Attribute::LIFETIME, lifetime))
      .init(SetPositionSphereModifier {
        center: p_center,
        radius: p_radius,
        dimension: ShapeDimension::Surface
      })
      .init(SetVelocitySphereModifier { center: v_center, speed })
      .render(ColorOverLifetimeModifier::new(cg))
      .render(SizeOverLifetimeModifier { gradient: sz, screen_space_size: false })
  );

  commands.spawn((
    ParticleEffect::new(handle),
    Transform::from_translation(grid_world(x, y, level_w, level_h)),
    EffectLifetime(Timer::from_seconds(0.8, TimerMode::Once))
  ));
}

pub fn liquid_splash_on_move(
  mut commands: Commands,
  mut effects: ResMut<Assets<EffectAsset>>,
  current: Res<CurrentZone>,
  moved_q: Query<&Location, Changed<Location>>
) {
  for location in &moved_q {
    if let &Location::Coords { x, y, z, .. } = location
      && let level = current.0.level(z)
      && let Some(tile) = level.get(x, y)
      && tile.is_liquid()
    {
      let (primary, secondary) = tile.render_mode().colors();
      spawn_liquid_splash(&mut commands, &mut effects, x, y, level.width, level.height, primary, secondary);
    }
  }
}

// ---------------------------------------------------------------------------
// Lifecycle
// ---------------------------------------------------------------------------

pub fn move_gun_bullets(
  time: Res<Time>,
  mut commands: Commands,
  mut q: Query<(Entity, &GunBulletVisual, &mut Transform, Option<&PendingImpact>)>
) {
  for (entity, visual, mut tf, impact) in q.iter_mut() {
    let to_dest = visual.dest - tf.translation;
    let dist = to_dest.truncate().length();
    let step = visual.speed * time.delta_secs();
    if dist <= step {
      tf.translation = visual.dest;
      if let Some(impact) = impact {
        commands.spawn((
          ParticleEffect::new(impact.effect.clone()),
          Transform::from_translation(impact.pos),
          EffectLifetime(Timer::from_seconds(0.5, TimerMode::Once))
        ));
        commands.entity(entity).remove::<PendingImpact>();
        commands.entity(entity).insert(EffectLifetime(Timer::from_seconds(0.2, TimerMode::Once)));
      }
    } else {
      let dir = to_dest.truncate().normalize().extend(0.0);
      tf.translation += dir * step;
    }
  }
}

pub fn tick_effect_lifetime(
  mut commands: Commands,
  mut query: Query<(Entity, &mut EffectLifetime)>,
  time: Res<Time>
) {
  for (entity, mut dur) in query.iter_mut() {
    dur.0.tick(time.delta());
    if dur.0.just_finished() {
      commands.entity(entity).despawn();
    }
  }
}
