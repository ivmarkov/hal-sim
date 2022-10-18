extern crate alloc;
use alloc::rc::Rc;

use itertools::Itertools;

use yew::prelude::*;

use edge_frame::redust::*;
use edge_frame::util::{get_input_checked, get_input_text};

use crate::dto::gpio::*;
use crate::web::{PinInputUpdate, PinUpdate};

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
    let pins = &*pins_store;

    html! {
        {
            for pins.0
                .iter()
                .enumerate()
                .map(|(index, state)| (index, state.meta.category.as_str()))
                .group_by(|(_, category)| *category)
                .into_iter()
                .map(|(category, group)| {
                    (
                        category.to_string(),
                        group.map(|(index, _)| index as u8).collect::<Vec<_>>(),
                    )
                })
                .map(|(category, pins)| html! {
                    <PinsPanel<R> category={category} pins={pins} projection={props.projection.clone()}/>
                })
        }
    }
}

#[derive(Properties, Clone, PartialEq)]
pub struct PinsPanelProps<R: Reducible2> {
    pub category: String,
    pub pins: Vec<u8>,
    pub projection: Projection<R, PinsState, PinAction>,
}

#[function_component(PinsPanel)]
pub fn pins_panel<R: Reducible2>(props: &PinsPanelProps<R>) -> Html {
    html! {
        <article class="panel is-primary is-size-7">
            <p class="panel-heading">{ props.category.clone() }</p>

            {
                for props.pins.iter().map(|id| html! {
                    <div class="panel-block is-flex">
                        <Pin<R> id={*id} projection={props.projection.clone()}/>
                    </div>
                })
            }
        </article>
    }
}

#[derive(Properties, Clone, PartialEq)]
pub struct PinProps<R: Reducible2> {
    pub id: u8,
    pub projection: Projection<R, PinsState, PinAction>,
}

#[function_component(Pin)]
pub fn pin<R: Reducible2>(props: &PinProps<R>) -> Html {
    let pins_store = use_projection(props.projection.clone());

    let pin = &pins_store.0[props.id as usize];

    let (pin_output_high, pin_output_html) = match pin.value {
        PinValue::Output(output) | PinValue::InputOutput { output, .. } => (
            output,
            if output {
                html! {
                    <span class="mr-2" style="height: 15px; width: 15px; background-color: hsl(348, 100%, 61%); border-radius: 50%; display: inline-block;"/>
                }
            } else {
                html! {
                    <span class="mr-2" style="height: 15px; width: 15px; border: 1px solid #bbb; border-radius: 50%; display: inline-block;"/>
                }
            },
        ),
        _ => (false, {
            html! {
                <span class="mr-2" style="height: 15px; width: 15px; display: inline-block;"/>
            }
        }),
    };

    let (pin_input_high, pin_input_html) = match pin.value {
        PinValue::Input(input) | PinValue::InputOutput { input, .. } => (input, {
            let cb_pins_store = pins_store.clone();
            let id = props.id;

            if pin.meta.pin_type.is_click() {
                let onupdown = Callback::from(move |_| {
                    let pin = &cb_pins_store.0[id as usize];

                    match pin.value {
                        PinValue::Input(input) | PinValue::InputOutput { input, .. } => {
                            cb_pins_store.dispatch(PinAction::InputUpdate(
                                PinInputUpdate::Discrete(id, !input),
                            ));
                        }
                        _ => unreachable!(),
                    }
                });

                html! {
                    <input
                        class="button is-outlined is-small is-primary"
                        style="font-size: 8px;"
                        type="button"
                        value="Click"
                        onmousedown={onupdown.clone()}
                        onmouseup={onupdown}
                    />
                }
            } else {
                let onclick = Callback::from(move |event: MouseEvent| {
                    let pin = &cb_pins_store.0[id as usize];

                    let value = get_input_checked(event.into());

                    match pin.value {
                        PinValue::Input(input) | PinValue::InputOutput { input, .. } => {
                            if input != value {
                                cb_pins_store.dispatch(PinAction::InputUpdate(
                                    PinInputUpdate::Discrete(id, value),
                                ));
                            }
                        }
                        _ => unreachable!(),
                    }
                });

                html! {
                    <>
                        <input
                            class="switch is-rounded is-outlined is-small is-primary p-0 m-0"
                            type="checkbox"
                            id={format!("pin_switch_{}", props.id)}
                            checked={input}
                            {onclick}
                        />
                        <label
                            style="padding-left: 36px; height: 15px; line-height: 10px;"
                            for={format!("pin_switch_{}", props.id)}>{ "" }
                        </label>
                    </>
                }
            }
        }),
        PinValue::Adc(value) => (value > 0, {
            let cb_pins_store = pins_store.clone();
            let id = props.id;

            let oninput = Callback::from(move |event: InputEvent| {
                let pin = &cb_pins_store.0[id as usize];
                let value = str::parse::<u16>(&get_input_text(event.into())).unwrap();

                match pin.value {
                    PinValue::Adc(input) => {
                        if input != value {
                            cb_pins_store.dispatch(PinAction::InputUpdate(PinInputUpdate::Analog(
                                id, value,
                            )));
                        }
                    }
                    _ => unreachable!(),
                }
            });

            html! {
                <>
                    <input class="input ml-4 is-small" type="text" style="width: 50px;" disabled={true} value={value.to_string()}/>
                    <input
                        class="slider is-circle is-small is-primary p-0 ml-2 mr-0 my-0"
                        style="width: 70px;" step="1" min="0" max="100"
                        value={value.to_string()}
                        type="range"
                        {oninput}
                    />
                </>
            }
        }),
        _ => (false, {
            html! {
                <></>
            }
        }),
    };

    html! {
        <>
            { pin_output_html }
            <span
                class={classes!(
                    "is-flex-grow-1",
                    pin_output_high.then_some("has-text-danger"),
                    pin_input_high.then_some("has-text-weight-bold"),
                )}
            >
                { pin.meta.name.clone() }
            </span>
            { pin_input_html }
        </>
    }
}
