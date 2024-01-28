extern crate alloc;
use alloc::rc::Rc;

use itertools::Itertools;
use web_sys::HtmlInputElement;

use yew::prelude::*;
use yewdux::use_store_value;
use yewdux_middleware::*;

use crate::dto::gpio::*;
use crate::dto::*;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PinMsg {
    Update(PinUpdate),
    InputUpdate(PinInputUpdate),
}

impl PinMsg {
    pub fn from_event(event: &UpdateEvent) -> Option<Self> {
        match event {
            UpdateEvent::PinUpdate(update) => Some(Self::Update(update.clone())),
            _ => None,
        }
    }
}

impl<'a> From<&'a PinMsg> for Option<UpdateRequest> {
    fn from(value: &'a PinMsg) -> Self {
        match value {
            PinMsg::InputUpdate(update) => Some(UpdateRequest::PinInputUpdate(update.clone())),
            _ => None,
        }
    }
}

impl Reducer<PinsStore> for PinMsg {
    fn apply(self, mut store: Rc<PinsStore>) -> Rc<PinsStore> {
        let state = Rc::make_mut(&mut store);
        let vec = &mut state.0;

        match self {
            Self::Update(update) => {
                while vec.len() <= update.id as _ {
                    vec.push(PinState {
                        meta: Rc::new(Default::default()),
                        dropped: false,
                        value: PinValue::Output(false),
                    });
                }

                let state: &mut PinState = &mut vec[update.id as usize];

                if let Some(meta) = &update.meta {
                    state.meta = Rc::new(meta.clone());
                }

                state.dropped = update.dropped;
                state.value = update.value;
            }
            Self::InputUpdate(update) => {
                for (id, pin) in vec.iter_mut().enumerate() {
                    if id == update.id() as usize {
                        update.update_value(&mut pin.value);
                    }
                }
            }
        }

        store
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Store)]
pub struct PinsStore(Vec<PinState>);

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PinState {
    pub meta: Rc<PinMeta>,
    pub dropped: bool,
    pub value: PinValue,
}

#[function_component(Pins)]
pub fn pins() -> Html {
    let pins = use_store_value::<PinsStore>();

    let pins = &*pins;

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
                        group.map(|(index, _)| (index as u8)).collect::<Vec<_>>(),
                    )
                })
                .map(|(category, pins)| html! {
                    <PinsPanel category={category} pins={pins}/>
                })
        }
    }
}

#[derive(Properties, Clone, PartialEq)]
pub struct PinsPanelProps {
    pub category: String,
    pub pins: Vec<u8>,
}

#[function_component(PinsPanel)]
pub fn pins_panel(props: &PinsPanelProps) -> Html {
    html! {
        <article class="panel is-primary is-size-7">
            <p class="panel-heading">{ props.category.clone() }</p>

            {
                for props.pins.iter().map(|id| html! {
                    <div class="panel-block is-flex">
                        <Pin id={*id}/>
                    </div>
                })
            }
        </article>
    }
}

#[derive(Properties, Clone, PartialEq)]
pub struct PinProps {
    pub id: u8,
}

#[function_component(Pin)]
pub fn pin(props: &PinProps) -> Html {
    let mcx = use_mcx();

    let pins = use_store_value::<PinsStore>();

    let pin: &PinState = &pins.0[props.id as usize];

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
            let id = props.id;
            let pins = pins.clone();

            if pin.meta.pin_type.is_click() {
                let onupdown = Callback::from(move |_| {
                    let pin: &PinState = &pins.0[id as usize];

                    match pin.value {
                        PinValue::Input(input) | PinValue::InputOutput { input, .. } => {
                            mcx.invoke(PinMsg::InputUpdate(PinInputUpdate::Discrete(id, !input)));
                        }
                        _ => unreachable!(),
                    }
                });

                html! {
                    <input
                        class="button is-outlined is-small is-primary"
                        style="font-size: 9px;"
                        type="button"
                        value="Click"
                        onmousedown={onupdown.clone()}
                        onmouseup={onupdown}
                    />
                }
            } else {
                let id = props.id;

                let onclick = Callback::from(move |event: MouseEvent| {
                    let value = event.target_unchecked_into::<HtmlInputElement>().checked();
                    let pin: &PinState = &pins.0[id as usize];

                    match pin.value {
                        PinValue::Input(input) | PinValue::InputOutput { input, .. } => {
                            if input != value {
                                mcx.invoke(PinMsg::InputUpdate(PinInputUpdate::Discrete(
                                    id, value,
                                )));
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
            let id = props.id;
            let pins = pins.clone();

            let oninput = Callback::from(move |event: InputEvent| {
                let value = str::parse::<u16>(
                    event
                        .target_unchecked_into::<HtmlInputElement>()
                        .value()
                        .as_str(),
                )
                .unwrap();
                let pin: &PinState = &pins.0[id as usize];

                match pin.value {
                    PinValue::Adc(input) => {
                        if input != value {
                            mcx.invoke(PinMsg::InputUpdate(PinInputUpdate::Analog(id, value)));
                        }
                    }
                    _ => unreachable!(),
                }
            });

            let (min, max) = match pin.meta.pin_type {
                PinType::Analog(min, max) => (min, max),
                _ => unreachable!(),
            };

            html! {
                <>
                    <input class="input ml-4 is-small py-0" type="text" style="width: 50px;" disabled={true} value={value.to_string()}/>
                    <input
                        class="slider is-circle is-small is-primary p-0 ml-2 mr-0 my-0"
                        style="font-size: 9px; width: 70px;" step="1" min={min.to_string()} max={max.to_string()}
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
                { pin.meta.name.as_str() }
            </span>
            { pin_input_html }
        </>
    }
}
