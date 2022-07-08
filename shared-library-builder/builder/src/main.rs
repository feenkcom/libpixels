use std::error::Error;

use shared_library_builder::build_standalone;

use libpixels_library::latest_libpixels;

fn main() -> Result<(), Box<dyn Error>> {
    build_standalone(|_| Ok(Box::new(latest_libpixels())))
}
