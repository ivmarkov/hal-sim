use std::rc::Rc;

use crate::dto::gpio::*;
use crate::web::{PinInputUpdate, PinUpdate};

use yew::prelude::*;

use edge_frame::redust::*;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PinAction {
    Update(PinUpdate),
    InputUpdate(PinInputUpdate),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PinState {
    pub meta: Rc<PinMeta>,
    pub dropped: bool,
    pub value: PinValue,
}

#[derive(Debug, Default, PartialEq, Eq, Clone)]
pub struct PinsState(Vec<PinState>);

impl Reducible for PinsState {
    type Action = PinAction;

    fn reduce(self: Rc<Self>, action: Self::Action) -> Rc<Self> {
        let mut vec = self.0.clone();

        match action {
            Self::Action::Update(update) => Self({
                while vec.len() <= update.id as _ {
                    vec.push(PinState {
                        meta: Rc::new(Default::default()),
                        dropped: false,
                        value: PinValue::Output(false),
                    });
                }

                let state: &mut PinState = &mut vec[update.id as usize];

                if let Some(meta) = update.meta {
                    state.meta = Rc::new(meta);
                }

                state.dropped = update.dropped;
                state.value = update.value;

                vec
            }),
            Self::Action::InputUpdate(update) => Self(
                vec.iter_mut()
                    .enumerate()
                    .map(|(index, state)| {
                        let mut state = state.clone();

                        if index == update.id() as usize {
                            update.update_value(&mut state.value);
                        }

                        state
                    })
                    .collect::<Vec<_>>(),
            ),
        }
        .into()
    }
}

#[derive(Properties, Clone, PartialEq)]
pub struct PinsProps<R: Reducible2> {
    pub projection: Projection<R, PinsState, PinAction>,
}

#[function_component(Pins)]
pub fn pins<R: Reducible2>(props: &PinsProps<R>) -> Html {
    let pins_store = use_projection(props.projection.clone());

    // TODO: Use Pin component
    html! {
        {format!("{:?}", *pins_store)}
    }
}

#[derive(Properties, Clone, PartialEq)]
pub struct PinProps<R: Reducible2> {
    pub projection: Projection<R, PinState, PinAction>,
}

#[function_component(Pin)]
pub fn pin<R: Reducible2>(props: &PinProps<R>) -> Html {
    let pin_store = use_projection(props.projection.clone());

    html! {
        {format!("{:?}", *pin_store)}
    }
}
