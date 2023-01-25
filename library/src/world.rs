use std::collections::VecDeque;
use std::mem::transmute;
use std::sync::Mutex;

use array_box::ArrayBox;
use euclid::{Point2D, Rect, Size2D};
use imgref::*;
use pixels::wgpu::{Backends, TextureFormat};
use pixels::{Pixels, PixelsBuilder, SurfaceTexture};
use raw_window_handle::{
    HasRawDisplayHandle, HasRawWindowHandle, RawDisplayHandle, RawWindowHandle,
};
use value_box::{BoxerError, ReturnBoxerResult, ValueBox, ValueBoxPointer};

#[no_mangle]
pub fn pixels_new_world(
    surface_width: u32,
    surface_height: u32,
    window_handle: *mut ValueBox<RawWindowHandle>,
    display_handle: *mut ValueBox<RawDisplayHandle>,
) -> *mut ValueBox<World> {
    window_handle
        .with_clone(|window_handle| {
            display_handle.with_clone(|display_handle| {
                let window = Window {
                    window_handle,
                    display_handle,
                };
                let surface_texture = SurfaceTexture::new(surface_width, surface_height, &window);
                PixelsBuilder::new(surface_width, surface_height, surface_texture)
                    .wgpu_backend(Backends::METAL | Backends::GL | Backends::DX12 | Backends::DX11)
                    .texture_format(TextureFormat::Bgra8UnormSrgb)
                    .build()
                    .map(|pixels| World {
                        _window: window,
                        pixels,
                        buffer: Mutex::new(Buffer::new()),
                        damages: Mutex::new(Default::default()),
                        current_damage: Default::default(),
                    })
                    .map_err(|error| BoxerError::AnyError(Box::new(error).into()))
            })
        })
        .into_raw()
}

#[no_mangle]
pub fn pixels_world_damage(
    world: *mut ValueBox<World>,
    left: usize,
    top: usize,
    width: usize,
    height: usize,
) {
    world
        .with_mut_ok(|world| {
            world.damage(Damage::new(
                Point2D::new(left, top),
                Size2D::new(width, height),
            ))
        })
        .log();
}

#[no_mangle]
pub fn pixels_world_resize_surface(world: *mut ValueBox<World>, width: usize, height: usize) {
    world
        .with_mut_ok(|world| world.resize_surface(width, height))
        .log();
}

#[no_mangle]
pub fn pixels_world_resize_buffer(world: *mut ValueBox<World>, width: usize, height: usize) {
    world
        .with_mut_ok(|world| world.resize_buffer(width, height))
        .log();
}

#[no_mangle]
pub fn pixels_world_get_buffer(world: *mut ValueBox<World>) -> *mut ValueBox<ArrayBox<u8>> {
    world
        .with_ref_ok(|world| {
            let buffer = world.buffer.lock().unwrap();
            let slice = buffer.pixels.as_slice();
            ArrayBox::from_data(slice.as_ptr() as *mut u8, slice.len() * 4)
        })
        .into_raw()
}

#[no_mangle]
pub fn pixels_world_draw(world: *mut ValueBox<World>) {
    world
        .with_mut(|world| {
            world
                .draw()
                .map_err(|error| BoxerError::AnyError(error.into()))
        })
        .log();
}

#[no_mangle]
pub fn pixels_world_drop(world: *mut ValueBox<World>) {
    world.release();
}

#[derive(Debug)]
pub struct DamageType {}
pub type Damage = Rect<usize, DamageType>;

pub trait Clamp {
    fn clamp(&self, image: ImgRef<u32>) -> Option<Damage>;
}

impl Clamp for Damage {
    fn clamp(&self, image: ImgRef<u32>) -> Option<Damage> {
        let left = self.min_x().max(0).min(image.width());
        let top = self.min_y().max(0).min(image.height());

        let width = self.width().min(image.width() - left);
        let height = self.height().min(image.height() - top);

        if width == 0 || height == 0 {
            return None;
        } else {
            Some(Damage::new(
                Point2D::new(left, top),
                Size2D::new(width, height),
            ))
        }
    }
}

#[derive(Debug)]
pub struct WorldDamage {
    damage: Damage,
    buffer: Vec<u32>,
}

impl WorldDamage {
    #[inline]
    pub fn left(&self) -> usize {
        self.damage.min_x()
    }

    #[inline]
    pub fn top(&self) -> usize {
        self.damage.min_y()
    }

    #[inline]
    pub fn width(&self) -> usize {
        self.damage.width()
    }

    #[inline]
    pub fn height(&self) -> usize {
        self.damage.height()
    }
}

#[derive(Debug)]
pub struct World {
    _window: Window,
    pixels: Pixels,
    buffer: Mutex<Buffer>,
    damages: Mutex<VecDeque<WorldDamage>>,
    current_damage: Damage,
}

