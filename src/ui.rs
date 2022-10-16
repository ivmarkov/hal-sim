extern crate alloc;
use alloc::rc::Rc;

use yew::prelude::*;

use edge_frame::middleware::*;
use edge_frame::redust::*;

use displays::*;
use middleware::*;
use pins::*;
use state::*;

mod displays;
mod middleware;
mod pins;
mod state;

#[cfg(all(feature = "middleware-ws", feature = "middleware-local"))]
compile_error!("Only one of the features `middleware-ws` and `middleware-local` can be enabled.");

#[cfg(not(any(feature = "middleware-ws", feature = "middleware-local")))]
compile_error!("One of the features `middleware-ws` or `middleware-local` must be enabled.");

#[derive(Properties, Clone, Default, Debug, PartialEq)]
pub struct HalProps {
    #[prop_or_default]
    pub children: Children,
}

#[function_component(Hal)]
pub fn hal(props: &HalProps) -> Html {
    #[cfg(feature = "middleware-ws")]
    let channel = channel("ws");

    #[cfg(feature = "middleware-local")]
    let channel = channel(move || {
        (
            comm::REQUEST_QUEUE.sender().into(),
            comm::EVENT_QUEUE.receiver().into(),
        )
    });

    let store = apply_middleware(
        use_store(|| Rc::new(AppState::new())),
        to_request,
        from_event,
        channel,
    )
    .unwrap();

    let content = html! {
        <ContextProvider<UseStoreHandle<AppState>> context={store}>
            <div class="columns">
                <div class="column">
                    <Displays<AppState> projection={AppState::displays()}/>
                </div>
                <div class="column">
                    <Pins<AppState> projection={AppState::pins()}/>
                </div>
            </div>
        </ContextProvider<UseStoreHandle<AppState>>>
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
