use core::fmt::Debug;

extern crate alloc;
use alloc::rc::Rc;

use log::trace;

use yew::prelude::*;
use yewdux_middleware::*;

use wasm_bindgen::JsCast;
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement};

use crate::dto::display::*;
use crate::web::{DisplayUpdate, WebEvent, WebRequest};

use super::fb::{FrameBuffer, FrameBufferStore};

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

impl Reducer<DisplaysStore> for DisplayMsg {
    fn apply(self, mut store: Rc<DisplaysStore>) -> Rc<DisplaysStore> {
        let state = Rc::make_mut(&mut store);
        let vec = &mut state.0;

        if let Self(DisplayUpdate::MetaUpdate { id, meta, dropped }) = self {
            while vec.len() <= id as _ {
                vec.push(DisplayState {
                    meta: Rc::new(Default::default()),
                    dropped: false,
                });
            }

            let display: &mut DisplayState = &mut vec[id as usize];
            if let Some(meta) = meta {
                display.meta = Rc::new(meta.clone());
            }

            display.dropped = dropped;
        }

        store
    }
}

#[derive(Debug, Default, PartialEq, Eq, Clone, Store)]
pub struct DisplaysStore(Vec<DisplayState>);

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DisplayState {
    pub meta: Rc<DisplayMeta>,
    pub dropped: bool,
}

#[function_component(Displays)]
pub fn displays() -> Html {
    let displays = use_store_value::<DisplaysStore>();
    let displays = &*displays;

    html! {
        {
            for displays.0.iter().enumerate().map(|(index, _)| {
                html! {
                    <Display id={index as u8} key={index}/>
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
    let displays = use_store_value::<DisplaysStore>();
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
    let _fbs = use_store_value::<FrameBufferStore>(); // To receive change notifications

    let node_ref = use_node_ref();
    let ctx_ref = use_mut_ref(|| None);

    {
        let node_ref = node_ref.clone();
        let ctx_ref = ctx_ref.clone();

        let id = props.id;
        let width = props.width;
        let height = props.height;

        use_effect_with(node_ref, move |node_ref| {
            if ctx_ref.borrow().is_none() {
                trace!("[FB DRAW] CONTEXT CREATED");

                let ctx = create_draw_context(node_ref, width, height);
                FrameBuffer::blit(id, true, |image_data, x, y| {
                    ctx.put_image_data(image_data, x as _, y as _).unwrap();

                    trace!("[FB DRAW] SCREEN FULL BLIT");
                });

                *ctx_ref.borrow_mut() = Some(ctx);
            }

            move || {
                trace!("[FB DRAW] CONTEXT DROPPED");
                *ctx_ref.borrow_mut() = None;
            }
        });
    }

    {
        let id = props.id;

        use_effect(move || {
            if let Some(ctx) = ctx_ref.borrow().as_ref() {
                trace!("[FB DRAW] SCREEN BLIT START");

                FrameBuffer::blit(id, false, |image_data, x, y| {
                    ctx.put_image_data(image_data, x as _, y as _).unwrap();

                    trace!(
                        "[FB DRAW] SCREEN BLIT: x={} y={} w={} h={}",
                        x,
                        y,
                        image_data.width(),
                        image_data.height()
                    );
                });
            }

            move || {}
        });
    }

    html! {
        <canvas ref={node_ref} width={props.width.to_string()} height={props.height.to_string()}/>
    }
}

fn create_draw_context(
    node_ref: &NodeRef,
    width: usize,
    height: usize,
) -> CanvasRenderingContext2d {
    let canvas = node_ref.cast::<HtmlCanvasElement>().unwrap();

    let ctx: CanvasRenderingContext2d = canvas
        .get_context("2d")
        .unwrap()
        .unwrap()
        .dyn_into()
        .unwrap();

    ctx.set_fill_style(&"#000000".into());
    ctx.fill_rect(0 as _, 0 as _, width as _, height as _);

    ctx
}
