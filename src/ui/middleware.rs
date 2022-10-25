use core::fmt::Debug;

use log::{log, Level};

use yewdux_middleware::*;

#[cfg(feature = "middleware-local")]
pub use local::*;

#[cfg(feature = "middleware-ws")]
pub use ws::*;

pub fn log_msg<M, D>(level: Level) -> impl Fn(M, D)
where
    M: Debug,
    D: Dispatch<M>,
{
    move |msg, dispatch| {
        log!(level, "Dispatching message: {:?}", msg);

        dispatch.invoke(msg);
    }
}

pub fn log_store<S, M, D>(level: Level) -> impl Fn(M, D)
where
    S: Store + Debug,
    M: Reducer<S> + Debug,
    D: Dispatch<M>,
{
    move |msg, dispatch| {
        log!(level, "Store (before): {:?}", yewdux::dispatch::get::<S>());

        dispatch.invoke(msg);

        log!(level, "Store (after): {:?}", yewdux::dispatch::get::<S>());
    }
}

#[cfg(feature = "middleware-local")]
mod local {
    use core::cell::RefCell;
    use core::fmt::Debug;

    extern crate alloc;
    use alloc::rc::Rc;

    use log::trace;

    use wasm_bindgen_futures::spawn_local;

    use embassy_sync::channel;

    use yewdux_middleware::*;

    pub fn send<M>(sender: impl Into<channel::DynamicSender<'static, M>>) -> impl Fn(M)
    where
        M: Debug + 'static,
    {
        let sender = Rc::new(RefCell::new(sender.into()));

        move |msg| {
            let sender = sender.clone();

            spawn_local(async move {
                trace!("Sending request: {:?}", msg);

                sender.borrow_mut().send(msg).await;
            });
        }
    }

    pub fn receive<M>(receiver: impl Into<channel::DynamicReceiver<'static, M>>)
    where
        M: Debug + 'static,
    {
        let receiver = receiver.into();

        spawn_local(async move {
            loop {
                let event = receiver.recv().await;
                trace!("Received event: {:?}", event);

                dispatch::invoke(event);
            }
        });
    }
}

#[cfg(feature = "middleware-ws")]
mod ws {
    use core::cell::RefCell;
    use core::fmt::Debug;

    extern crate alloc;
    use alloc::rc::Rc;

    use serde::{de::DeserializeOwned, Serialize};

    use log::trace;

    use futures::stream::{SplitSink, SplitStream};
    use futures::{SinkExt, StreamExt};

    use gloo_net::websocket::{futures::WebSocket, Message};

    use postcard::to_allocvec;

    use wasm_bindgen::JsError;
    use wasm_bindgen_futures::spawn_local;

    use yewdux_middleware::dispatch;

    pub fn open(
        ws_endpoint: &str,
    ) -> Result<(SplitSink<WebSocket, Message>, SplitStream<WebSocket>), JsError> {
        open_url(&format!(
            "ws://{}/{}",
            web_sys::window().unwrap().location().host().unwrap(),
            ws_endpoint,
        ))
    }

    fn open_url(
        url: &str,
    ) -> Result<(SplitSink<WebSocket, Message>, SplitStream<WebSocket>), JsError> {
        let ws = WebSocket::open(url)?;

        Ok(ws.split())
    }

    pub fn send<M>(sender: SplitSink<WebSocket, Message>) -> impl Fn(M)
    where
        M: Serialize + Debug + 'static,
    {
        let sender = Rc::new(RefCell::new(sender));

        move |msg| {
            let sender = sender.clone();

            spawn_local(async move {
                trace!("Sending request: {:?}", msg);

                sender
                    .borrow_mut()
                    .send(Message::Bytes(to_allocvec(&msg).unwrap()))
                    .await
                    .unwrap();
            });
        }
    }

    pub fn receive<M>(mut receiver: SplitStream<WebSocket>)
    where
        M: DeserializeOwned + Debug + 'static,
    {
        spawn_local(async move {
            loop {
                let event = receiver.next().await.unwrap().unwrap();
                trace!("Received event: {:?}", event);

                dispatch::invoke(event);
            }
        });
    }
}
