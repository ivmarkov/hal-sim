use std::sync::Mutex;

use embassy_futures::select::select3;

use embassy_sync::blocking_mutex::raw::{NoopRawMutex, RawMutex};
use embassy_sync::mutex::Mutex as AsyncMutex;

use channel_bridge::asynch::{Receiver, Sender};

use crate::display::{Change as DisplayChange, SharedDisplay, SharedDisplays};
use crate::gpio::{Change as PinChange, SharedPin, SharedPins};
use crate::notification::Notification;

pub use crate::dto::web::*;

pub static NOTIFY: Notification = Notification::new();

pub async fn process<S, R>(
    sender: S,
    receiver: R,
    pins: SharedPins,
    displays: SharedDisplays,
) -> Result<(), S::Error>
where
    S: Sender<Data = WebEvent>,
    R: Receiver<Data = Option<WebRequest>, Error = S::Error>,
{
    handle(sender, receiver, &pins, None, &displays, None, &NOTIFY).await
}

pub type HandlerPinChanges = Mutex<Vec<PinChange>>;
pub type HandlerDisplayChanges = Mutex<Vec<DisplayChange>>;

pub async fn handle<S, R>(
    sender: S,
    receiver: R,
    pins: &SharedPins,
    pins_changes: Option<&HandlerPinChanges>,
    displays: &SharedDisplays,
    displays_changes: Option<&HandlerDisplayChanges>,
    notification: &Notification,
) -> Result<(), S::Error>
where
    S: Sender<Data = WebEvent>,
    R: Receiver<Data = Option<WebRequest>, Error = S::Error>,
{
    let sender = AsyncMutex::<NoopRawMutex, _>::new(sender);

    select3(
        receive(receiver, pins),
        send_pin_state(&sender, pins, pins_changes, notification),
        send_display_state(&sender, displays, displays_changes, notification),
    )
    .await;

    Ok(())
}

async fn receive<R>(mut receiver: R, pins: &SharedPins) -> Result<(), R::Error>
where
    R: Receiver<Data = Option<WebRequest>>,
{
    loop {
        if let Some(request) = receiver.recv().await? {
            //info!("[WEB RECEIVE] {:?}", request);

            let mut pins = pins.lock().unwrap();

            match request {
                WebRequest::PinInputUpdate(update) => match update {
                    PinInputUpdate::Discrete(id, high) => {
                        pins[id as usize].pin_mut().set_discrete_input(high);
                    }
                    PinInputUpdate::Analog(id, input) => {
                        pins[id as usize].pin_mut().set_analog_input(input);
                    }
                },
            }
        }
    }
}

async fn send_pin_state<'a, S>(
    sender: &AsyncMutex<impl RawMutex, S>,
    pins: &SharedPins,
    changes: Option<&HandlerPinChanges>,
    notification: &Notification,
) -> Result<(), S::Error>
where
    S: Sender<Data = WebEvent>,
{
    loop {
        notification.wait().await;

        let mut sender = sender.lock().await;

        while let Some(event) = find_pin_change(pins, changes) {
            //info!("[WEB SEND] {:?}", event);
            sender.send(&event).await?;
        }
    }
}

async fn send_display_state<S>(
    sender: &AsyncMutex<impl RawMutex, S>,
    displays: &SharedDisplays,
    changes: Option<&HandlerDisplayChanges>,
    notification: &Notification,
) -> Result<(), S::Error>
where
    S: Sender<Data = WebEvent>,
{
    loop {
        notification.wait().await;

        let mut sender = sender.lock().await;

        while let Some(event) = find_display_change(displays, changes) {
            //info!("[WEB SEND] {:?}", event);
            sender.send(&event).await?;
        }
    }
}

fn find_pin_change(pins: &SharedPins, changes: Option<&HandlerPinChanges>) -> Option<WebEvent> {
    let mut states = pins.lock().unwrap();

    states.iter_mut().enumerate().find_map(|(id, state)| {
        if let Some(changes) = changes.as_ref() {
            let mut changes = changes.lock().unwrap();

            if id < changes.len() {
                consume_pin_change(id as u8, state.pin(), &mut (*changes)[id])
            } else {
                None
            }
        } else {
            let (display, changed_state) = state.split();

            consume_pin_change(id as u8, display, changed_state)
        }
    })
}

fn consume_pin_change(id: u8, pin: &SharedPin, change: &mut PinChange) -> Option<WebEvent> {
    if *change != PinChange::None {
        let event = Some(WebEvent::PinUpdate(PinUpdate {
            id,
            meta: if *change == PinChange::Created {
                Some(pin.meta().clone())
            } else {
                None
            },
            dropped: pin.dropped(),
            value: *pin.value(),
        }));

        change.reset();

        event
    } else {
        None
    }
}

fn find_display_change(
    displays: &SharedDisplays,
    changes: Option<&HandlerDisplayChanges>,
) -> Option<WebEvent> {
    let mut states = displays.lock().unwrap();

    states.iter_mut().enumerate().find_map(|(id, state)| {
        if let Some(changes) = changes.as_ref() {
            let mut changes = changes.lock().unwrap();

            if id < changes.len() {
                consume_display_change(id as u8, state.display(), &mut (*changes)[id])
            } else {
                None
            }
        } else {
            let (display, change) = state.split();

            consume_display_change(id as u8, display, change)
        }
    })
}

fn consume_display_change(
    id: u8,
    display: &SharedDisplay,
    change: &mut DisplayChange,
) -> Option<WebEvent> {
    let event = match change {
        DisplayChange::Created => Some(WebEvent::DisplayUpdate(DisplayUpdate {
            id,
            meta: Some(display.meta().clone()),
            dropped: display.dropped(),
            screen: heapless::Vec::new(), // TODO
        })),
        DisplayChange::Updated(changed_rows, dropped) => {
            if !changed_rows.is_empty() || *dropped {
                Some(WebEvent::DisplayUpdate(DisplayUpdate {
                    id,
                    meta: None,
                    dropped: display.dropped(),
                    screen: heapless::Vec::new(), // TODO
                }))
            } else {
                None
            }
        }
    };

    change.reset();

    event
}
