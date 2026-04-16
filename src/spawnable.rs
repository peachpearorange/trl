// src/spawnable.rs
//! Spawnable macro for declarative entity spawning with pseudo-inheritance.
//!
//! # Syntax
//!
//! ```ignore
//! spawnable! {
//!     // Root variant (2-tuple): no parent, base components only
//!     (VariantName { field: Type, ... }, (Component1, Component2, ...)),
//!
//!     // Delegating variant (3-tuple): inherits parent's components
//!     (ChildName { field: Type, ... }, ParentName { parent_field: expr, ... }, (ExtraComponents, ...)),
//! }
//! ```
//!
//! # Example
//!
//! ```ignore
//! spawnable! {
//!     (Root { x: i32 }, (Transform::default(),)),
//!     (Tree { x: i32, height: f32 }, Root { x: x }, (Sprite::new(height),)),
//!     (OakTree { x: i32, height: f32, age: u32 }, Tree { x: x, height: height }, (OakMarker,)),
//! }
//!
//! // Usage (location is any Component type):
//! Spawnable::OakTree { x: 10, height: 5.0, age: 100 }.spawn(&mut commands, location);
//! ```
//!
//! # Features
//!
//! - **Single inheritance**: Each variant can delegate to one parent
//! - **Multi-level inheritance**: Grandparent components are automatically included
//! - **Field expressions**: Parent field mappings can use arithmetic (e.g., `height: height * 2.0`)
//! - **Compile-time resolution**: Full component list is determined at macro expansion

