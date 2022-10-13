use core::fmt::Debug;

use serde::*;

pub type DisplayName = heapless::String<64>;

#[derive(Debug, Default, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct DisplayMeta {
    pub name: DisplayName,
    pub width: usize,
    pub height: usize,
}