impl World {
    pub fn draw(&mut self) -> Result<(), pixels::Error> {
        let mut buffer = self.buffer.lock().unwrap();

        let buffer_width = buffer.buffer_width;
        let buffer_height = buffer.buffer_height;

        if buffer.buffer_size_dirty {
            trace!("Resize buffer to {}x{}", &buffer_width, buffer_height);
            self.pixels
                .resize_buffer(buffer_width as u32, buffer_height as u32)?;
        }
        if buffer.surface_size_dirty {
            trace!(
                "Resize surface to {}x{}",
                &buffer.surface_width,
                buffer.surface_height
            );
            self.pixels
                .resize_surface(buffer.surface_width as u32, buffer.surface_height as u32)?;
        }

        buffer.mark_clean();
        drop(buffer);

        let frame: &mut [u32] = unsafe { transmute(self.pixels.get_frame_mut()) };

        let mut frame_image =
            ImgRefMut::new_stride(frame, buffer_width, buffer_height, buffer_width);

        let mut damages = self.damages.lock().unwrap();
        for world_damage in damages.drain(0..) {
            if let Some(damage) = world_damage.damage.clamp(frame_image.as_ref()) {
                trace!("Draw damage {:?}", &damage);

                let damage_image = ImgRef::new(
                    world_damage.buffer.as_slice(),
                    damage.width(),
                    damage.height(),
                );

                let mut frame_image = frame_image.sub_image_mut(
                    damage.min_x(),
                    damage.min_y(),
                    damage.width(),
                    damage.height(),
                );

                for (damage_row, frame_row) in damage_image.rows().zip(frame_image.rows_mut()) {
                    frame_row.clone_from_slice(damage_row);
                }
            }
        }
        self.current_damage = Damage::default();
        drop(damages);

        self.pixels.render()?;

        Ok(())
    }

    pub fn resize_buffer(&mut self, buffer_width: usize, buffer_height: usize) {
        trace!("Record buffer resize to {}x{}", buffer_width, buffer_height);
        self.buffer
            .lock()
            .unwrap()
            .resize_buffer(buffer_width, buffer_height);
    }

    pub fn resize_surface(&mut self, surface_width: usize, surface_height: usize) {
        trace!(
            "Record surface resize to {}x{}",
            surface_width,
            surface_height
        );
        self.buffer
            .lock()
            .unwrap()
            .resize_surface(surface_width, surface_height);
    }

    pub fn damage(&mut self, damage: Damage) {
        if let Some(world_damage) = self.buffer.lock().unwrap().damage(damage) {
            trace!("Record damage {:?}", &world_damage.damage);

            let mut damages = self.damages.lock().unwrap();

            // if the new damage is larger than all previous damages combined, we get rid of all previous damages
            if world_damage.damage.contains_rect(&self.current_damage) {
                damages.drain(0..);
                self.current_damage = world_damage.damage.clone();
            } else {
                self.current_damage = self.current_damage.union(&world_damage.damage);
            }
            damages.push_back(world_damage)
        }
    }
}

#[derive(Debug)]
pub struct Buffer {
    buffer_width: usize,
    buffer_height: usize,
    buffer_size_dirty: bool,
    surface_width: usize,
    surface_height: usize,
    surface_size_dirty: bool,
    pixels: Vec<u32>,
}

impl Buffer {
    pub fn new() -> Self {
        Self {
            buffer_width: 1,
            buffer_height: 1,
            buffer_size_dirty: false,
            surface_width: 1,
            surface_height: 1,
            surface_size_dirty: false,
            pixels: vec![0],
        }
    }

    pub fn resize_buffer(&mut self, buffer_width: usize, buffer_height: usize) {
        if self.buffer_width == buffer_width && self.buffer_height == buffer_height {
            return;
        }

        self.buffer_width = buffer_width;
        self.buffer_height = buffer_height;
        self.buffer_size_dirty = true;

        self.pixels
            .resize(buffer_width * buffer_height, Default::default());
    }

    pub fn resize_surface(&mut self, surface_width: usize, surface_height: usize) {
        if self.surface_width == surface_width && self.surface_height == surface_height {
            return;
        }

        self.surface_width = surface_width;
        self.surface_height = surface_height;
        self.surface_size_dirty = true;
    }

    fn buffer_ref(&self) -> ImgRef<u32> {
        ImgRef::new(
            self.pixels.as_slice(),
            self.buffer_width,
            self.buffer_height,
        )
    }

    pub fn damage(&mut self, damage: Damage) -> Option<WorldDamage> {
        let buffer_image = self.buffer_ref();

        damage.clamp(buffer_image).map(|damage| {
            let damaged_image = buffer_image.sub_image(
                damage.min_x(),
                damage.min_y(),
                damage.width(),
                damage.height(),
            );

            let (buffer, _, _) = damaged_image.to_contiguous_buf();

            WorldDamage {
                damage,
                buffer: buffer.to_vec(),
            }
        })
    }

    pub fn mark_clean(&mut self) {
        self.buffer_size_dirty = false;
        self.surface_size_dirty = false;
    }

    pub fn pixels(&self) -> &[u32] {
        self.pixels.as_slice()
    }
}

#[derive(Debug, Clone)]
#[repr(C)]
pub struct Window {
    window_handle: RawWindowHandle,
    display_handle: RawDisplayHandle,
}

unsafe impl HasRawWindowHandle for Window {
    fn raw_window_handle(&self) -> RawWindowHandle {
        self.window_handle.clone()
    }
}

unsafe impl HasRawDisplayHandle for Window {
    fn raw_display_handle(&self) -> RawDisplayHandle {
        self.display_handle.clone()
    }
}
