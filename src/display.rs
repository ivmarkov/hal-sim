use core::cmp::{max, min};
use core::convert::Infallible;

extern crate alloc;
use alloc::sync::Arc;

use std::sync::Mutex;

use embedded_graphics_core::{
    prelude::{Dimensions, DrawTarget, PixelColor, Point, Size},
    primitives::Rectangle,
    Pixel,
};

pub use crate::dto::display::*;

pub struct Displays {
    id_gen: u8,
    shared: SharedDisplays,
    changed: DisplaysChangedCallback,
}

impl Displays {
    pub fn new(changed: impl Fn() + 'static) -> Self {
        Self {
            id_gen: 0,
            shared: Arc::new(Mutex::new(Vec::new())),
            changed: Arc::new(changed),
        }
    }

    pub fn shared(&self) -> &SharedDisplays {
        &self.shared
    }

    pub fn display<C>(
        &mut self,
        name: DisplayName,
        width: usize,
        height: usize,
        converter: impl Fn(C) -> u32 + 'static,
    ) -> Display<C>
    where
        C: Clone + Default,
    {
        let id = self.id_gen;
        self.id_gen += 1;

        let state = DisplayState::new(name, width, height);

        {
            let mut states = self.shared.lock().unwrap();
            states.push(state);
        }

        Display::new(id, self.shared.clone(), self.changed.clone(), converter)
    }
}

pub type SharedDisplays = Arc<Mutex<Vec<DisplayState>>>;
pub type DisplaysChangedCallback = Arc<dyn Fn()>;

pub struct Display<C> {
    id: u8,
    displays: SharedDisplays,
    changed: Arc<dyn Fn()>,
    converter: Box<dyn Fn(C) -> u32>,
}

impl<C> Display<C>
where
    C: Clone + Default,
{
    fn new(
        id: u8,
        displays: SharedDisplays,
        changed: Arc<dyn Fn()>,
        converter: impl Fn(C) -> u32 + 'static,
    ) -> Self {
        Self {
            id,
            displays,
            changed,
            converter: Box::new(converter),
        }
    }
}

impl<C> Drop for Display<C> {
    fn drop(&mut self) {
        {
            let mut guard = self.displays.lock().unwrap();
            let state = &mut guard[self.id as usize];

            state.display.dropped = true;
            state.change.update(&Change::Updated(Vec::new(), true));
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
            let mut guard = self.displays.lock().unwrap();

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
        let guard = self.displays.lock().unwrap();

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
            change: Change::Created,
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
                }
            }
        }

        changed
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Change {
    Created,
    Updated(Vec<(usize, usize)>, bool),
}

impl Change {
    pub fn reset(&mut self) {
        *self = Self::Updated(Vec::new(), false);
    }

    pub fn update(&mut self, other: &Self) {
        match self {
            Self::Created => (),
            Self::Updated(_, dropped) => match other {
                Self::Created => *self = Self::Created,
                Self::Updated(other_rows, other_dropped) => {
                    *dropped = *dropped || *other_dropped;

                    for (i, other_row) in other_rows.iter().enumerate() {
                        self.update_row(i, other_row.0, other_row.1);
                    }
                }
            },
        }
    }

    pub fn update_row(&mut self, index: usize, start: usize, end: usize) {
        match self {
            Self::Created => (),
            Self::Updated(rows, _) => {
                if start < end {
                    while rows.len() <= index {
                        rows.push((0, 0));
                    }

                    let row = &mut rows[index];

                    if row.0 < row.1 {
                        if start < end {
                            row.0 = min(row.0, start);
                            row.1 = max(row.1, end);
                        }
                    } else {
                        *row = (start, end);
                    }
                }
            }
        }
    }
}
