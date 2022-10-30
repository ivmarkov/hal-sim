use serde::*;

use super::{
    display::DisplayMeta,
    gpio::{PinMeta, PinValue},
};

pub type RequestId = usize;

pub const SCREEN_MAX_STRIPE_LEN: usize = 320; // Stripes and overall WebEvents get allocated on the stack, so we want to keep these small
pub const SCREEN_MAX_STRIPE_U8_LEN: usize = SCREEN_MAX_STRIPE_LEN * STRIPE_PIXEL_SIZE;
pub const STRIPE_PIXEL_SIZE: usize = 3;

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
pub enum DisplayUpdate {
    MetaUpdate {
        id: u8,
        meta: Option<DisplayMeta>,
        dropped: bool,
    },
    StripeUpdate(StripeUpdate),
}

impl DisplayUpdate {
    pub fn id(&self) -> u8 {
        match self {
            Self::MetaUpdate { id, .. } => *id,
            Self::StripeUpdate(StripeUpdate { id, .. }) => *id,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct StripeUpdate {
    pub id: u8,
    pub row: u16,
    pub start: u16,
    pub data: heapless::Vec<u8, { SCREEN_MAX_STRIPE_U8_LEN }>,
}
