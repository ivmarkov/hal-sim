use core::convert::Infallible;

extern crate alloc;
use alloc::sync::Arc;

use log::trace;

use std::sync::Mutex;

use embedded_graphics_core::{
    prelude::{Dimensions, DrawTarget, PixelColor, Point, Size},
    primitives::Rectangle,
    Pixel,
};

pub use crate::dto::display::*;

pub(crate) static DISPLAYS: Mutex<Vec<DisplayState>> = Mutex::new(Vec::new());

pub struct Displays {
    id_gen: u8,
    changed: DisplaysChangedCallback,
}

impl Displays {
    pub(crate) fn new(changed: impl Fn() + 'static) -> Self {
        Self {
            id_gen: 0,
            changed: Arc::new(changed),
        }
    }

    pub fn display<C>(
        &mut self,
        name: impl Into<DisplayName>,
        width: usize,
        height: usize,
        converter: impl Fn(C) -> u32 + 'static,
    ) -> Display<C>
    where
        C: Clone + Default,
    {
        let id = self.id_gen;
        self.id_gen += 1;

        let state = DisplayState::new(name.into(), width, height);

        {
            let mut states = DISPLAYS.lock().unwrap();
            states.push(state);
        }

        Display::new(id, self.changed.clone(), converter)
    }
}

pub type DisplaysChangedCallback = Arc<dyn Fn()>;

pub struct Display<C> {
    id: u8,
    changed: Arc<dyn Fn()>,
    converter: Box<dyn Fn(C) -> u32>,
}

impl<C> Display<C>
where
    C: Clone + Default,
{
    fn new(id: u8, changed: Arc<dyn Fn()>, converter: impl Fn(C) -> u32 + 'static) -> Self {
        Self {
            id,
            changed,
            converter: Box::new(converter),
        }
    }
}

impl<C> Drop for Display<C> {
    fn drop(&mut self) {
        {
            let mut guard = DISPLAYS.lock().unwrap();
            let state = &mut guard[self.id as usize];

            state.display.dropped = true;
            state.change.dropped = true;
        }

        (self.changed)();
    }
}

impl<C> DrawTarget for Display<C>
where
    C: PixelColor,
{
    type Color = C;

    type Error = Infallible;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        let changed = {
            let mut guard = DISPLAYS.lock().unwrap();

            guard[self.id as usize].draw_iter(
                pixels
                    .into_iter()
                    .map(|Pixel(point, pixel)| (point, (self.converter)(pixel))),
            )
        };

        if changed {
            (self.changed)();
        }

        Ok(())
    }
}

impl<C> Dimensions for Display<C> {
    fn bounding_box(&self) -> Rectangle {
        let guard = DISPLAYS.lock().unwrap();

        let state = &guard[self.id as usize];

        Rectangle::new(
            Point::new(0, 0),
            Size::new(
                state.display.meta.width as _,
                state.display.meta.height as _,
            ),
        )
    }
}

pub struct DisplayState {
    display: SharedDisplay,
    change: Change,
}

impl DisplayState {
    fn new(name: DisplayName, width: usize, height: usize) -> Self {
        Self {
            display: SharedDisplay::new(name, width, height),
            change: Change {
                created: true,
                dropped: false,
                screen_updates: Vec::new(),
            },
        }
    }

    pub fn change(&self) -> &Change {
        &self.change
    }

    pub fn display(&self) -> &SharedDisplay {
        &self.display
    }

    pub fn split(&mut self) -> (&SharedDisplay, &mut Change) {
        (&self.display, &mut self.change)
    }

    fn draw_iter<I>(&mut self, pixels: I) -> bool
    where
        I: IntoIterator<Item = (Point, u32)>,
    {
        self.display.draw_iter(&mut self.change, pixels)
    }
}

pub struct SharedDisplay {
    meta: DisplayMeta,
    dropped: bool,
    buffer: Vec<u32>,
}

impl SharedDisplay {
    fn new(name: DisplayName, width: usize, height: usize) -> Self {
        Self {
            meta: DisplayMeta {
                name,
                width,
                height,
            },
            dropped: false,
            buffer: vec![0; width * height],
        }
    }

    pub fn meta(&self) -> &DisplayMeta {
        &self.meta
    }

    pub fn dropped(&self) -> bool {
        self.dropped
    }

    pub fn buffer(&self) -> &[u32] {
        &self.buffer
    }

    fn draw_iter<I>(&mut self, changed_state: &mut Change, pixels: I) -> bool
    where
        I: IntoIterator<Item = (Point, u32)>,
    {
        let mut changed = false;

        for pixel in pixels {
            if pixel.0.x >= 0
                && pixel.0.x < self.meta.width as _
                && pixel.0.y >= 0
                && pixel.0.y < self.meta.height as _
            {
                let x = pixel.0.x as usize;
                let y = pixel.0.y as usize;

                let cell = &mut self.buffer[y * self.meta.width + x];

                if *cell != pixel.1 {
                    *cell = pixel.1;

                    changed_state.update_row(y, x, x + 1);
                    changed = true;

                    trace!("Updated pixel x={} y={}", x, y);
                }
            }
        }

        changed
    }
}