#[macro_export]
macro_rules! spawnable {
    // Entry point: parse all variants
    ( $($variants:tt),* $(,)? ) => {
        spawnable!(@parse [ $($variants),* ] -> []);
    };

    // Parse root variant (2-tuple)
    (@parse [ ($name:ident { $($field:ident : $ftype:ty),* $(,)? }, ($($comp:expr),* $(,)?)) $(, $($rest:tt)*)? ] -> [$($parsed:tt)*]) => {
        spawnable!(@parse [ $($($rest)*)? ] -> [
            $($parsed)*
            (root $name [$($field : $ftype),*] [$($comp),*])
        ]);
    };

    // Parse delegating variant (3-tuple)
    (@parse [ ($name:ident { $($field:ident : $ftype:ty),* $(,)? }, $parent:ident { $($pfield:ident : $pexpr:expr),* $(,)? }, ($($comp:expr),* $(,)?)) $(, $($rest:tt)*)? ] -> [$($parsed:tt)*]) => {
        spawnable!(@parse [ $($($rest)*)? ] -> [
            $($parsed)*
            (child $name [$($field : $ftype),*] $parent [$($pfield : $pexpr),*] [$($comp),*])
        ]);
    };

    // Done parsing - generate all code
    (@parse [] -> [$($parsed:tt)*]) => {
        spawnable!(@gen_enum [] [$($parsed)*]);
        spawnable!(@gen_impl [$($parsed)*] [$($parsed)*]);
    };

    // ========== ENUM GENERATION ==========

    (@gen_enum [$($collected:tt)*] [(root $name:ident [$($field:ident : $ftype:ty),*] $comps:tt) $($rest:tt)*]) => {
        spawnable!(@gen_enum [$($collected)* $name { $($field : $ftype),* },] [$($rest)*]);
    };

    (@gen_enum [$($collected:tt)*] [(child $name:ident [$($field:ident : $ftype:ty),*] $parent:ident $mapping:tt $comps:tt) $($rest:tt)*]) => {
        spawnable!(@gen_enum [$($collected)* $name { $($field : $ftype),* },] [$($rest)*]);
    };

    (@gen_enum [$($variants:tt)*] []) => {
        #[allow(dead_code)]
        pub enum Spawnable { $($variants)* }
    };

    // ========== IMPL GENERATION ==========

    (@gen_impl [$($all:tt)*] [$($variants:tt)*]) => {
        spawnable!(@gen_arms commands loc [$($all)*] [$($variants)*] -> []);
    };

    // Accumulate root arm
    (@gen_arms $cmd:ident $loc:ident [$($all:tt)*] [(root $name:ident [$($field:ident : $ftype:ty),*] [$($comp:expr),*]) $($rest:tt)*] -> [$($arms:tt)*]) => {
        spawnable!(@gen_arms $cmd $loc [$($all)*] [$($rest)*] -> [
            $($arms)*
            Spawnable::$name { $($field),* } => {
                $cmd.spawn(( $loc, $($comp,)* )).id()
            },
        ]);
    };

    // Accumulate child arm
    (@gen_arms $cmd:ident $loc:ident [$($all:tt)*] [(child $name:ident [$($field:ident : $ftype:ty),*] $parent:ident [$($pfield:ident : $pexpr:expr),*] [$($comp:expr),*]) $($rest:tt)*] -> [$($arms:tt)*]) => {
        spawnable!(@gen_arms $cmd $loc [$($all)*] [$($rest)*] -> [
            $($arms)*
            Spawnable::$name { $($field),* } => {
                let __parent = spawnable!(@find_parent $parent [$($pfield : $pexpr),*] [$($all)*] [$($all)*]);
                $cmd.spawn(($loc, __parent, $($comp,)*)).id()
            },
        ]);
    };

    // Done - emit impl with all arms
    (@gen_arms $cmd:ident $loc:ident [$($all:tt)*] [] -> [$($arms:tt)*]) => {
        impl Spawnable {
            #[allow(unused_variables, clippy::let_unit_value)]
            pub fn spawn<L: bevy::prelude::Component>(self, $cmd: &mut bevy::prelude::Commands, $loc: L) -> bevy::prelude::Entity {
                match self {
                    $($arms)*
                }
            }
        }
    };

    // ========== PARENT LOOKUP ==========

    // Find parent in variant list - check first root
    (@find_parent $parent:ident [$($mapping:tt)*] [(root $candidate:ident [$($f:ident : $ft:ty),*] [$($pcomp:expr),*]) $($rest:tt)*] [$($all:tt)*]) => {
        spawnable!(@check_parent $parent $candidate [$($mapping)*] [$($f),*] [$($pcomp),*] [$($rest)*] [$($all)*])
    };

    // Check child variants as potential parents
    (@find_parent $parent:ident [$($mapping:tt)*] [(child $cname:ident [$($cf:ident : $cft:ty),*] $cp:ident [$($cpf:ident : $cpexpr:expr),*] [$($ccomp:expr),*]) $($rest:tt)*] [$($all:tt)*]) => {
        spawnable!(@check_child_parent $parent $cname [$($mapping)*] [$($cf),*] $cp [$($cpf : $cpexpr),*] [$($ccomp),*] [$($rest)*] [$($all)*])
    };

    // Check if child candidate matches parent
    (@check_child_parent $parent:ident $candidate:ident [$($pfield:ident : $pexpr:expr),*] [$($cf:ident),*] $cp:ident [$($cpf:ident : $cpexpr:expr),*] [$($ccomp:expr),*] [$($rest:tt)*] [$($all:tt)*]) => {
        spawnable!(@do_check_child $parent $candidate
            { $( let $pfield = $pexpr; )* }
            $cp [$($cpf : $cpexpr),*] ( $($ccomp,)* )
            [$($pfield : $pexpr),*] [$($rest)*] [$($all)*])
    };

    // Do the actual check for child parent
    (@do_check_child $parent:ident $candidate:ident { $($bindings:tt)* } $cp:ident [$($cpf:ident : $cpexpr:expr),*] $ccomps:tt [$($mapping:tt)*] [$($rest:tt)*] [$($all:tt)*]) => {{
        macro_rules! __resolve_child {
            ($parent) => {{
                $($bindings)*
                // Recursively resolve the child's parent
                let __grandparent = spawnable!(@find_parent $cp [$($cpf : $cpexpr),*] [$($all)*] [$($all)*]);
                (__grandparent, $ccomps)
            }};
            ($other:ident) => {
                spawnable!(@find_parent $parent [$($mapping)*] [$($rest)*] [$($all)*])
            };
        }
        __resolve_child!($candidate)
    }};

    // Parent not found - compile error
    (@find_parent $parent:ident [$($mapping:tt)*] [] [$($all:tt)*]) => {
        compile_error!(concat!("Parent variant not found: ", stringify!($parent)))
    };

    // Check if candidate matches parent - pre-expand repetitions
    (@check_parent $parent:ident $candidate:ident [$($pfield:ident : $pexpr:expr),*] [$($f:ident),*] [$($pcomp:expr),*] [$($rest:tt)*] [$($all:tt)*]) => {
        spawnable!(@do_check $parent $candidate
            { $( let $pfield = $pexpr; )* }
            ( $($pcomp,)* )
            [$($pfield : $pexpr),*] [$($rest)*] [$($all)*])
    };

    // Do the actual check with pre-expanded code
    (@do_check $parent:ident $candidate:ident { $($bindings:tt)* } $comps:tt [$($mapping:tt)*] [$($rest:tt)*] [$($all:tt)*]) => {{
        macro_rules! __resolve_impl {
            ($parent) => {{
                $($bindings)*
                $comps
            }};
            ($other:ident) => {
                spawnable!(@find_parent $parent [$($mapping)*] [$($rest)*] [$($all)*])
            };
        }
        __resolve_impl!($candidate)
    }};
}

