use shared_library_builder::{GitLocation, LibraryLocation, RustLibrary};

pub fn libpixels(version: Option<impl Into<String>>) -> RustLibrary {
    RustLibrary::new(
        "Pixels",
        LibraryLocation::Git(GitLocation::github("feenkcom", "libpixels").tag_or_latest(version)),
    )
    .package("libpixels")
}

pub fn latest_libpixels() -> RustLibrary {
    let version: Option<String> = None;
    libpixels(version)
}
