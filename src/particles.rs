//! Visual particle effects for gunshots and grenade explosions using bevy_hanabi.

use {bevy::prelude::*,
     bevy_hanabi::prelude::*,
     crate::{TILE_SIZE, level::{ZONE_WIDTH, ZONE_HEIGHT}}};

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// Cached effect-asset handles created at startup.
#[derive(Resource)]
pub struct ParticleEffects {
  /// Single bright dot: one spawned per tile along the bullet path.
  pub bullet_tracer: Handle<EffectAsset>,
  /// Impact spark burst at the hit point.
  pub bullet_spark:  Handle<EffectAsset>,
  /// Large orange-red explosion burst.
  pub explosion:     Handle<EffectAsset>,
  /// Tight cyan glow: one spawned per tile along the laser beam.
  pub laser_beam:    Handle<EffectAsset>,
}

/// Despawn this entity once the timer expires.
#[derive(Component)]
pub struct EffectLifetime(pub Timer);

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

pub struct ParticlesPlugin;

impl Plugin for ParticlesPlugin {
  fn build(&self, app: &mut App) {
    app
      .add_plugins(HanabiPlugin)
      .add_systems(Startup, setup_particle_effects)
      .add_systems(Update, tick_effect_lifetime);
  }
}

// ---------------------------------------------------------------------------
// Coordinate helper
// ---------------------------------------------------------------------------

/// Convert a grid cell to world-space Vec3 using the *actual* level dimensions.
/// Mirrors `tile_screen_pos` — must use the same `w`/`h` that the renderer uses,
/// not the hardcoded `ZONE_WIDTH`/`ZONE_HEIGHT` constants (which differ per location).
pub fn grid_world(x: i32, y: i32, w: usize, h: usize) -> Vec3 {
  Vec3::new(
    (x as f32 - w as f32 / 2.0) * TILE_SIZE,
    (h as f32 / 2.0 - y as f32) * TILE_SIZE,
    5.0,
  )
}

/// Like `grid_world` but accepts continuous tile-space coordinates, e.g. (3.7, 5.1).
pub fn tile_to_world(x: f32, y: f32, w: usize, h: usize) -> Vec3 {
  Vec3::new(
    (x - 0.5 - w as f32 / 2.0) * TILE_SIZE,
    (h as f32 / 2.0 - y + 0.5) * TILE_SIZE,
    5.0,
  )
}

// ---------------------------------------------------------------------------
// Startup: build effect assets
// ---------------------------------------------------------------------------