#[cfg(test)]
mod basic_tests {
  use bevy::prelude::*;

  #[derive(Component)]
  struct TestComponent(f32);

  #[derive(Component, Clone)]
  struct TestLocation(i32, i32);

  spawnable! {
      (Root { x: f32, y: f32 }, ((),)),
      (TestRoot { val: f32 }, (TestComponent(val),)),
  }

  #[test]
  fn test_root_variant_exists() { let _root = Spawnable::Root { x: 1.0, y: 2.0 }; }

  #[test]
  fn test_spawn_returns_entity() {
    let mut app = App::new();
    app.add_systems(Startup, |mut commands: Commands| {
      let entity =
        Spawnable::TestRoot { val: 42.0 }.spawn(&mut commands, TestLocation(0, 0));
      assert!(entity != Entity::PLACEHOLDER);
    });
    app.update();
  }

  #[test]
  fn test_spawn_inserts_component() {
    use std::sync::{Arc, Mutex};

    let mut app = App::new();
    let entity_id = Arc::new(Mutex::new(Entity::PLACEHOLDER));
    let entity_id_clone = entity_id.clone();

    app.add_systems(Startup, move |mut commands: Commands| {
      let id = Spawnable::TestRoot { val: 42.0 }.spawn(&mut commands, TestLocation(5, 5));
      *entity_id_clone.lock().unwrap() = id;
    });
    app.update();

    let id = *entity_id.lock().unwrap();
    let world = app.world();
    let comp = world.get::<TestComponent>(id).expect("TestComponent not found");
    assert_eq!(comp.0, 42.0);
  }
}

#[cfg(test)]
mod delegation_tests {
  use bevy::prelude::*;

  #[derive(Component, Debug, PartialEq)]
  struct BaseComp(i32);

  #[derive(Component, Debug, PartialEq)]
  struct ChildComp(String);

  #[derive(Component, Clone)]
  struct TestLocation(i32, i32);

  spawnable! {
      (Base { num: i32 }, (BaseComp(num),)),
      (Child { num: i32, name: String }, Base { num: num }, (ChildComp(name.clone()),)),
  }

