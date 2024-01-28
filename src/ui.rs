use yew::prelude::*;

pub use displays::*;
pub use pins::*;
use yewdux_middleware::use_mcx;

use self::fb::FrameBuffer;

mod displays;
mod fb;
pub mod middleware;
mod pins;

#[derive(Properties, Clone, PartialEq)]
pub struct HalProps {
    #[prop_or_default]
    pub endpoint: Option<String>,

    #[prop_or_default]
    pub children: Children,
}

#[function_component(Hal)]
pub fn hal(props: &HalProps) -> Html {
    let _endpoint = props.endpoint.clone();
    let mcx = use_mcx();

    use_effect_with((), move |_| {
        middleware::init(&mcx, _endpoint.as_deref());

        move || ()
    });

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
