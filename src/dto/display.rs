use core::cmp::{max, min};
use core::fmt::Debug;

use serde::*;

pub type DisplayName = heapless::String<64>;

#[derive(Debug, Default, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct DisplayMeta {
    pub name: DisplayName,
    pub width: usize,
    pub height: usize,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Change {
    pub created: bool,
    pub dropped: bool,
    pub screen_updates: Vec<(usize, usize)>,
}

impl Change {
    pub fn update(&mut self, other: &Self) {
        self.created |= other.created;
        self.dropped |= other.dropped;

        for (i, other_row) in other.screen_updates.iter().enumerate() {
            self.update_row(i, other_row.0, other_row.1);
        }
    }

    pub fn update_row(&mut self, index: usize, start: usize, end: usize) {
        if start < end {
            while self.screen_updates.len() <= index {
                self.screen_updates.push((0, 0));
            }

            let row = &mut self.screen_updates[index];

            Self::update_stripe(&mut row.0, &mut row.1, start, end);
        }
    }

    fn update_stripe(s_start: &mut usize, s_end: &mut usize, start: usize, end: usize) {
        if start < end {
            if *s_start < *s_end {
                *s_start = min(*s_start, start);
                *s_end = max(*s_end, end);
            } else {
                *s_start = start;
                *s_end = end;
            }
        }
    }
}
