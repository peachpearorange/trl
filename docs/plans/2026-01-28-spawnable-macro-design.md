# Spawnable Macro Design

A `macro_rules!` macro for declarative entity spawning with pseudo-inheritance.

## Syntax

```rust
spawnable! {
    // Root variants (2-tuple): base components only
    (Root {}, (
        Transform::default(),
        Visibility::default(),
    )),

    // Delegating variants (3-tuple): variant, delegation, extra components
    (EnvObject { height: f32, collideable: bool },
     Root {},
     (
         Collider::new(collideable),
         Sprite { size: Vec2::new(1.0, height) },
     )),

    (Tree { height: f32, age: f32 },
     EnvObject { height: height, collideable: true },
     (
         Loot(&[WOOD, LEAVES]),
         TreeComponent { age: age },
     )),

    (OakTree { height: f32, age: f32, acorns: u32 },
     Tree { height: height, age: age },
     (
         OakMarker,
         AcornProducer { count: acorns },
     )),
}
```

## Generated Code

```rust
pub enum Spawnable {
    Root {},
    EnvObject { height: f32, collideable: bool },
    Tree { height: f32, age: f32 },
    OakTree { height: f32, age: f32, acorns: u32 },
}

impl Spawnable {
    pub fn spawn(self, commands: &mut Commands) -> Entity {
        match self {
            Spawnable::Root {} => {
                commands.spawn((
                    Transform::default(),
                    Visibility::default(),
                )).id()
            }
            Spawnable::EnvObject { height, collideable } => {
                commands.spawn((
                    // From Root
                    Transform::default(),
                    Visibility::default(),
                    // Own components
                    Collider::new(collideable),
                    Sprite { size: Vec2::new(1.0, height) },
                )).id()
            }
            Spawnable::Tree { height, age } => {
                commands.spawn((
                    // From Root (via EnvObject)
                    Transform::default(),
                    Visibility::default(),
                    // From EnvObject (height=height, collideable=true)
                    Collider::new(true),
                    Sprite { size: Vec2::new(1.0, height) },
                    // Own components
                    Loot(&[WOOD, LEAVES]),
                    TreeComponent { age: age },
                )).id()
            }
            Spawnable::OakTree { height, age, acorns } => {
                commands.spawn((
                    // From Root
                    Transform::default(),
                    Visibility::default(),
                    // From EnvObject
                    Collider::new(true),
                    Sprite { size: Vec2::new(1.0, height) },
                    // From Tree
                    Loot(&[WOOD, LEAVES]),
                    TreeComponent { age: age },
                    // Own components
                    OakMarker,
                    AcornProducer { count: acorns },
                )).id()
            }
        }
    }
}
```

## Usage

```rust
let tree = Spawnable::Tree { height: 5.0, age: 100.0 }.spawn(&mut commands);
let oak = Spawnable::OakTree { height: 8.0, age: 200.0, acorns: 5 }.spawn(&mut commands);
```

## Macro Implementation

Uses `macro_rules!` with:

1. Parse all variant definitions into a list
2. Build dependency graph (which variant delegates to which)
3. Recursively resolve each variant's full component list
4. Substitute field expressions when walking up the chain

## Design Constraints

- **Single inheritance only** - each variant has at most one parent
- **All parent fields must be specified** when delegating
- **Parent-first component insertion** - components from Root inserted first, then down the chain
- **Compile-time resolution** - full component list flattened at macro expansion
- **Field expressions** - variant fields accessible by name, simple arithmetic allowed
