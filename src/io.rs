use log::trace;

use embassy_futures::select::select;

use embassy_sync::blocking_mutex::raw::{NoopRawMutex, RawMutex};
use embassy_sync::mutex::Mutex as AsyncMutex;

use channel_bridge::asynch::{Receiver, Sender};
use channel_bridge::notification::Notification;

use crate::display::Change as DisplayChange;
use crate::gpio::Change as PinChange;
use crate::peripherals::Peripherals;

pub use crate::dto::web::*;

pub(crate) static NOTIFY: Notification = Notification::new();

pub fn peripherals_callback() {
    NOTIFY.notify();
}

pub async fn process<S, R>(sender: S, receiver: R)
where
    S: Sender<Data = WebEvent>,
    R: Receiver<Data = WebRequest, Error = S::Error>,
{
    handle(sender, receiver, &mut None, &mut None, &NOTIFY)
        .await
        .unwrap();
}

pub async fn handle<S, R>(
    sender: S,
    receiver: R,
    pins_changes: &mut Option<Vec<PinChange>>,
    displays_changes: &mut Option<Vec<DisplayChange>>,
    notification: &Notification,
) -> Result<(), S::Error>
where
    S: Sender<Data = WebEvent>,
    R: Receiver<Data = WebRequest, Error = S::Error>,
{
    let sender = AsyncMutex::<NoopRawMutex, _>::new(sender);

    select(
        receive(receiver),
        send(&sender, pins_changes, displays_changes, notification),
    )
    .await;

    Ok(())
}

async fn receive<R>(mut receiver: R) -> Result<(), R::Error>
where
    R: Receiver<Data = WebRequest>,
{
    loop {
        Peripherals::apply(receiver.recv().await?);
    }
}

async fn send<S>(
    sender: &AsyncMutex<impl RawMutex, S>,
    pins_changes: &mut Option<Vec<PinChange>>,
    displays_changes: &mut Option<Vec<DisplayChange>>,
    notification: &Notification,
) -> Result<(), S::Error>
where
    S: Sender<Data = WebEvent>,
{
    loop {
        notification.wait().await;

        let mut sender = sender.lock().await;

        while let Some(event) = Peripherals::fetch(pins_changes, displays_changes) {
            trace!("SENDING: {:?}", event);
            sender.send(event).await?;
        }
    }
}
