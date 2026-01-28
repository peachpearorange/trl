# Spawnable Macro Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Implement a `spawnable!` macro that generates an enum with pseudo-inheritance for Bevy entity spawning.

**Architecture:** A `macro_rules!` macro that parses variant definitions, resolves delegation chains at compile time, and generates a `spawn()` method that inserts flattened component lists.

**Tech Stack:** Rust, macro_rules!, Bevy 0.17

---

### Task 1: Create spawnable module with minimal macro skeleton

**Files:**
- Create: `src/spawnable.rs`
- Modify: `src/main.rs` (add module declaration)

**Step 1: Create the macro file with simplest possible macro**

```rust
// src/spawnable.rs

#[macro_export]
macro_rules! spawnable {
    ( $( ( $variant:ident { $( $field:ident : $ftype:ty ),* $(,)? } , ( $( $comp:expr ),* $(,)? ) ) ),* $(,)? ) => {
        pub enum Spawnable {
            $( $variant { $( $field : $ftype ),* } ),*
        }
    };
}
```

**Step 2: Add module to main.rs**

Add at top of `src/main.rs`:
```rust
mod spawnable;
```

**Step 3: Verify it compiles**

Run: `cargo check`
Expected: Compiles without errors

**Step 4: Commit**

```bash
git add src/spawnable.rs src/main.rs
git commit -m "feat: add spawnable macro skeleton"
```

---

### Task 2: Add test that uses the macro to define root variants

**Files:**
- Modify: `src/spawnable.rs` (add test module)

**Step 1: Write test for root variant enum generation**

Add at bottom of `src/spawnable.rs`:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    spawnable! {
        (Root { x: f32, y: f32 }, ((),)),
    }

    #[test]
    fn test_root_variant_exists() {
        let _root = Spawnable::Root { x: 1.0, y: 2.0 };
    }
}
```

**Step 2: Run test to verify it passes**

Run: `cargo test test_root_variant_exists`
Expected: PASS (enum variant is generated)

**Step 3: Commit**

```bash
git add src/spawnable.rs
git commit -m "test: verify root variant enum generation"
```

---

### Task 3: Add spawn method for root variants

**Files:**
- Modify: `src/spawnable.rs`

**Step 1: Write failing test for spawn method**

Add to test module in `src/spawnable.rs`:
```rust
    use bevy::prelude::*;

    #[derive(Component)]
    struct TestComponent(f32);

    spawnable! {
        (TestRoot { val: f32 }, (TestComponent(val),)),
    }

    #[test]
    fn test_spawn_returns_entity() {
        let mut app = App::new();
        app.add_systems(Startup, |mut commands: Commands| {
            let entity = Spawnable::TestRoot { val: 42.0 }.spawn(&mut commands);
            assert!(entity != Entity::PLACEHOLDER);
        });
        app.update();
    }
