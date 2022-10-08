use serde::*;

use super::{
    display::DisplayMeta,
    gpio::{PinMeta, PinValue},
};

pub type RequestId = usize;

pub const RECT_MAX_DATA_SIZE: usize = 1024 * 1024;
pub const SCREEN_MAX_RECT: usize = 6;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WebRequest {
    id: RequestId,
    payload: WebRequestPayload,
}

impl WebRequest {
    pub fn new(id: RequestId, payload: WebRequestPayload) -> Self {
        Self { id, payload }
    }

    pub fn id(&self) -> RequestId {
        self.id
    }

    pub fn payload(&self) -> &WebRequestPayload {
        &self.payload
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum WebRequestPayload {
    PinInputUpdate(u8, bool),
    PinAnalogUpdate(u8, u16),
}

#[allow(clippy::large_enum_variant)]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum WebEvent {
    PinUpdate { id: u8, update: PinUpdate },
    DisplayUpdate { id: u8, update: DisplayUpdate },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PinUpdate {
    pub meta: Option<PinMeta>,
    pub dropped: bool,
    pub value: PinValue,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ScreenUpdate {
    pub rect: (usize, usize, usize, usize),
    pub data: heapless::Vec<u8, { RECT_MAX_DATA_SIZE }>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DisplayUpdate {
    pub meta: Option<DisplayMeta>,
    pub dropped: bool,
    pub screen: heapless::Vec<ScreenUpdate, { SCREEN_MAX_RECT }>,
}