  #[test]
  fn test_delegating_variant_has_both_components() {
    use std::sync::{Arc, Mutex};

    let mut app = App::new();
    let entity_id = Arc::new(Mutex::new(Entity::PLACEHOLDER));
    let entity_id_clone = entity_id.clone();

    app.add_systems(Startup, move |mut commands: Commands| {
      let id = Spawnable::Child { num: 10, name: "test".to_string() }
        .spawn(&mut commands, TestLocation(0, 0));
      *entity_id_clone.lock().unwrap() = id;
    });
    app.update();

    let id = *entity_id.lock().unwrap();
    let world = app.world();
    let base = world.get::<BaseComp>(id).expect("BaseComp not found");
    let child = world.get::<ChildComp>(id).expect("ChildComp not found");
    assert_eq!(base.0, 10);
    assert_eq!(child.0, "test");
  }
}

#[cfg(test)]
mod multilevel_tests {
  use bevy::prelude::*;

  #[derive(Component, Debug, PartialEq)]
  struct RootComp(i32);

  #[derive(Component, Debug, PartialEq)]
  struct EnvComp(f32);

  #[derive(Component, Debug, PartialEq)]
  struct TreeComp(String);

  #[derive(Component, Clone)]
  struct TestLocation(i32, i32);

  spawnable! {
      (Root { x: i32 }, (RootComp(x),)),
      (EnvObject { x: i32, height: f32 }, Root { x: x }, (EnvComp(height),)),
      (Tree { x: i32, height: f32, name: String }, EnvObject { x: x, height: height }, (TreeComp(name.clone()),)),
  }

  #[test]
  fn test_grandchild_has_all_three_components() {
    use std::sync::{Arc, Mutex};

    let mut app = App::new();
    let entity_id = Arc::new(Mutex::new(Entity::PLACEHOLDER));
    let entity_id_clone = entity_id.clone();

    app.add_systems(Startup, move |mut commands: Commands| {
      let id = Spawnable::Tree { x: 42, height: 10.5, name: "oak".to_string() }
        .spawn(&mut commands, TestLocation(1, 1));
      *entity_id_clone.lock().unwrap() = id;
    });
    app.update();

    let id = *entity_id.lock().unwrap();
    let world = app.world();
    let root = world.get::<RootComp>(id).expect("RootComp not found");
    let env = world.get::<EnvComp>(id).expect("EnvComp not found");
    let tree = world.get::<TreeComp>(id).expect("TreeComp not found");
    assert_eq!(root.0, 42);
    assert_eq!(env.0, 10.5);
    assert_eq!(tree.0, "oak");
  }
}

#[cfg(test)]
mod arithmetic_tests {
  use bevy::prelude::*;

  #[derive(Component, Debug, PartialEq)]
  struct SizeComp(f32);

  #[derive(Component, Debug, PartialEq)]
  struct ScaledComp(f32);

  #[derive(Component, Clone)]
  struct TestLocation(i32, i32);

  spawnable! {
      (Base { size: f32 }, (SizeComp(size),)),
      // Arithmetic expression in parent field mapping
      (Scaled { size: f32, scale: f32 }, Base { size: size * scale }, (ScaledComp(scale),)),
  }

  #[test]
  fn test_arithmetic_field_expression() {
    use std::sync::{Arc, Mutex};

    let mut app = App::new();
    let entity_id = Arc::new(Mutex::new(Entity::PLACEHOLDER));
    let entity_id_clone = entity_id.clone();

    app.add_systems(Startup, move |mut commands: Commands| {
      let id = Spawnable::Scaled { size: 10.0, scale: 2.5 }
        .spawn(&mut commands, TestLocation(0, 0));
      *entity_id_clone.lock().unwrap() = id;
    });
    app.update();

    let id = *entity_id.lock().unwrap();
    let world = app.world();
    let size = world.get::<SizeComp>(id).expect("SizeComp not found");
    let scaled = world.get::<ScaledComp>(id).expect("ScaledComp not found");
    // size * scale = 10.0 * 2.5 = 25.0
    assert_eq!(size.0, 25.0);
    assert_eq!(scaled.0, 2.5);
  }
}
