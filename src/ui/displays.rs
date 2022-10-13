use std::rc::Rc;

use crate::dto::display::*;
use crate::web::DisplayUpdate;

use yew::prelude::*;

use edge_frame::redust::*;

pub type DisplayAction = ValueAction<DisplayUpdate>;

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

    // TODO: Use Display component
    html! {
        {format!("{:?}", *displays_store)}
    }
}

#[derive(Properties, Clone, PartialEq)]
pub struct DisplayProps<R: Reducible2> {
    pub projection: Projection<R, DisplayState, DisplayAction>,
}

#[function_component(Display)]
pub fn display<R: Reducible2>(props: &DisplayProps<R>) -> Html {
    let display_store = use_projection(props.projection.clone());

    html! {
        {format!("{:?}", *display_store)}
    }
}