fn setup_particle_effects(mut commands: Commands, mut effects: ResMut<Assets<EffectAsset>>) {
  // --- Bullet tracer dot ---
  // Spawned once per tile on the bullet path. Very bright white flash.
  // Particles start on a tiny seed sphere so the velocity sphere direction is defined.
  let writer = ExprWriter::new();
  let age      = writer.lit(0.0_f32).expr();
  let lifetime = writer.lit(0.08_f32).uniform(writer.lit(0.14_f32)).expr();
  let p_center = writer.lit(Vec3::ZERO).expr();
  let p_radius = writer.lit(1.0_f32).expr();
  let v_center = writer.lit(Vec3::ZERO).expr();
  let speed    = writer.lit(10.0_f32).uniform(writer.lit(50.0_f32)).expr();

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
      .init(SetPositionSphereModifier { center: p_center, radius: p_radius, dimension: ShapeDimension::Surface })
      .init(SetVelocitySphereModifier { center: v_center, speed })
      .render(ColorOverLifetimeModifier::new(cg))
      .render(SizeOverLifetimeModifier { gradient: sg, screen_space_size: false })
  );

  // --- Bullet impact spark ---
  // Larger burst at the hit tile.
  let writer = ExprWriter::new();
  let age      = writer.lit(0.0_f32).expr();
  let lifetime = writer.lit(0.15_f32).uniform(writer.lit(0.30_f32)).expr();
  let p_center = writer.lit(Vec3::ZERO).expr();
  let p_radius = writer.lit(2.0_f32).expr();
  let v_center = writer.lit(Vec3::ZERO).expr();
  let speed    = writer.lit(120.0_f32).uniform(writer.lit(300.0_f32)).expr();

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
      .init(SetPositionSphereModifier { center: p_center, radius: p_radius, dimension: ShapeDimension::Surface })
      .init(SetVelocitySphereModifier { center: v_center, speed })
      .render(ColorOverLifetimeModifier::new(cg))
      .render(SizeOverLifetimeModifier { gradient: sg, screen_space_size: false })
  );

  // --- Grenade explosion ---
  // Massive burst of orange-red particles spread across the blast area.
  let writer = ExprWriter::new();
  let age      = writer.lit(0.0_f32).expr();
  let lifetime = writer.lit(0.35_f32).uniform(writer.lit(0.80_f32)).expr();
  let p_center = writer.lit(Vec3::ZERO).expr();
  let p_radius = writer.lit(TILE_SIZE * 2.0_f32).expr();
  let v_center = writer.lit(Vec3::ZERO).expr();
  let speed    = writer.lit(80.0_f32).uniform(writer.lit(500.0_f32)).expr();

  let mut cg: bevy_hanabi::Gradient<Vec4> = bevy_hanabi::Gradient::new();
  cg.add_key(0.0,  Vec4::new(1.0, 1.0, 0.5, 1.0));   // bright flash
  cg.add_key(0.12, Vec4::new(1.0, 0.6, 0.0, 1.0));   // orange
  cg.add_key(0.4,  Vec4::new(0.9, 0.15, 0.0, 0.9));  // red
  cg.add_key(0.75, Vec4::new(0.4, 0.05, 0.0, 0.5));  // dark red
  cg.add_key(1.0,  Vec4::new(0.1, 0.0, 0.0, 0.0));   // fade out

  let mut sg: bevy_hanabi::Gradient<Vec3> = bevy_hanabi::Gradient::new();
  sg.add_key(0.0,  Vec3::splat(22.0));   // start big
  sg.add_key(0.3,  Vec3::splat(16.0));
  sg.add_key(0.7,  Vec3::splat(8.0));
  sg.add_key(1.0,  Vec3::ZERO);

  let explosion = effects.add(
    EffectAsset::new(1024, SpawnerSettings::once(300.0_f32.into()), writer.finish())
      .with_name("explosion")
      .init(SetAttributeModifier::new(Attribute::AGE, age))
      .init(SetAttributeModifier::new(Attribute::LIFETIME, lifetime))
      .init(SetPositionSphereModifier { center: p_center, radius: p_radius, dimension: ShapeDimension::Volume })
      .init(SetVelocitySphereModifier { center: v_center, speed })
      .render(ColorOverLifetimeModifier::new(cg))
      .render(SizeOverLifetimeModifier { gradient: sg, screen_space_size: false })
  );

  // --- Laser beam glow ---
  // Tight cyan/white cluster per tile — very low drift so it looks like a solid beam.
  let writer = ExprWriter::new();
  let age      = writer.lit(0.0_f32).expr();
  let lifetime = writer.lit(0.15_f32).uniform(writer.lit(0.35_f32)).expr();
  let p_center = writer.lit(Vec3::ZERO).expr();
  let p_radius = writer.lit(2.0_f32).expr();
  let v_center = writer.lit(Vec3::ZERO).expr();
  let speed    = writer.lit(5.0_f32).uniform(writer.lit(25.0_f32)).expr();

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
      .init(SetPositionSphereModifier { center: p_center, radius: p_radius, dimension: ShapeDimension::Surface })
      .init(SetVelocitySphereModifier { center: v_center, speed })
      .render(ColorOverLifetimeModifier::new(cg))
      .render(SizeOverLifetimeModifier { gradient: sg, screen_space_size: false })
  );

  commands.insert_resource(ParticleEffects { bullet_tracer, bullet_spark, explosion, laser_beam });
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
      EffectLifetime(Timer::from_seconds(0.3, TimerMode::Once)),
    ));
  }
  if let Some(&(x, y)) = path.last() {
    commands.spawn((
      ParticleEffect::new(effects.bullet_spark.clone()),
      Transform::from_translation(grid_world(x, y, level_w, level_h)),
      EffectLifetime(Timer::from_seconds(0.6, TimerMode::Once)),
    ));
  }
}

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
      EffectLifetime(Timer::from_seconds(0.5, TimerMode::Once)),
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
    EffectLifetime(Timer::from_seconds(2.0, TimerMode::Once)),
  ));
}

// ---------------------------------------------------------------------------
// Lifecycle
// ---------------------------------------------------------------------------

fn tick_effect_lifetime(
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
