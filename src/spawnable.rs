// src/spawnable.rs

#[macro_export]
macro_rules! spawnable {
    ( $( ( $variant:ident { $( $field:ident : $ftype:ty ),* $(,)? } , ( $( $comp:expr ),* $(,)? ) ) ),* $(,)? ) => {
        pub enum Spawnable {
            $( $variant { $( $field : $ftype ),* } ),*
        }
    };
}
