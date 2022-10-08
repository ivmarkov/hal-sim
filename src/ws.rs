use std::sync::Mutex;

use embedded_svc::ws::asynch::server::Acceptor;

use edge_net::asynch::{
    channel::{Receiver, Sender},
    ws_channel,
};
use futures::Future;

use crate::display::{Change as DisplayChange, SharedDisplays};
use crate::dto::web::{WebEvent, WebRequest};
use crate::gpio::{Change as PinChange, SharedPins};
use crate::notification::Notification;
use crate::web;

#[cfg(any(
    feature = "ws-max-connections-2",
    not(any(
        feature = "ws-max-connections-4",
        feature = "ws-max-connections-8",
        feature = "ws-max-connections-16"
    ))
))]
pub const WS_MAX_CONNECTIONS: usize = 2;
#[cfg(feature = "ws-max-connections-4")]
pub const WS_MAX_CONNECTIONS: usize = 4;
#[cfg(feature = "ws-max-connections-8")]
pub const WS_MAX_CONNECTIONS: usize = 8;
#[cfg(feature = "ws-max-connections-16")]
pub const WS_MAX_CONNECTIONS: usize = 16;

pub const WS_MAX_FRAME_LEN: usize = 512;

const NOTIF: Notification = Notification::new();
const PIN_MUTEX: Mutex<Vec<PinChange>> = Mutex::new(Vec::new());
const DISPLAY_MUTEX: Mutex<Vec<DisplayChange>> = Mutex::new(Vec::new());

static HANDLERS_NOTIFS: [Notification; WS_MAX_CONNECTIONS] = [NOTIF; WS_MAX_CONNECTIONS];
static HANDLER_PIN_CHANGES: [Mutex<Vec<PinChange>>; WS_MAX_CONNECTIONS] =
    [PIN_MUTEX; WS_MAX_CONNECTIONS];
static HANDLER_DISPLAY_CHANGES: [Mutex<Vec<DisplayChange>>; WS_MAX_CONNECTIONS] =
    [DISPLAY_MUTEX; WS_MAX_CONNECTIONS];

struct WebHandler {
    pins: SharedPins,
    displays: SharedDisplays,
}

impl ws_channel::AcceptorHandler for WebHandler {
    type SendData = WebEvent;

    type ReceiveData = WebRequest;

    type HandleFuture<'a, S, R> = impl Future<Output = Result<(), S::Error>>
    where
        Self: 'a,
        S: Sender<Data = Self::SendData> + 'a,
        R: Receiver<Error = S::Error, Data = Option<Self::ReceiveData>> + 'a,
        S::Error: core::fmt::Debug + 'a;

    fn handle<'a, S, R>(
        &'a self,
        sender: S,
        receiver: R,
        index: usize,
    ) -> Self::HandleFuture<'a, S, R>
    where
        S: Sender<Data = Self::SendData> + 'a,
        R: Receiver<Error = S::Error, Data = Option<Self::ReceiveData>> + 'a,
        S::Error: core::fmt::Debug + 'a,
    {
        async move {
            web::handle(
                sender,
                receiver,
                &self.pins,
                Some(&HANDLER_PIN_CHANGES[index]),
                &self.displays,
                Some(&HANDLER_DISPLAY_CHANGES[index]),
                &HANDLERS_NOTIFS[index],
            )
            .await
        }
    }
}

pub async fn process<A: Acceptor, const W: usize>(
    acceptor: A,
    pins: SharedPins,
    displays: SharedDisplays,
) {
    embassy_futures::select::select(
        ws_channel::accept::<{ WS_MAX_CONNECTIONS }, 1, { WS_MAX_FRAME_LEN }, _, _>(
            acceptor,
            WebHandler {
                pins: pins.clone(),
                displays: displays.clone(),
            },
        ),
        broadcast(pins, displays),
    )
    .await;
}

async fn broadcast(pins: SharedPins, displays: SharedDisplays) {
    loop {
        web::NOTIFY.wait().await;

        {
            let pins = pins.lock().unwrap();

            for changes in &HANDLER_PIN_CHANGES {
                let mut changes = changes.lock().unwrap();

                while changes.len() < pins.len() {
                    changes.push(PinChange::None);
                }

                changes
                    .iter_mut()
                    .enumerate()
                    .for_each(|(index, change)| change.update(pins[index].change()));
            }

            let displays = displays.lock().unwrap();

            for changes in &HANDLER_DISPLAY_CHANGES {
                let mut changes = changes.lock().unwrap();

                while changes.len() < displays.len() {
                    changes.push(DisplayChange::Updated(Vec::new(), false));
                }

                changes
                    .iter_mut()
                    .enumerate()
                    .for_each(|(index, change)| change.update(displays[index].change()));
            }
        }

        for notification in &HANDLERS_NOTIFS {
            notification.notify();
        }
    }
}
