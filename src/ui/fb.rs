use core::cell::RefCell;

use log::trace;

use wasm_bindgen::Clamped;
use web_sys::ImageData;

use crate::dto::display::*;
use crate::web::{DisplayUpdate, StripeUpdate};

use super::displays::DisplayMsg;

pub struct FrameBuffer {
    width: usize,
    height: usize,
    change: Change,
    screen_fb: Vec<u8>,
}

impl FrameBuffer {
    const PIXEL_SIZE: usize = 4;

    fn new(width: usize, height: usize) -> Self {
        let mut screen = Vec::new();
        screen.reserve_exact(width * height * Self::PIXEL_SIZE);

        let mut screen_updates = Vec::new();
        screen_updates.reserve_exact(height);

        Self {
            width,
            height,
            change: Change {
                created: false,
                dropped: false,
                screen_updates,
            },
            screen_fb: screen,
        }
    }

    pub fn update(msg: &DisplayMsg) {
        match msg {
            DisplayMsg(DisplayUpdate::MetaUpdate { id, meta, .. }) => {
                if let Some(meta) = meta.as_ref() {
                    FBS.with(|fbs| {
                        let mut fbs = fbs.borrow_mut();

                        while fbs.len() <= *id as _ {
                            fbs.push(FrameBuffer::new(meta.width, meta.height));
                        }
                    });
                }
            }
            DisplayMsg(DisplayUpdate::StripeUpdate(update)) => {
                FBS.with(|fbs| {
                    let mut fbs = fbs.borrow_mut();

                    fbs[update.id as usize].update_changes(update);
                });
            }
        }
    }

    pub fn blit<F>(id: u8, full: bool, f: F)
    where
        F: FnMut(&ImageData, usize, usize),
    {
        FBS.with(|fbs| {
            let fb = &mut fbs.borrow_mut()[id as usize];

            fb.blit_fb(full, f);
        });
    }

    fn update_changes(&mut self, update: &StripeUpdate) {
        self.change.update_row(
            update.row as _,
            update.start as _,
            update.start as usize + update.data.len() / Self::PIXEL_SIZE,
        );

        let offset_start =
            (self.width * update.row as usize + update.start as usize) * Self::PIXEL_SIZE;
        let offset_end = offset_start + update.data.len();

        self.extend_screen_fb(offset_end);

        self.screen_fb[offset_start..offset_end].copy_from_slice(&update.data);
    }

    fn blit_fb<F>(&mut self, full: bool, mut f: F)
    where
        F: FnMut(&ImageData, usize, usize),
    {
        if full {
            self.extend_screen_fb(self.width * self.height * Self::PIXEL_SIZE);

            for change in self.change.screen_updates.iter_mut() {
                change.0 = 0;
                change.1 = 0;
            }

            trace!("FB FULL BLIT");

            let image_data = ImageData::new_with_u8_clamped_array_and_sh(
                Clamped(&self.screen_fb),
                self.width as _,
                self.height as _,
            )
            .unwrap();

            f(&image_data, 0, 0);
        } else {
            for (row, change) in self.change.screen_updates.iter_mut().enumerate() {
                if change.0 < change.1 {
                    let offset_start = (self.width * row + change.0) * Self::PIXEL_SIZE;
                    let offset_end = offset_start + (change.1 - change.0) * Self::PIXEL_SIZE;

                    trace!(
                        "FB PARTIAL BLIT: x={}, y={}, w={} h={}",
                        change.0,
                        row,
                        change.1 - change.0,
                        1
                    );

                    let image_data = ImageData::new_with_u8_clamped_array_and_sh(
                        Clamped(&self.screen_fb[offset_start..offset_end]),
                        (change.1 - change.0) as _,
                        1,
                    )
                    .unwrap();

                    f(&image_data, change.0 as _, row as _);

                    change.0 = 0;
                    change.1 = 0;
                }
            }
        }
    }

    fn extend_screen_fb(&mut self, end: usize) {
        while self.screen_fb.len() < end {
            // Fill with black, 0% transparency
            self.screen_fb.push(if self.screen_fb.len() % 4 == 3 {
                255
            } else {
                0
            });
        }
    }
}

thread_local! {
    static FBS: RefCell<Vec<FrameBuffer>> = RefCell::new(Vec::new());
}
