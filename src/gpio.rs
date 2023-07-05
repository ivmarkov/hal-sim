use core::convert::Infallible;
use core::marker::PhantomData;

extern crate alloc;
use alloc::sync::Arc;

use std::sync::Mutex;

use embedded_hal::digital::v2::{InputPin, OutputPin};

use crate::adc::AdcTrait;

pub trait InputMode {}
pub trait OutputMode {}

pub struct Input;
pub struct Output;
pub struct InputOutput;

impl InputMode for Input {}
impl InputMode for InputOutput {}

impl OutputMode for Output {}
impl OutputMode for InputOutput {}

pub use crate::dto::gpio::*;

pub struct Pins {
    id_gen: u8,
    shared: SharedPins,
    changed: PinsChangedCallback,
}

impl Pins {
    pub(crate) fn new(changed: impl Fn() + 'static) -> Self {
        Self {
            id_gen: 0,
            shared: Arc::new(Mutex::new(Vec::new())),
            changed: Arc::new(changed),
        }
    }

    pub(crate) fn shared(&self) -> &SharedPins {
        &self.shared
    }

    pub fn input(
        &mut self,
        name: impl Into<PinName>,
        category: impl Into<PinCategory>,
        value: bool,
    ) -> Pin<Input> {
        self.new_pin(
            name,
            category,
            PinType::Input(ButtonType::Toggle),
            PinValue::Input(value),
        )
    }

    pub fn input_click(
        &mut self,
        name: impl Into<PinName>,
        category: impl Into<PinCategory>,
        value: bool,
    ) -> Pin<Input> {
        self.new_pin(
            name,
            category,
            PinType::InputOutput(ButtonType::Click),
            PinValue::Input(value),
        )
    }

    pub fn output(
        &mut self,
        name: impl Into<PinName>,
        category: impl Into<PinCategory>,
        value: bool,
    ) -> Pin<Output> {
        self.new_pin(name, category, PinType::Output, PinValue::Output(value))
    }

    pub fn input_output(
        &mut self,
        name: impl Into<PinName>,
        category: impl Into<PinCategory>,
        input: bool,
        output: bool,
    ) -> Pin<InputOutput> {
        self.new_pin(
            name,
            category,
            PinType::InputOutput(ButtonType::Toggle),
            PinValue::InputOutput { input, output },
        )
    }

    pub fn input_output_click(
        &mut self,
        name: impl Into<PinName>,
        category: impl Into<PinCategory>,
        input: bool,
        output: bool,
    ) -> Pin<InputOutput> {
        self.new_pin(
            name,
            category,
            PinType::InputOutput(ButtonType::Click),
            PinValue::InputOutput { input, output },
        )
    }

    pub fn adc<ADC>(
        &mut self,
        name: impl Into<PinName>,
        category: impl Into<PinCategory>,
        value: u16,
    ) -> Pin<ADC>
    where
        ADC: AdcTrait,
    {
        self.adc_range(name, category, 0, 3300, value)
    }

    pub fn adc_range<ADC>(
        &mut self,
        name: impl Into<PinName>,
        category: impl Into<PinCategory>,
        min: u16,
        max: u16,
        value: u16,
    ) -> Pin<ADC>
    where
        ADC: AdcTrait,
    {
        self.new_pin(
            name,
            category,
            PinType::Analog(min, max),
            PinValue::Adc(value),
        )
    }

    fn new_pin<MODE>(
        &mut self,
        name: impl Into<PinName>,
        category: impl Into<PinCategory>,
        pin_type: PinType,
        value: PinValue,
    ) -> Pin<MODE> {
        let id = self.id_gen;
        self.id_gen += 1;

        let state = PinState::new(name.into(), category.into(), pin_type, value);

        {
            let mut states = self.shared.lock().unwrap();
            states.push(state);
        }

        Pin::new(id, self.shared.clone(), self.changed.clone())
    }
}

pub type SharedPins = Arc<Mutex<Vec<PinState>>>;
pub type PinsChangedCallback = Arc<dyn Fn()>;

pub struct Pin<MODE> {
    id: u8,
    pins: SharedPins,
    changed: PinsChangedCallback,
    _mode: PhantomData<MODE>,
}

impl<MODE> Pin<MODE> {
    const fn new(id: u8, pins: SharedPins, changed: PinsChangedCallback) -> Self {
        Self {
            id,
            pins,
            changed,
            _mode: PhantomData,
        }
    }
}

impl<MODE> Pin<MODE>
where
    MODE: InputMode,
{
    fn is_high(&self) -> bool {
        let guard = self.pins.lock().unwrap();

        match guard[self.id as usize].shared.value {
            PinValue::Input(value) => value,
            PinValue::InputOutput { input: value, .. } => value,
            _ => unreachable!(),
        }
    }

    pub fn subscribe(&mut self, callback: impl Fn() + Send + 'static) {
        let mut guard = self.pins.lock().unwrap();

        guard[self.id as usize].shared.callback = Some(Box::new(callback));
    }

    pub fn unsubscribe(&mut self) {
        let mut guard = self.pins.lock().unwrap();

        guard[self.id as usize].shared.callback = None;
    }
}

