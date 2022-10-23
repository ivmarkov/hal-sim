use self::yewdux_middleware::*;
use log::Level;
use yew::prelude::*;

use crate::web::{WebEvent, WebRequest};
use displays::*;
use pins::*;

use edgeframe_middleware::{log_msg, log_store};

pub mod edgeframe_middleware;
pub mod yewdux_middleware;

mod displays;
mod pins;

#[cfg(all(feature = "middleware-ws", feature = "middleware-local"))]
compile_error!("Only one of the features `middleware-ws` and `middleware-local` can be enabled.");

#[cfg(not(any(feature = "middleware-ws", feature = "middleware-local")))]
compile_error!("One of the features `middleware-ws` or `middleware-local` must be enabled.");

#[derive(Properties, Clone, PartialEq)]
pub struct HalProps {
    #[prop_or_default]
    pub children: Children,
}

#[function_component(Hal)]
pub fn hal(props: &HalProps) -> Html {
    use_effect_with_deps(
        move |_| {
            init_middleware();

            move || ()
        },
        (),
    );

    let content = html! {
        <div class="columns">
            <div class="column">
                <Displays/>
            </div>
            <div class="column">
                <Pins/>
            </div>
        </div>
    };

    if props.children.is_empty() {
        content
    } else {
        html! {
            <div class="columns m-4">
                <div class="column">
                    { content }
                </div>
                <div class="column">
                    { for props.children.iter() }
                </div>
            </div>
        }
    }
}

#[function_component(App)]
pub fn app() -> Html {
    html! {
        <Hal/>
    }
}

fn init_middleware() {
    #[cfg(feature = "middleware-ws")]
    let (sender, receiver) = {
        let (sender, receiver) = edgeframe_middleware::open("/ws").unwrap().split();

        edgeframe_middleware::receive(receiver);
        crate::ui::yewdux_middleware::dispatch::register(edgeframe_middleware::send(sender));
    };

    #[cfg(feature = "middleware-local")]
    let (sender, receiver) = (comm::REQUEST_QUEUE.sender(), comm::REQUEST_QUEUE.receiver());

    // Dispatch WebRequest messages => send to backend
    dispatch::register(edgeframe_middleware::send(sender));

    // Receive from backend => dispatch WebEvent messages
    edgeframe_middleware::receive(receiver);

    // Dispatch WebEvent messages => redispatch as PinMsg or DisplayMsg messages
    dispatch::register::<WebEvent, _>(|event| {
        if let Some(msg) = PinMsg::from_event(&event) {
            dispatch::invoke(msg);
        } else if let Some(msg) = DisplayMsg::from_event(&event) {
            dispatch::invoke(msg);
        }
    });

    dispatch::register(store_dispatch::<PinsState, PinMsg>());
    dispatch::register(store_dispatch::<DisplaysState, DisplayMsg>().fuse(Rc::new(draw)));
}

use core::fmt::Debug;
use std::rc::Rc;

// Set the middleware for each store type (PinsState & DisplaysState)
fn store_dispatch<S, M>() -> impl Dispatch<M> + Clone
where
    S: Store + Debug,
    M: Reducer<S> + Debug + 'static,
    for<'a> &'a M: Into<Option<WebRequest>>,
{
    // Update store
    yewdux::dispatch::apply
        // PinMsg => WebRequest
        .fuse(as_request)
        // Log store before/after dispatching
        .fuse(Rc::new(log_store(Level::Trace)))
        // Log msg before dispatching
        .fuse(Rc::new(log_msg(Level::Info)))
}

fn as_request<M, D>(msg: M, dispatch: D)
where
    M: Debug + 'static,
    for<'a> &'a M: Into<Option<WebRequest>>,
    D: Dispatch<M>,
{
    if let Some(request) = (&msg).into() {
        dispatch::invoke(request);
    }

    dispatch.invoke(msg);
}

#[cfg(feature = "middleware-local")]
pub mod comm {
    use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, channel};

    use crate::dto::web::*;

    pub(crate) static REQUEST_QUEUE: channel::Channel<CriticalSectionRawMutex, WebRequest, 1> =
        channel::Channel::new();
    pub(crate) static EVENT_QUEUE: channel::Channel<CriticalSectionRawMutex, WebEvent, 1> =
        channel::Channel::new();

    pub fn sender() -> channel::DynamicSender<'static, WebEvent> {
        EVENT_QUEUE.sender().into()
    }

    pub fn receiver() -> channel::DynamicReceiver<'static, WebRequest> {
        REQUEST_QUEUE.receiver().into()
    }
}
