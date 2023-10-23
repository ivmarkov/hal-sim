use core::fmt::{self, Debug};

use std::sync::Mutex;

extern crate alloc;
use alloc::sync::Arc;

use crate::adc::Adc;
use crate::display::{Change as DisplayChange, Displays, SharedDisplay, DISPLAYS};
use crate::gpio::{Change as PinChange, Pins, SharedPin, PINS};

pub use crate::dto::*;

#[derive(Debug)]
pub enum TakeError {
    AlreadyTaken,
}

impl fmt::Display for TakeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AlreadyTaken => write!(f, "Already taken Error"),
        }
    }
}

#[cfg(feature = "std")]
impl ::std::error::Error for TakeError {}

static TAKEN: Mutex<bool> = Mutex::new(false);

pub struct Peripherals {
    pub pins: Pins,
    pub displays: Displays,
    pub adc0: Adc<0>,
    pub adc1: Adc<1>,
    pub adc2: Adc<2>,
    pub adc3: Adc<3>,
}

impl Peripherals {
    pub fn take(changed: impl Fn() + 'static) -> Result<Self, TakeError> {
        let mut taken = TAKEN.lock().unwrap();

        if *taken {
            Err(TakeError::AlreadyTaken)
        } else {
            let changed = Arc::new(changed);
            let changed_pins = changed.clone();
            let changed_displays = changed;

            let this = Self {
                pins: Pins::new(move || changed_pins()),
                displays: Displays::new(move || changed_displays()),
                adc0: Adc::new(),
                adc1: Adc::new(),
                adc2: Adc::new(),
                adc3: Adc::new(),
            };

            *taken = true;

            Ok(this)
        }
    }

    pub fn apply(request: UpdateRequest) {
        let mut pins = PINS.lock().unwrap();

        let UpdateRequest::PinInputUpdate(update) = request;

        match update {
            PinInputUpdate::Discrete(id, high) => {
                pins[id as usize].pin_mut().set_discrete_input(high);
            }
            PinInputUpdate::Analog(id, input) => {
                pins[id as usize].pin_mut().set_analog_input(input);
            }
        }
    }

    pub fn fetch(
        pins_changes: &mut Option<Vec<PinChange>>,
        displays_changes: &mut Option<Vec<DisplayChange>>,
    ) -> Option<UpdateEvent> {
        if let Some(event) = Self::find_pin_change(pins_changes) {
            Some(event)
        } else {
            Self::find_display_change(displays_changes)
        }
    }

    fn find_pin_change(changes: &mut Option<Vec<PinChange>>) -> Option<UpdateEvent> {
        let mut states = PINS.lock().unwrap();

        states.iter_mut().enumerate().find_map(|(id, state)| {
            if let Some(changes) = changes.as_deref_mut() {
                if id < changes.len() {
                    Self::consume_pin_change(id as u8, state.pin(), &mut (*changes)[id])
                } else {
                    None
                }
            } else {
                let (display, changed_state) = state.split();

                Self::consume_pin_change(id as u8, display, changed_state)
            }
        })
    }

    fn consume_pin_change(id: u8, pin: &SharedPin, change: &mut PinChange) -> Option<UpdateEvent> {
        if *change != PinChange::None {
            let event = Some(UpdateEvent::PinUpdate(PinUpdate {
                id,
                meta: if *change == PinChange::Created {
                    Some(pin.meta().clone())
                } else {
                    None
                },
                dropped: pin.dropped(),
                value: *pin.value(),
            }));

            change.reset();

            event
        } else {
            None
        }
    }

    fn find_display_change(changes: &mut Option<Vec<DisplayChange>>) -> Option<UpdateEvent> {
        let mut states = DISPLAYS.lock().unwrap();

        states.iter_mut().enumerate().find_map(|(id, state)| {
            if let Some(changes) = changes.as_deref_mut() {
                if id < changes.len() {
                    Self::consume_display_change(id as u8, state.display(), &mut (*changes)[id])
                } else {
                    None
                }
            } else {
                let (display, change) = state.split();

                Self::consume_display_change(id as u8, display, change)
            }
        })
    }

    fn consume_display_change(
        id: u8,
        display: &SharedDisplay,
        change: &mut DisplayChange,
    ) -> Option<UpdateEvent> {
        if change.created || change.dropped {
            let event = Some(UpdateEvent::DisplayUpdate(DisplayUpdate::MetaUpdate {
                id,
                meta: change.created.then_some(display.meta().clone()),
                dropped: display.dropped(),
            }));

            change.created = false;
            change.dropped = false;

            event
        } else {
            let changed_row = change
                .screen_updates
                .iter_mut()
                .enumerate()
                .find_map(|(row, (start, end))| (*start < *end).then_some((row, start, end)));

            if let Some((row, start, end)) = changed_row {
                let event = Some(UpdateEvent::DisplayUpdate(DisplayUpdate::StripeUpdate(
                    StripeUpdate {
                        id,
                        row: row as _,
                        start: *start as _,
                        data: {
                            let row_data = &display.buffer()[row * display.meta().width..];

                            row_data[*start..*end].iter()
                                .flat_map(|pixel| {
                                    let bytes = pixel.to_be_bytes();
                                    [bytes[1], bytes[2], bytes[3]]
                                })
                                .collect::<heapless::Vec<_, { crate::dto::SCREEN_MAX_STRIPE_U8_LEN }>>()
                        },
                    },
                )));

                *start = 0;
                *end = 0;

                event
            } else {
                None
            }
        }
    }
}
