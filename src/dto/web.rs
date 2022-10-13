use serde::*;

use super::{
    display::DisplayMeta,
    gpio::{PinMeta, PinValue},
};

pub type RequestId = usize;

pub const RECT_MAX_DATA_SIZE: usize = 1024 * 1024;
pub const SCREEN_MAX_RECT: usize = 6;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum WebRequest {
    PinInputUpdate(PinInputUpdate),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum PinInputUpdate {
    Discrete(u8, bool),
    Analog(u8, u16),
}

impl PinInputUpdate {
    pub fn id(&self) -> u8 {
        match self {
            Self::Discrete(id, _) => *id,
            Self::Analog(id, _) => *id,
        }
    }

    pub fn update_value(&self, pin_value: &mut PinValue) {
        match self {
            Self::Discrete(_, value) => match pin_value {
                PinValue::Input(input) | PinValue::InputOutput { input, .. } => *input = *value,
                _ => panic!(),
            },
            Self::Analog(_, value) => match pin_value {
                PinValue::Adc(input) => *input = *value,
                _ => panic!(),
            },
        }
    }
}

#[allow(clippy::large_enum_variant)]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum WebEvent {
    PinUpdate(PinUpdate),
    DisplayUpdate(DisplayUpdate),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PinUpdate {
    pub id: u8,
    pub meta: Option<PinMeta>,
    pub dropped: bool,
    pub value: PinValue,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DisplayUpdate {
    pub id: u8,
    pub meta: Option<DisplayMeta>,
    pub dropped: bool,
    pub screen: heapless::Vec<ScreenUpdate, { SCREEN_MAX_RECT }>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ScreenUpdate {
    pub rect: (usize, usize, usize, usize),
    pub data: heapless::Vec<u8, { RECT_MAX_DATA_SIZE }>,
}
