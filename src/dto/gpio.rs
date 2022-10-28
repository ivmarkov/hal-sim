use core::fmt::Debug;

use serde::*;

pub type PinName = heapless::String<64>;
pub type PinCategory = heapless::String<64>;

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ButtonType {
    Toggle,
    Click,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum PinType {
    Input(ButtonType),
    Output,
    InputOutput(ButtonType),
    Analog(u16, u16),
}

impl PinType {
    pub fn is_click(&self) -> bool {
        matches!(
            self,
            Self::Input(ButtonType::Click) | Self::InputOutput(ButtonType::Click)
        )
    }
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
