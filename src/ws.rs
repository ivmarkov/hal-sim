use std::sync::Mutex;

use channel_bridge::notification::Notification;

use crate::display::{Change as DisplayChange, SharedDisplays};
use crate::gpio::{Change as PinChange, SharedPins};
use crate::web;

#[cfg(feature = "embedded-svc")]
pub use embedded_svc_impl::*;

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

#[allow(clippy::declare_interior_mutable_const)]
const NOTIF: Notification = Notification::new();

#[allow(clippy::declare_interior_mutable_const)]
const PIN_MUTEX: Mutex<Vec<PinChange>> = Mutex::new(Vec::new());
#[allow(clippy::declare_interior_mutable_const)]
const DISPLAY_MUTEX: Mutex<Vec<DisplayChange>> = Mutex::new(Vec::new());

static HANDLERS_NOTIFS: [Notification; WS_MAX_CONNECTIONS] = [NOTIF; WS_MAX_CONNECTIONS];
static HANDLER_PIN_CHANGES: [Mutex<Vec<PinChange>>; WS_MAX_CONNECTIONS] =
    [PIN_MUTEX; WS_MAX_CONNECTIONS];
static HANDLER_DISPLAY_CHANGES: [Mutex<Vec<DisplayChange>>; WS_MAX_CONNECTIONS] =
    [DISPLAY_MUTEX; WS_MAX_CONNECTIONS];

pub async fn broadcast(pins: &SharedPins, displays: &SharedDisplays) {
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

#[cfg(feature = "embedded-svc")]
pub mod embedded_svc_impl {
    use core::future::Future;

    use embedded_svc::ws::asynch::server::Acceptor;

    use edge_net::asynch::channel::{Receiver, Sender};
    use edge_net::asynch::ws_channel;

    use crate::display::SharedDisplays;
    use crate::dto::web::{WebEvent, WebRequest};
    use crate::gpio::SharedPins;
    use crate::peripherals::SharedPeripherals;
    use crate::web;

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
                    Some(&super::HANDLER_PIN_CHANGES[index]),
                    &self.displays,
                    Some(&super::HANDLER_DISPLAY_CHANGES[index]),
                    &super::HANDLERS_NOTIFS[index],
                )
                .await
            }
        }
    }

    pub async fn accept<A: Acceptor, const W: usize>(
        acceptor: A,
        shared_peripherals: SharedPeripherals,
    ) {
        embassy_futures::select::select(
            ws_channel::accept::<{ super::WS_MAX_CONNECTIONS }, 1, {super:: WS_MAX_FRAME_LEN }, _, _>(
                acceptor,
                WebHandler {
                    pins: shared_peripherals.0.clone(),
                    displays: shared_peripherals.1.clone(),
                },
            ),
            super::broadcast(&shared_peripherals.0, &shared_peripherals.1),
        )
        .await;
    }
}
