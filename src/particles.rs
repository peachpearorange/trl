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
  pub bullet_spark: Handle<EffectAsset>,
  pub explosion:    Handle<EffectAsset>,
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

/// Convert a grid cell to world-space Vec3, above all sprites (sprites sit at z ≈ 2).
pub fn grid_world(x: i32, y: i32) -> Vec3 {
  Vec3::new(
    (x as f32 - ZONE_WIDTH as f32 / 2.0) * TILE_SIZE,
    (ZONE_HEIGHT as f32 / 2.0 - y as f32) * TILE_SIZE,
    5.0,
  )
}

// ---------------------------------------------------------------------------
// Startup: build effect assets
// ---------------------------------------------------------------------------

fn setup_particle_effects(mut commands: Commands, mut effects: ResMut<Assets<EffectAsset>>) {
  // --- Bullet impact sparks ---
  // Particles start spread over a tiny sphere so the velocity sphere direction is
  // always defined (normalize(pos - center) is non-zero).
  let writer = ExprWriter::new();

  let age      = writer.lit(0.0_f32).expr();
  let lifetime = writer.lit(0.1_f32).uniform(writer.lit(0.22_f32)).expr();
  let p_center = writer.lit(Vec3::ZERO).expr();
  let p_radius = writer.lit(2.0_f32).expr();           // 2 world-unit seed sphere (invisible)
  let v_center = writer.lit(Vec3::ZERO).expr();
  let speed    = writer.lit(80.0_f32).uniform(writer.lit(200.0_f32)).expr();

  let mut cg: bevy_hanabi::Gradient<Vec4> = bevy_hanabi::Gradient::new();
  cg.add_key(0.0, Vec4::new(1.0, 1.0, 0.5, 1.0));
  cg.add_key(0.5, Vec4::new(1.0, 0.5, 0.0, 1.0));
  cg.add_key(1.0, Vec4::new(0.4, 0.1, 0.0, 0.0));

  let bullet_spark = effects.add(
    EffectAsset::new(64, SpawnerSettings::once(24.0_f32.into()), writer.finish())
      .with_name("bullet_spark")
      .init(SetAttributeModifier::new(Attribute::AGE, age))
      .init(SetAttributeModifier::new(Attribute::LIFETIME, lifetime))
      .init(SetPositionSphereModifier { center: p_center, radius: p_radius, dimension: ShapeDimension::Surface })
      .init(SetVelocitySphereModifier { center: v_center, speed })
      .render(ColorOverLifetimeModifier::new(cg))
  );

  // --- Grenade explosion burst ---
  // Particles spread across the blast radius and fly outward.
  let writer = ExprWriter::new();

  let age      = writer.lit(0.0_f32).expr();
  let lifetime = writer.lit(0.25_f32).uniform(writer.lit(0.65_f32)).expr();
  let p_center = writer.lit(Vec3::ZERO).expr();
  let p_radius = writer.lit(TILE_SIZE * 2.2_f32).expr();
  let v_center = writer.lit(Vec3::ZERO).expr();
  let speed    = writer.lit(30.0_f32).uniform(writer.lit(180.0_f32)).expr();

  let mut cg: bevy_hanabi::Gradient<Vec4> = bevy_hanabi::Gradient::new();
  cg.add_key(0.0,  Vec4::new(1.0, 0.9, 0.2, 1.0));
  cg.add_key(0.25, Vec4::new(1.0, 0.4, 0.0, 1.0));
  cg.add_key(0.6,  Vec4::new(0.8, 0.1, 0.0, 0.8));
  cg.add_key(1.0,  Vec4::new(0.2, 0.05, 0.0, 0.0));

  let explosion = effects.add(
    EffectAsset::new(512, SpawnerSettings::once(96.0_f32.into()), writer.finish())
      .with_name("explosion")
      .init(SetAttributeModifier::new(Attribute::AGE, age))
      .init(SetAttributeModifier::new(Attribute::LIFETIME, lifetime))
      .init(SetPositionSphereModifier { center: p_center, radius: p_radius, dimension: ShapeDimension::Volume })
      .init(SetVelocitySphereModifier { center: v_center, speed })
      .render(ColorOverLifetimeModifier::new(cg))
  );

  commands.insert_resource(ParticleEffects { bullet_spark, explosion });
}

// ---------------------------------------------------------------------------
// Public helpers — called directly from ability / combat systems
// ---------------------------------------------------------------------------

/// Spawn a brief spark burst at the given grid tile (gun hit point).
pub fn spawn_bullet_spark(commands: &mut Commands, effects: &ParticleEffects, at: (i32, i32)) {
  commands.spawn((
    ParticleEffect::new(effects.bullet_spark.clone()),
    Transform::from_translation(grid_world(at.0, at.1)),
    EffectLifetime(Timer::from_seconds(0.5, TimerMode::Once)),
  ));
}

/// Spawn a large explosion burst centered on the given grid tile.
pub fn spawn_explosion_burst(
  commands: &mut Commands,
  effects: &ParticleEffects,
  at: (i32, i32)
) {
  commands.spawn((
    ParticleEffect::new(effects.explosion.clone()),
    Transform::from_translation(grid_world(at.0, at.1)),
    EffectLifetime(Timer::from_seconds(1.5, TimerMode::Once)),
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
