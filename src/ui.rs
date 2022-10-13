use std::rc::Rc;

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

#[function_component(App)]
pub fn app() -> Html {
    #[cfg(feature = "middleware-ws")]
    let channel = channel("ws");

    #[cfg(feature = "middleware-local")]
    let channel = channel(
        comm::REQUEST_QUEUE.sender().into(),
        comm::EVENT_QUEUE.receiver().into(),
    );

    let store = apply_middleware(
        use_store(|| Rc::new(AppState::new())),
        to_request,
        from_event,
        channel,
    )
    .unwrap();

    html! {
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
    }
}

#[cfg(feature = "middleware-local")]
pub mod comm {
    use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, channel};

    use hal_sim::dto::web::*;

    pub(crate) static REQUEST_QUEUE: channel::Channel<CriticalSectionRawMutex, WebRequest, 1> =
        channel::Channel::new();
    pub(crate) static EVENT_QUEUE: channel::Channel<CriticalSectionRawMutex, WebEvent, 1> =
        channel::Channel::new();

    pub fn channel() -> (
        channel::DynamicSender<'static, WebEvent>,
        channel::DynamicReceiver<'static, WebRequest>,
    ) {
        (EVENT_QUEUE.sender().into(), REQUEST_QUEUE.receiver().into())
    }
}
