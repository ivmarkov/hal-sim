use core::fmt::Debug;

use yewdux_middleware::*;

use log::{log, Level};

extern crate alloc;
use alloc::rc::Rc;

use super::*;
use crate::dto::*;

#[cfg(feature = "nightly")]
pub use io::*;

pub fn log_msg<M, D>(level: Level) -> impl Fn(M, D)
where
    M: Debug,
    D: MiddlewareDispatch<M>,
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
    D: MiddlewareDispatch<M>,
{
    move |msg, dispatch| {
        log!(level, "Store (before): {:?}", yewdux::dispatch::get::<S>());

        dispatch.invoke(msg);

        log!(level, "Store (after): {:?}", yewdux::dispatch::get::<S>());
    }
}

pub fn hook<S, R>(send: S, receive: R)
where
    S: Fn(UpdateRequest) + 'static,
    R: FnOnce() + 'static,
{
    // Dispatch UpdateRequest messages => send to backend
    dispatch::register(send);

    // Dispatch UpdateEvent messages => redispatch as PinMsg or DisplayMsg messages
    dispatch::register::<UpdateEvent, _>(|event| {
        if let Some(msg) = PinMsg::from_event(&event) {
            dispatch::invoke(msg);
        } else if let Some(msg) = DisplayMsg::from_event(&event) {
            FrameBuffer::update(&msg);
            dispatch::invoke(msg);
        }
    });

    dispatch::register(store_dispatch::<PinsStore, PinMsg>());
    dispatch::register(store_dispatch::<DisplaysStore, DisplayMsg>());

    // Receive from backend => dispatch UpdateEvent messages
    receive();
}

// Set the middleware for each store type (PinsState & DisplaysState)
fn store_dispatch<S, M>() -> impl MiddlewareDispatch<M> + Clone
where
    S: Store + Debug,
    M: Reducer<S> + Debug + 'static,
    for<'a> &'a M: Into<Option<UpdateRequest>>,
{
    // Update store
    dispatch::store
        // PinMsg => UpdateRequest
        .fuse(as_request)
        // Log store before/after dispatching
        .fuse(Rc::new(log_store(Level::Trace)))
        // Log msg before dispatching
        .fuse(Rc::new(log_msg(Level::Trace)))
}

fn as_request<M, D>(msg: M, dispatch: D)
where
    M: Debug + 'static,
    for<'a> &'a M: Into<Option<UpdateRequest>>,
    D: MiddlewareDispatch<M>,
{
    if let Some(request) = (&msg).into() {
        dispatch::invoke(request);
    }

    dispatch.invoke(msg);
}

#[cfg(feature = "nightly")]
mod io {
    use core::fmt::Debug;

    use channel_bridge::asynch::ws::{WsWebReceiver, WsWebSender};
    use channel_bridge::asynch::{Receiver, Sender};

    use log::trace;

    use embassy_sync::{
        blocking_mutex::raw::{CriticalSectionRawMutex, NoopRawMutex},
        channel,
        mutex::Mutex,
    };

    use wasm_bindgen_futures::spawn_local;

    use yewdux_middleware::*;

    extern crate alloc;
    use alloc::rc::Rc;

    use super::*;

    use gloo_net::websocket::futures::WebSocket;

    use futures::StreamExt;

    pub fn init(endpoint: Option<&str>) {
        if let Some(endpoint) = endpoint {
            let (sender, receiver) = WebSocket::open(&format!(
                "ws://{}/{}",
                web_sys::window().unwrap().location().host().unwrap(),
                endpoint,
            ))
            .unwrap_or_else(|_| panic!("Failed to open websocket"))
            .split();

            hook(send(WsWebSender::new(sender)), move || {
                receive(WsWebReceiver::<UpdateEvent>::new(receiver))
            });
        } else {
            pub(crate) static REQUEST_QUEUE: channel::Channel<
                CriticalSectionRawMutex,
                UpdateRequest,
                1,
            > = channel::Channel::new();
            pub(crate) static EVENT_QUEUE: channel::Channel<
                CriticalSectionRawMutex,
                UpdateEvent,
                1,
            > = channel::Channel::new();

            hook(send(REQUEST_QUEUE.sender()), move || {
                receive(EVENT_QUEUE.receiver())
            });

            process_local(EVENT_QUEUE.sender(), REQUEST_QUEUE.receiver());
        }
    }

    fn send<S>(sender: S) -> impl Fn(S::Data)
    where
        S: Sender + 'static,
        S::Data: Debug + 'static,
    {
        let sender = Rc::new(Mutex::<NoopRawMutex, _>::new(sender));

        move |msg| {
            let sender = sender.clone();

            spawn_local(async move {
                trace!("Sending request: {:?}", msg);

                let mut guard = sender.lock().await;

                guard.send(msg).await.unwrap();
            });
        }
    }

    fn receive<R>(mut receiver: R)
    where
        R: Receiver + 'static,
        R::Data: Debug + 'static,
    {
        spawn_local(async move {
            loop {
                let event = receiver.recv().await.unwrap();
                trace!("Received event: {:?}", event);

                dispatch::invoke(event);
            }
        });
    }

    fn process_local<S, R>(sender: S, receiver: R)
    where
        S: Sender<Data = UpdateEvent> + 'static,
        R: Receiver<Data = UpdateRequest, Error = S::Error> + 'static,
    {
        spawn_local(async move {
            crate::io::process(sender, receiver).await;
        });
    }
}
