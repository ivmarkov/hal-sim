extern crate alloc;
use alloc::rc::Rc;

use yew::prelude::*;

use edge_frame::redust::*;

use super::displays::{DisplayAction, DisplaysState};
use super::pins::{PinAction, PinsState};

#[derive(Debug, Clone, PartialEq)]
pub enum AppAction {
    Pin(PinAction),
    Display(DisplayAction),
}

#[derive(Default, Debug, Clone, PartialEq)]
pub struct AppState {
    pub pins: Rc<PinsState>,
    pub displays: Rc<DisplaysState>,
}

impl AppState {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn pins() -> Projection<AppState, PinsState, PinAction> {
        Projection::new(|state: &AppState| &*state.pins, AppAction::Pin)
    }

    pub fn displays() -> Projection<AppState, DisplaysState, DisplayAction> {
        Projection::new(|state: &AppState| &*state.displays, AppAction::Display)
    }
}

impl Reducible for AppState {
    type Action = AppAction;

    fn reduce(self: Rc<Self>, action: Self::Action) -> Rc<Self> {
        match action {
            AppAction::Pin(action) => Self {
                pins: self.pins.clone().reduce(action),
                ..(*self).clone()
            },
            AppAction::Display(action) => Self {
                displays: self.displays.clone().reduce(action),
                ..(*self).clone()
            },
        }
        .into()
    }
}
