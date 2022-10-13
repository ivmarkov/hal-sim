use edge_frame::redust::*;

use crate::web::{WebEvent, WebRequest};

use super::{
    pins::PinAction,
    state::{AppAction, AppState},
};

pub fn from_event(_store: &UseStoreHandle<AppState>, event: &WebEvent) -> Option<AppAction> {
    match event {
        WebEvent::PinUpdate(update) => Some(AppAction::Pin(PinAction::Update(update.clone()))),
        WebEvent::DisplayUpdate(update) => {
            Some(AppAction::Display(ValueAction::Update(update.clone())))
        }
    }
}

pub fn to_request(action: &AppAction) -> Option<WebRequest> {
    match action {
        AppAction::Pin(PinAction::InputUpdate(update)) => {
            Some(WebRequest::PinInputUpdate(update.clone()))
        }
        _ => None,
    }
}
