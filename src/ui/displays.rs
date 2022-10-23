use core::fmt::Debug;

extern crate alloc;
use alloc::rc::Rc;

use log::warn;

use super::yewdux_middleware::*;
use yew::prelude::*;

use wasm_bindgen_futures::spawn_local;

use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel;

use wasm_bindgen::{Clamped, JsCast};
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement, ImageData};

use crate::dto::display::*;
use crate::web::{DisplayUpdate, StripeUpdate, WebEvent, WebRequest};

#[derive(Debug)]
pub struct DisplayMsg(pub DisplayUpdate);

impl DisplayMsg {
    pub fn from_event(event: &WebEvent) -> Option<Self> {
        match event {
            WebEvent::DisplayUpdate(update) => Some(Self(update.clone())),
            _ => None,
        }
    }
}

impl<'a> From<&'a DisplayMsg> for Option<WebRequest> {
    fn from(_value: &'a DisplayMsg) -> Self {
        None
    }
}

impl Reducer<DisplaysState> for DisplayMsg {
    fn apply(&self, mut store: Rc<DisplaysState>) -> Rc<DisplaysState> {
        let state = Rc::make_mut(&mut store);
        let vec = &mut state.0;

        match self {
            Self(DisplayUpdate::MetaUpdate { id, meta, dropped }) => {
                while vec.len() <= *id as _ {
                    vec.push(DisplayState {
                        meta: Rc::new(Default::default()),
                        dropped: false,
                        render_cycle: 0,
                    });
                }

                let display: &mut DisplayState = &mut vec[*id as usize];
                if let Some(meta) = meta {
                    display.meta = Rc::new(meta.clone());
                }

                display.dropped = *dropped;
            }
            Self(DisplayUpdate::StripeUpdate(StripeUpdate { id, .. })) => {
                let display: &mut DisplayState = &mut vec[*id as usize];

                display.render_cycle += 1;
            }
        }

        store
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DisplayState {
    pub meta: Rc<DisplayMeta>,
    pub dropped: bool,
    pub render_cycle: u32,
}

#[derive(Debug, Default, PartialEq, Eq, Clone, Store)]
pub struct DisplaysState(Vec<DisplayState>);

#[function_component(Displays)]
pub fn displays() -> Html {
    let displays = use_store::<DisplaysState>();
    let displays = &*displays;

    html! {
        {
            for displays.0.iter().enumerate().map(|(index, _)| {
                html! {
                    <Display id={index as u8}/>
                }
            })
        }
    }
}

#[derive(Properties, Clone, PartialEq)]
pub struct DisplayProps {
    pub id: u8,
}

#[function_component(Display)]
pub fn display(props: &DisplayProps) -> Html {
    let displays = use_store::<DisplaysState>();
    let display = &displays.0[props.id as usize];

    html! {
        <article class="panel is-primary is-size-7">
            <p class="panel-heading">{ display.meta.name.clone() }{" "}{ display.meta.width }{"x"}{ display.meta.height }</p>
            <div class="panel-block">
                <DisplayCanvas
                    id={props.id}
                    width={display.meta.width}
                    height={display.meta.height}
                />
            </div>
        </article>
    }
}

#[derive(Properties, Clone, PartialEq)]
pub struct DisplayCanvasProps {
    pub id: u8,
    pub width: usize,
    pub height: usize,
}

#[function_component(DisplayCanvas)]
pub fn display_canvas(props: &DisplayCanvasProps) -> Html {
    let node_ref = use_node_ref();

    {
        let id = props.id;
        let width = props.width;

        use_effect(move || {
            spawn_local(async move {
                loop {
                    let update = DISPLAY_QUEUE[id as usize].recv().await;

                    warn!("About to draw: {:?}", update);

                    let canvas = node_ref.cast::<HtmlCanvasElement>().unwrap();

                    let ctx: CanvasRenderingContext2d = canvas
                        .get_context("2d")
                        .unwrap()
                        .unwrap()
                        .dyn_into()
                        .unwrap();

                    let image_data = ImageData::new_with_u8_clamped_array_and_sh(
                        Clamped(&update.data),
                        width as _,
                        1,
                    )
                    .unwrap();

                    ctx.put_image_data(&image_data, update.start as _, update.row as _)
                        .unwrap();

                    warn!("Drawing complete");
                }
            });

            || ()
        });
    }

    html! {
        <canvas width={props.width.to_string()} height={props.height.to_string()}/>
    }
}

const CHANNEL: channel::Channel<CriticalSectionRawMutex, StripeUpdate, 1> = channel::Channel::new();
static DISPLAY_QUEUE: [channel::Channel<CriticalSectionRawMutex, StripeUpdate, 1>; 8] =
    [CHANNEL; 8];

pub fn draw<D>(msg: DisplayMsg, dispatch: D)
where
    D: Dispatch<DisplayMsg>,
{
    warn!("Draw dispatching: {:?}", msg);

    match &msg {
        DisplayMsg(DisplayUpdate::StripeUpdate(update)) => {
            let update = update.clone();

            spawn_local(async move {
                warn!("About to send draw update: {:?}", update);

                DISPLAY_QUEUE[update.id as usize].send(update).await;
            });
        }
        _ => (),
    }

    dispatch.invoke(msg);
}