```

**Step 2: Run test to verify it fails**

Run: `cargo test test_spawn_returns_entity`
Expected: FAIL - no method `spawn` found

**Step 3: Implement spawn method in macro**

Update macro in `src/spawnable.rs`:
```rust
#[macro_export]
macro_rules! spawnable {
    ( $( ( $variant:ident { $( $field:ident : $ftype:ty ),* $(,)? } , ( $( $comp:expr ),* $(,)? ) ) ),* $(,)? ) => {
        pub enum Spawnable {
            $( $variant { $( $field : $ftype ),* } ),*
        }

        impl Spawnable {
            pub fn spawn(self, commands: &mut bevy::prelude::Commands) -> bevy::prelude::Entity {
                match self {
                    $(
                        Spawnable::$variant { $( $field ),* } => {
                            commands.spawn(( $( $comp ),* )).id()
                        }
                    )*
                }
            }
        }
    };
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test test_spawn_returns_entity`
Expected: PASS

**Step 5: Commit**

```bash
git add src/spawnable.rs
git commit -m "feat: add spawn method for root variants"
```

---

### Task 4: Add test for component insertion verification

**Files:**
- Modify: `src/spawnable.rs`

**Step 1: Write test that verifies component is actually inserted**

Add to test module:
```rust
    #[test]
    fn test_spawn_inserts_component() {
        let mut app = App::new();
        let mut entity_id = Entity::PLACEHOLDER;

        app.add_systems(Startup, |mut commands: Commands| {
            entity_id = Spawnable::TestRoot { val: 42.0 }.spawn(&mut commands);
        });
        app.update();

        let world = app.world();
        let comp = world.get::<TestComponent>(entity_id).expect("TestComponent not found");
        assert_eq!(comp.0, 42.0);
    }
```

**Step 2: Run test**

Run: `cargo test test_spawn_inserts_component`
Expected: PASS (component insertion already works from Task 3)

**Step 3: Commit**

```bash
git add src/spawnable.rs
git commit -m "test: verify component insertion with field values"
```

---

### Task 5: Add support for delegating variants (3-tuples)

**Files:**
- Modify: `src/spawnable.rs`

**Step 1: Write failing test for delegating variant**

Add to test module:
```rust
    #[derive(Component)]
    struct BaseComp(i32);

    #[derive(Component)]
    struct ChildComp(String);

    spawnable! {
        (Base { num: i32 }, (BaseComp(num),)),
        (Child { num: i32, name: String }, Base { num: num }, (ChildComp(name.clone()),)),
    }

    #[test]
    fn test_delegating_variant_has_both_components() {
        let mut app = App::new();
        let mut entity_id = Entity::PLACEHOLDER;

        app.add_systems(Startup, |mut commands: Commands| {
            entity_id = Spawnable::Child { num: 10, name: "test".to_string() }.spawn(&mut commands);
        });
        app.update();

        let world = app.world();
        let base = world.get::<BaseComp>(entity_id).expect("BaseComp not found");
        let child = world.get::<ChildComp>(entity_id).expect("ChildComp not found");
        assert_eq!(base.0, 10);
        assert_eq!(child.0, "test");
    }
```

**Step 2: Run test to verify it fails**

Run: `cargo test test_delegating_variant_has_both_components`
Expected: FAIL - macro doesn't accept 3-tuple syntax

**Step 3: Extend macro to handle both 2-tuple and 3-tuple**

Replace macro in `src/spawnable.rs`:
```rust
#[macro_export]
macro_rules! spawnable {
    // Internal rule: collect all variants first, then generate
    (@collect [] -> [$($collected:tt)*]) => {
        spawnable!(@generate $($collected)*);
    };

    // Collect root variant (2-tuple)
    (@collect [($variant:ident { $($field:ident : $ftype:ty),* $(,)? }, ($($comp:expr),* $(,)?)) , $($rest:tt)*] -> [$($collected:tt)*]) => {
        spawnable!(@collect [$($rest)*] -> [$($collected)* (root $variant { $($field : $ftype),* } => ($($comp),*))]);
    };

    // Collect delegating variant (3-tuple)
    (@collect [($variant:ident { $($field:ident : $ftype:ty),* $(,)? }, $parent:ident { $($pfield:ident : $pexpr:expr),* $(,)? }, ($($comp:expr),* $(,)?)) , $($rest:tt)*] -> [$($collected:tt)*]) => {
        spawnable!(@collect [$($rest)*] -> [$($collected)* (delegate $variant { $($field : $ftype),* } => $parent { $($pfield : $pexpr),* } => ($($comp),*))]);
    };

    // Generate enum and impl
    (@generate $((root $rvar:ident { $($rfield:ident : $rftype:ty),* } => ($($rcomp:expr),*)))* $((delegate $dvar:ident { $($dfield:ident : $dftype:ty),* } => $dparent:ident { $($dpfield:ident : $dpexpr:expr),* } => ($($dcomp:expr),*)))*) => {
        pub enum Spawnable {
            $( $rvar { $($rfield : $rftype),* } ),*
            ,
            $( $dvar { $($dfield : $dftype),* } ),*
        }

        impl Spawnable {
            pub fn spawn(self, commands: &mut bevy::prelude::Commands) -> bevy::prelude::Entity {
                match self {
                    $(
                        Spawnable::$rvar { $($rfield),* } => {
                            commands.spawn(($($rcomp),*)).id()
                        }
                    )*
                    $(
                        Spawnable::$dvar { $($dfield),* } => {
                            // For now, just spawn own components + call parent logic inline
                            // We need to expand parent's components here
                            let parent_comps = spawnable!(@parent_components $dparent { $($dpfield : $dpexpr),* });
                            commands.spawn(($($dcomp),*)).id()
                        }
                    )*
                }
            }
        }
    };

    // Entry point
    ($($input:tt)*) => {
        spawnable!(@collect [$($input)*,] -> []);
    };
}
```

**Note:** This intermediate step won't fully work yet - we need Task 6 to resolve parent components.

**Step 4: Verify compilation (test will still fail)**

Run: `cargo check`
Expected: Compiles (test still fails)

**Step 5: Commit work in progress**

```bash
git add src/spawnable.rs
git commit -m "wip: add 3-tuple parsing for delegating variants"
```

---

### Task 6: Implement parent component resolution

**Files:**
- Modify: `src/spawnable.rs`

**Step 1: Rewrite macro with component flattening**

The key insight: we can't truly recurse in macro_rules!, so we need a different approach. We'll require all variants to be defined in dependency order (roots first), and generate each match arm by looking up the parent's components inline.

Replace macro:
```rust
#[macro_export]
macro_rules! spawnable {
    // Entry: parse all variants into normalized form, then generate
    ($($input:tt)*) => {
        spawnable!(@parse [] $($input)*);
    };

    // Parse root variant (2-tuple)
    (@parse [$($parsed:tt)*] ($variant:ident { $($field:ident : $ftype:ty),* $(,)? }, ($($comp:expr),* $(,)?)) $(, $($rest:tt)*)?) => {
        spawnable!(@parse [$($parsed)* (root $variant [$($field : $ftype),*] [$($comp),*])] $($($rest)*)?);
    };

    // Parse delegating variant (3-tuple)
    (@parse [$($parsed:tt)*] ($variant:ident { $($field:ident : $ftype:ty),* $(,)? }, $parent:ident { $($pfield:ident : $pexpr:expr),* $(,)? }, ($($comp:expr),* $(,)?)) $(, $($rest:tt)*)?) => {
        spawnable!(@parse [$($parsed)* (child $variant [$($field : $ftype),*] $parent [$($pfield = $pexpr),*] [$($comp),*])] $($($rest)*)?);
    };

    // Done parsing, generate code
    (@parse [$($parsed:tt)*]) => {
        spawnable!(@generate [$($parsed)*] [$($parsed)*]);
    };

    // Generate enum and impl
    (@generate [$($all:tt)*] [$(
        $( (root $rvar:ident [$($rfield:ident : $rftype:ty),*] [$($rcomp:expr),*]) )?
        $( (child $cvar:ident [$($cfield:ident : $cftype:ty),*] $cparent:ident [$($cpfield:ident = $cpexpr:expr),*] [$($ccomp:expr),*]) )?
    )*]) => {
        pub enum Spawnable {
            $($( $rvar { $($rfield : $rftype),* }, )?)*
            $($( $cvar { $($cfield : $cftype),* }, )?)*
        }

        impl Spawnable {
            pub fn spawn(self, commands: &mut bevy::prelude::Commands) -> bevy::prelude::Entity {
                match self {
                    $($(
                        Spawnable::$rvar { $($rfield),* } => {
                            commands.spawn(($($rcomp,)*)).id()
                        }
                    )?)*
                    $($(
                        Spawnable::$cvar { $($cfield),* } => {
                            // Resolve parent components
                            let parent_components = spawnable!(@resolve $cparent [$($cpfield = $cpexpr),*] [$($all)*]);
                            commands.spawn((parent_components, $($ccomp,)*)).id()
                        }
                    )?)*
                }
            }
        }
    };

    // Resolve: find parent in list and substitute its components
    (@resolve $parent:ident [$($pfield:ident = $pexpr:expr),*] [(root $parent2:ident [$($f:ident : $ft:ty),*] [$($comp:expr),*]) $($rest:tt)*]) => {
        {
            $(let $f = spawnable!(@lookup $f [$($pfield = $pexpr),*]);)*
            ($($comp,)*)
        }
    };

    // Skip non-matching variant
    (@resolve $parent:ident [$($pfield:ident = $pexpr:expr),*] [$skip:tt $($rest:tt)*]) => {
        spawnable!(@resolve $parent [$($pfield = $pexpr),*] [$($rest)*])
    };

    // Lookup field value from parent mapping
    (@lookup $field:ident [$field2:ident = $expr:expr $(, $($rest:tt)*)?]) => {
        $expr
    };
    (@lookup $field:ident [$other:ident = $expr:expr, $($rest:tt)*]) => {
        spawnable!(@lookup $field [$($rest)*])
    };
}
```

**Step 2: Run test**

Run: `cargo test test_delegating_variant_has_both_components`
Expected: May still fail - this is complex, iterate as needed

**Step 3: Debug and fix**

If tests fail, simplify and debug. The macro is complex - may need adjustment based on actual compiler errors.

**Step 4: Once passing, commit**

```bash
git add src/spawnable.rs
git commit -m "feat: implement parent component resolution for delegation"
```

---

### Task 7: Add test for multi-level delegation (grandparent)

**Files:**
- Modify: `src/spawnable.rs`

**Step 1: Write test for 3-level inheritance**

Add to tests:
```rust
    #[derive(Component)]
    struct GrandparentComp;

    #[derive(Component)]
    struct ParentComp(i32);

    #[derive(Component)]
    struct GrandchildComp(String);

    spawnable! {
        (Grandparent {}, (GrandparentComp,)),
        (Parent { val: i32 }, Grandparent {}, (ParentComp(val),)),
        (Grandchild { val: i32, name: String }, Parent { val: val }, (GrandchildComp(name.clone()),)),
    }

    #[test]
    fn test_three_level_inheritance() {
        let mut app = App::new();
        let mut entity_id = Entity::PLACEHOLDER;

        app.add_systems(Startup, |mut commands: Commands| {
            entity_id = Spawnable::Grandchild { val: 5, name: "gc".to_string() }.spawn(&mut commands);
        });
        app.update();

        let world = app.world();
        assert!(world.get::<GrandparentComp>(entity_id).is_some());
        assert_eq!(world.get::<ParentComp>(entity_id).unwrap().0, 5);
        assert_eq!(world.get::<GrandchildComp>(entity_id).unwrap().0, "gc");
    }
```

**Step 2: Run test**

Run: `cargo test test_three_level_inheritance`
Expected: FAIL (need recursive resolution)

**Step 3: Extend @resolve to handle child parents**

This requires @resolve to recursively call itself when it finds a child (not root) parent. Update the macro's @resolve rules to handle this case.

**Step 4: Run test to verify it passes**

Run: `cargo test test_three_level_inheritance`
Expected: PASS

**Step 5: Commit**

```bash
git add src/spawnable.rs
git commit -m "feat: support multi-level delegation chains"
```

---

### Task 8: Add test for field expressions with arithmetic

**Files:**
- Modify: `src/spawnable.rs`

**Step 1: Write test with arithmetic in delegation**

```rust
    #[derive(Component)]
    struct SizeComp { width: f32, height: f32 }

    spawnable! {
        (Sized { w: f32, h: f32 }, (SizeComp { width: w, height: h },)),
        (DoubleSized { base: f32 }, Sized { w: base * 2.0, h: base * 2.0 }, ((),)),
    }

    #[test]
    fn test_arithmetic_in_delegation() {
        let mut app = App::new();
        let mut entity_id = Entity::PLACEHOLDER;

        app.add_systems(Startup, |mut commands: Commands| {
            entity_id = Spawnable::DoubleSized { base: 5.0 }.spawn(&mut commands);
        });
        app.update();

        let world = app.world();
        let size = world.get::<SizeComp>(entity_id).unwrap();
        assert_eq!(size.width, 10.0);
        assert_eq!(size.height, 10.0);
    }
```

**Step 2: Run test**

Run: `cargo test test_arithmetic_in_delegation`
Expected: PASS (expressions should work since we use `$pexpr:expr`)

**Step 3: Commit**

```bash
git add src/spawnable.rs
git commit -m "test: verify arithmetic expressions in delegation"
```

---

### Task 9: Clean up and document

**Files:**
- Modify: `src/spawnable.rs`

**Step 1: Add doc comments to macro**

Add documentation at top of `src/spawnable.rs`:
```rust
//! Spawnable macro for declarative entity spawning with pseudo-inheritance.
//!
//! # Example
//! ```
//! spawnable! {
//!     (Root {}, (Transform::default(),)),
//!     (Tree { height: f32 }, Root {}, (TreeComponent { height },)),
//! }
//!
//! // Usage:
//! // Spawnable::Tree { height: 5.0 }.spawn(&mut commands);
//! ```
```

**Step 2: Run all tests**

Run: `cargo test`
Expected: All tests pass

**Step 3: Commit**

```bash
git add src/spawnable.rs
git commit -m "docs: add documentation for spawnable macro"
```

---

## Notes

- The macro implementation in Tasks 5-6 is approximate - `macro_rules!` is finicky and may need debugging during implementation
- If the recursive resolution becomes too complex for `macro_rules!`, consider simplifying to only support single-level delegation, or switch to a proc macro
- Test with real Bevy components (Transform, Sprite, etc.) once basic tests pass
