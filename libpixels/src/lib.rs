#![allow(non_snake_case)]

#[macro_use]
extern crate log;

pub use value_box_ffi::*;

pub use world::*;

mod world;

#[no_mangle]
pub fn pixels_test() -> bool {
    true
}

#[no_mangle]
pub fn pixels_init_logger() {
    if let Err(error) = env_logger::try_init() {
        eprintln!("[pixels] Failed to init env_logger: {}", error);
    }
}
