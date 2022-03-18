use boxer::array::BoxerArrayU8;
use boxer::{ValueBox, ValueBoxPointer, ValueBoxPointerReference};
use pixels::{Pixels, SurfaceTexture};
use raw_window_handle::{HasRawWindowHandle, RawWindowHandle};
use std::sync::Mutex;

#[no_mangle]
pub fn pixels_test() -> bool {
    true
}

#[no_mangle]
pub fn pixels_new_world(
    width: u32,
    height: u32,
    handle: *mut ValueBox<RawWindowHandle>,
) -> *mut ValueBox<World> {
    handle.with_not_null_value_return(std::ptr::null_mut(), |window_handle| {
        let window = Window {
            handle: window_handle,
        };
        let surface_texture = SurfaceTexture::new(width, height, &window);
        let pixels = Pixels::new(width, height, surface_texture).expect("Failed to create pixels");

        ValueBox::new(World {
            _window: window,
            pixels,
            buffer: Mutex::new(Buffer::new()),
        })
        .into_raw()
    })
}

#[no_mangle]
pub fn pixels_world_update(
    world: *mut ValueBox<World>,
    width: u32,
    height: u32,
    pixels: *mut ValueBox<BoxerArrayU8>,
) {
    world.with_not_null(|world| {
        pixels.with_not_null(|pixels| {
            world.update(width, height, pixels.to_slice());
        })
    });
}

#[no_mangle]
pub fn pixels_world_draw(world: *mut ValueBox<World>) {
    world.with_not_null(|world| {
        world.draw();
    });
}

#[no_mangle]
pub fn pixels_world_drop(world: &mut *mut ValueBox<World>) {
    world.drop();
}

#[derive(Debug)]
pub struct World {
    _window: Window,
    pixels: Pixels,
    buffer: Mutex<Buffer>,
}

impl World {
    pub fn draw(&mut self) {
        let mut buffer = self.buffer.lock().unwrap();
        if buffer.size_dirty {
            self.pixels.resize_buffer(buffer.width, buffer.height);
        }
        if buffer.pixels_dirty {
            let frame = self.pixels.get_frame();
            frame.clone_from_slice(buffer.pixels());
        }
        buffer.mark_clean();
        drop(buffer);

        self.pixels.render().expect("pixels.render() failed");
    }

    pub fn update(&mut self, width: u32, height: u32, pixels: &[u8]) {
        self.buffer.lock().unwrap().update(width, height, pixels);
    }
}

#[derive(Debug)]
pub struct Buffer {
    width: u32,
    height: u32,
    size_dirty: bool,
    pixels: Vec<u8>,
    pixels_dirty: bool,
}

impl Buffer {
    pub fn new() -> Self {
        Self {
            width: 1,
            height: 1,
            size_dirty: false,
            pixels: vec![0, 0, 0, 0],
            pixels_dirty: false,
        }
    }

    pub fn update(&mut self, width: u32, height: u32, pixels: &[u8]) {
        if self.width != width || self.height != height {
            self.width = width;
            self.height = height;
            self.size_dirty = true;
        }

        self.pixels = Vec::from(pixels);
        self.pixels_dirty = true;
    }

    pub fn mark_clean(&mut self) {
        self.size_dirty = false;
        self.pixels_dirty = false;
    }

    pub fn pixels(&self) -> &[u8] {
        self.pixels.as_slice()
    }
}

#[derive(Debug)]
pub struct Window {
    handle: RawWindowHandle,
}

unsafe impl HasRawWindowHandle for Window {
    fn raw_window_handle(&self) -> RawWindowHandle {
        self.handle.clone()
    }
}
