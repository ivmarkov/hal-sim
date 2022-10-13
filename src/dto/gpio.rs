use core::fmt::Debug;

use serde::*;

pub type PinName = heapless::String<64>;
pub type PinCategory = heapless::String<64>;

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum PinType {
    Input,
    Output,
    InputOutput,
    Analog,
}

impl Default for PinType {
    fn default() -> Self {
        Self::Output
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct PinMeta {
    pub name: PinName,
    pub category: PinCategory,
    pub pin_type: PinType,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum PinValue {
    Input(bool),
    Output(bool),
    InputOutput { input: bool, output: bool },
    Adc(u16),
}

impl PinValue {
    pub const fn pin_type(&self) -> PinType {
        match self {
            Self::Input(_) => PinType::Input,
            Self::InputOutput { .. } => PinType::InputOutput,
            Self::Output(_) => PinType::Output,
            Self::Adc(_) => PinType::Analog,
        }
    }
}
