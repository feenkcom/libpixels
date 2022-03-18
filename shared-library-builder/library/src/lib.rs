use shared_library_builder::{GitLocation, LibraryLocation, RustLibrary};

pub fn libpixels(version: impl Into<String>) -> RustLibrary {
    RustLibrary::new(
        "Pixels",
        LibraryLocation::Git(GitLocation::github("feenkcom", "libpixels").tag(version)),
    )
    .package("libpixels")
}