impl<MODE> Pin<MODE>
where
    MODE: OutputMode,
{
    fn set_output(&mut self, high: bool) {
        let changed = {
            let mut guard = self.pins.lock().unwrap();
            let pin = &mut guard[self.id as usize];

            match &mut pin.shared.value {
                PinValue::Output(output) | PinValue::InputOutput { output, .. } => {
                    if *output != high {
                        *output = high;
                        pin.change.update(&Change::Updated);

                        true
                    } else {
                        false
                    }
                }
                _ => unreachable!(),
            }
        };

        if changed {
            (self.changed)()
        }
    }
}

impl<MODE> Pin<MODE>
where
    MODE: AdcTrait,
{
    pub(crate) fn get_input(&self) -> u16 {
        let guard = self.pins.lock().unwrap();

        match guard[self.id as usize].shared.value {
            PinValue::Adc(value) => value,
            _ => unreachable!(),
        }
    }
}

impl<MODE> Drop for Pin<MODE> {
    fn drop(&mut self) {
        {
            let mut guard = self.pins.lock().unwrap();

            guard[self.id as usize].shared.dropped = true;
            guard[self.id as usize].change.update(&Change::Updated);
        }

        (self.changed)();
    }
}

impl<MODE> InputPin for Pin<MODE>
where
    MODE: InputMode,
{
    type Error = Infallible;

    fn is_high(&self) -> Result<bool, Self::Error> {
        Ok(Pin::is_high(self))
    }

    fn is_low(&self) -> Result<bool, Self::Error> {
        Ok(!Pin::is_high(self))
    }
}

impl<MODE> OutputPin for Pin<MODE>
where
    MODE: OutputMode,
{
    type Error = Infallible;

    fn set_low(&mut self) -> Result<(), Self::Error> {
        Pin::set_output(self, false);
        Ok(())
    }

    fn set_high(&mut self) -> Result<(), Self::Error> {
        Pin::set_output(self, true);
        Ok(())
    }
}

pub struct PinState {
    shared: SharedPin,
    change: Change,
}

impl PinState {
    const fn new(name: PinName, category: PinCategory, pin_type: PinType, value: PinValue) -> Self {
        Self {
            shared: SharedPin::new(name, category, pin_type, value),
            change: Change::Created,
        }
    }

    pub fn change(&self) -> &Change {
        &self.change
    }

    pub fn pin(&self) -> &SharedPin {
        &self.shared
    }

    pub fn pin_mut(&mut self) -> &mut SharedPin {
        &mut self.shared
    }

    pub fn split(&mut self) -> (&SharedPin, &mut Change) {
        (&self.shared, &mut self.change)
    }
}

pub struct SharedPin {
    meta: PinMeta,
    value: PinValue,
    dropped: bool,
    callback: Option<Box<dyn Fn() + Send>>,
}

impl SharedPin {
    const fn new(name: PinName, category: PinCategory, pin_type: PinType, value: PinValue) -> Self {
        Self {
            meta: PinMeta {
                name,
                category,
                pin_type,
            },
            value,
            dropped: false,
            callback: None,
        }
    }

    pub fn meta(&self) -> &PinMeta {
        &self.meta
    }

    pub fn value(&self) -> &PinValue {
        &self.value
    }

    pub fn dropped(&self) -> bool {
        self.dropped
    }

    pub fn set_discrete_input(&mut self, high: bool) {
        if !self.dropped {
            let changed = match &mut self.value {
                PinValue::Input(value)
                | PinValue::InputOutput {
                    input: value,
                    output: _,
                } => {
                    if *value != high {
                        *value = high;
                        true
                    } else {
                        false
                    }
                }
                _ => unreachable!(),
            };

            if changed {
                if let Some(callback) = self.callback.as_ref() {
                    (callback)();
                }
            }
        }
    }

    pub fn set_analog_input(&mut self, value: u16) {
        if !self.dropped {
            let changed = match &mut self.value {
                PinValue::Adc(adc) => {
                    if *adc != value {
                        *adc = value;
                        true
                    } else {
                        false
                    }
                }
                _ => unreachable!(),
            };

            if changed {
                if let Some(callback) = self.callback.as_ref() {
                    (callback)();
                }
            }
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Change {
    None,
    Created,
    Updated,
}

impl Change {
    pub fn reset(&mut self) {
        *self = Self::None;
    }

    pub fn update(&mut self, other: &Change) {
        if *self != Self::Created {
            *self = *other;
        }
    }
}
