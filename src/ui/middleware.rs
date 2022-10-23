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

    use log::info;

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
                info!("Sending request: {:?}", msg);

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
                info!("Received event: {:?}", event);

                dispatch::invoke(event);
            }
        });
    }
}

#[cfg(feature = "middleware-ws")]
mod ws {
    use core::cell::RefCell;
    use core::marker::PhantomData;

    extern crate alloc;
    use alloc::rc::Rc;

    use serde::{de::DeserializeOwned, Serialize};

    use futures::stream::{SplitSink, SplitStream};
    use futures::{SinkExt, StreamExt};

    use gloo_net::websocket::{futures::WebSocket, Message};

    use postcard::*;

    use yew::use_ref;

    pub fn open<R, E>(ws_endpoint: &'static str) -> anyhow::Result<(WebSender<R>, WebReceiver<E>)>
    where
        R: 'static,
        E: 'static,
    {
        open_url(&format!(
            "ws://{}/{}",
            web_sys::window().unwrap().location().host().unwrap(),
            ws_endpoint,
        ))
    }

    fn open_url<R, E>(url: &str) -> anyhow::Result<(WebSender<R>, WebReceiver<E>)> {
        let ws = WebSocket::open(url).map_err(|e| anyhow::anyhow!("{}", e))?;

        let (write, read) = ws.split();

        Ok((
            WebSender(write, PhantomData),
            WebReceiver(read, PhantomData),
        ))
    }

    pub struct WebSender<R>(SplitSink<WebSocket, Message>, PhantomData<fn() -> R>);

    impl<R> WebSender<R>
    where
        R: Serialize,
    {
        pub async fn send(&mut self, request: R) -> anyhow::Result<()> {
            self.0
                .send(Message::Bytes(to_allocvec(&request)?))
                .await
                .map_err(|e| anyhow::anyhow!("{}", e))?;

            Ok(())
        }
    }

    pub struct WebReceiver<E>(SplitStream<WebSocket>, PhantomData<fn() -> E>);

    impl<E> WebReceiver<E>
    where
        E: DeserializeOwned,
    {
        pub async fn recv(&mut self) -> anyhow::Result<E> {
            let message = self
                .0
                .next()
                .await
                .unwrap()
                .map_err(|e| anyhow::anyhow!("{}", e))?;

            let event = match message {
                Message::Bytes(data) => from_bytes(&data)?,
                _ => anyhow::bail!("Invalid message format"),
            };

            Ok(event)
        }
    }
}
