extern crate alloc;
use alloc::rc::Rc;

use crate::dto::display::*;
use crate::web::DisplayUpdate;

use yew::prelude::*;

use edge_frame::redust::*;

pub type DisplayAction = ValueAction<Box<DisplayUpdate>>;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DisplayState {
    pub meta: Rc<DisplayMeta>,
    pub dropped: bool,
    pub screen: Vec<u32>,
}

#[derive(Debug, Default, PartialEq, Eq, Clone)]
pub struct DisplaysState(Vec<DisplayState>);

impl Reducible for DisplaysState {
    type Action = DisplayAction;

    fn reduce(self: Rc<Self>, action: Self::Action) -> Rc<Self> {
        match action {
            Self::Action::Update(update) => Self({
                let mut vec = self.0.clone();
                while vec.len() <= update.id as _ {
                    vec.push(DisplayState {
                        meta: Rc::new(Default::default()),
                        dropped: false,
                        screen: Vec::new(),
                    });
                }

                let state: &mut DisplayState = &mut vec[update.id as usize];

                if let Some(meta) = update.meta {
                    state.meta = Rc::new(meta);
                }

                state.dropped = update.dropped;
                //state.value = update.value;

                vec
            }),
        }
        .into()
    }
}

#[derive(Properties, Clone, PartialEq)]
pub struct DisplaysProps<R: Reducible2> {
    pub projection: Projection<R, DisplaysState, DisplayAction>,
}

#[function_component(Displays)]
pub fn displays<R: Reducible2>(props: &DisplaysProps<R>) -> Html {
    let displays_store = use_projection(props.projection.clone());
    let displays = &*displays_store;

    html! {
        {
            for displays.0.iter().enumerate().map(|(index, _)| {
                html! {
                    <Display<R> id={index as u8} projection={props.projection.clone()}/>
                }
            })
        }
    }
}

#[derive(Properties, Clone, PartialEq)]
pub struct DisplayProps<R: Reducible2> {
    pub id: u8,
    pub projection: Projection<R, DisplaysState, DisplayAction>,
}

#[function_component(Display)]
pub fn display<R: Reducible2>(props: &DisplayProps<R>) -> Html {
    let displays_store = use_projection(props.projection.clone());
    let display = &displays_store.0[props.id as usize];

    html! {
        <article class="panel is-primary">
            <p class="panel-heading">{ display.meta.name.clone() } { display.meta.width }{"x"}{ display.meta.height }</p>
            <div class="panel-block">
                <canvas width={display.meta.width.to_string()} height={display.meta.height.to_string()}/>
            </div>
        </article>
    }
}
