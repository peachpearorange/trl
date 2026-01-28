// src/spawnable.rs

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

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::prelude::*;

    #[derive(Component)]
    struct TestComponent(f32);

    spawnable! {
        (Root { x: f32, y: f32 }, ((),)),
        (TestRoot { val: f32 }, (TestComponent(val),)),
    }

    #[test]
    fn test_root_variant_exists() {
        let _root = Spawnable::Root { x: 1.0, y: 2.0 };
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

    #[test]
    fn test_spawn_inserts_component() {
        use std::sync::{Arc, Mutex};

        let mut app = App::new();
        let entity_id = Arc::new(Mutex::new(Entity::PLACEHOLDER));
        let entity_id_clone = entity_id.clone();

        app.add_systems(Startup, move |mut commands: Commands| {
            let id = Spawnable::TestRoot { val: 42.0 }.spawn(&mut commands);
            *entity_id_clone.lock().unwrap() = id;
        });
        app.update();

        let id = *entity_id.lock().unwrap();
        let world = app.world();
        let comp = world.get::<TestComponent>(id).expect("TestComponent not found");
        assert_eq!(comp.0, 42.0);
    }
}
