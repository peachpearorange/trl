// src/spawnable.rs

#[macro_export]
macro_rules! spawnable {
    ( $( ( $variant:ident { $( $field:ident : $ftype:ty ),* $(,)? } , ( $( $comp:expr ),* $(,)? ) ) ),* $(,)? ) => {
        pub enum Spawnable {
            $( $variant { $( $field : $ftype ),* } ),*
        }
    };
}

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
