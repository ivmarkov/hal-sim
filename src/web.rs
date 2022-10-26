use std::sync::Mutex;

use embassy_futures::select::select;

use embassy_sync::blocking_mutex::raw::{NoopRawMutex, RawMutex};
use embassy_sync::mutex::Mutex as AsyncMutex;

use channel_bridge::asynch::{Receiver, Sender};
use channel_bridge::notification::Notification;

use crate::display::{Change as DisplayChange, SharedDisplay, SharedDisplays};
use crate::gpio::{Change as PinChange, SharedPin, SharedPins};

pub use crate::dto::web::*;
use crate::peripherals::SharedPeripherals;

pub(crate) static NOTIFY: Notification = Notification::new();

pub fn peripherals_callback() {
    NOTIFY.notify();
}

pub async fn process<S, R>(sender: S, receiver: R, shared_peripherals: SharedPeripherals)
where
    S: Sender<Data = WebEvent>,
    R: Receiver<Data = Option<WebRequest>, Error = S::Error>,
{
    handle(
        sender,
        receiver,
        &shared_peripherals.0,
        None,
        &shared_peripherals.1,
        None,
        &NOTIFY,
    )
    .await
    .unwrap();
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

    select(
        receive(receiver, pins),
        send_state(
            &sender,
            pins,
            pins_changes,
            displays,
            displays_changes,
            notification,
        ),
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

async fn send_state<'a, S>(
    sender: &AsyncMutex<impl RawMutex, S>,
    pins: &SharedPins,
    pins_changes: Option<&HandlerPinChanges>,
    displays: &SharedDisplays,
    displays_changes: Option<&HandlerDisplayChanges>,
    notification: &Notification,
) -> Result<(), S::Error>
where
    S: Sender<Data = WebEvent>,
{
    loop {
        notification.wait().await;

        let mut sender = sender.lock().await;

        while let Some(event) = find_pin_change(pins, pins_changes) {
            sender.send(event).await?;
        }

        while let Some(event) = find_display_change(displays, displays_changes) {
            sender.send(event).await?;
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
    if change.created || change.dropped {
        let event = Some(WebEvent::DisplayUpdate(DisplayUpdate::MetaUpdate {
            id,
            meta: change.created.then_some(display.meta().clone()),
            dropped: display.dropped(),
        }));

        change.created = false;
        change.dropped = false;

        event
    } else {
        let changed_row = change
            .screen_updates
            .iter_mut()
            .enumerate()
            .find_map(|(row, (start, end))| (*start < *end).then_some((row, start, end)));

        if let Some((row, start, end)) = changed_row {
            let event = Some(WebEvent::DisplayUpdate(DisplayUpdate::StripeUpdate(
                StripeUpdate {
                    id,
                    row: row as _,
                    start: *start as _,
                    data: {
                        let row_data = &display.buffer()[row * display.meta().width..];

                        row_data[*start..*end].iter()
                        .flat_map(|pixel| {
                            let mut bytes = pixel.to_le_bytes();
                            bytes[3] = 255; // Bytes are RGBA; set to 0% transparency

                            bytes
                        })
                        .collect::<heapless::Vec<_, { crate::dto::web::SCREEN_MAX_STRIPE_U8_LEN }>>()
                    },
                },
            )));

            *start = 0;
            *end = 0;

            event
        } else {
            None
        }
    }
}
