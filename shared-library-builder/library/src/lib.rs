use shared_library_builder::{GitLocation, LibraryLocation, RustLibrary};

pub fn libpixels(version: Option<impl Into<String>>) -> RustLibrary {
    let mut location = GitLocation::github("feenkcom", "libpixels");
    if let Some(version) = version {
        location = location.tag(version);
    }

    RustLibrary::new("Pixels", LibraryLocation::Git(location)).package("libpixels")
}

pub fn latest_libpixels() -> RustLibrary {
    let version: Option<String> = None;
    libpixels(version)
}
