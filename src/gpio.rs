use core::convert::Infallible;
use core::marker::PhantomData;

extern crate alloc;
use alloc::sync::Arc;
use channel_bridge::notification::Notification;

use std::sync::Mutex;

use embedded_hal::digital::{ErrorType, InputPin, OutputPin};
use embedded_hal02::digital::v2::{InputPin as InputPin02, OutputPin as OutputPin02};

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

pub(crate) static PINS: Mutex<Vec<PinState>> = Mutex::new(Vec::new());

pub struct Pins {
    id_gen: u8,
    changed: PinsChangedCallback,
}

impl Pins {
    pub(crate) fn new(changed: impl Fn() + 'static) -> Self {
        Self {
            id_gen: 0,
            changed: Arc::new(changed),
        }
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
            let mut states = PINS.lock().unwrap();
            states.push(state);
        }

        Pin::new(id, self.changed.clone())
    }
}

pub type PinsChangedCallback = Arc<dyn Fn()>;

pub struct Pin<MODE> {
    id: u8,
    changed: PinsChangedCallback,
    _mode: PhantomData<MODE>,
}

impl<MODE> Pin<MODE> {
    fn new(id: u8, changed: PinsChangedCallback) -> Self {
        Self {
            id,
            changed,
            _mode: PhantomData,
        }
    }
}

#[cfg(feature = "nightly")]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
enum WaitType {
    LowLevel,
    HighLevel,
    Edge,
}

impl<MODE> Pin<MODE>
where
    MODE: InputMode,
{
    fn is_high(&self) -> bool {
        let guard = PINS.lock().unwrap();

        match guard[self.id as usize].shared.value {
            PinValue::Input(value) => value,
            PinValue::InputOutput { input: value, .. } => value,
            _ => unreachable!(),
        }
    }

    #[cfg(feature = "nightly")]
    async fn wait(&self, wait_type: WaitType) {
        let notif = {
            let guard = PINS.lock().unwrap();

            let notif = guard[self.id as usize].shared.notification();

            notif.reset();

            notif
        };

        match wait_type {
            WaitType::LowLevel => {
                if !self.is_high() {
                    return;
                }
            }
            WaitType::HighLevel => {
                if self.is_high() {
                    return;
                }
            }
            _ => (),
        }

        notif.wait().await;
    }

    pub fn subscribe(&mut self, callback: impl Fn() + Send + 'static) {
        let mut guard = PINS.lock().unwrap();

        guard[self.id as usize].shared.callback = Some(Box::new(callback));
    }

    pub fn unsubscribe(&mut self) {
        let mut guard = PINS.lock().unwrap();

        guard[self.id as usize].shared.callback = None;
    }
}

impl<MODE> Pin<MODE>
where
    MODE: OutputMode,
{
    fn set_output(&mut self, high: bool) {
        let changed = {
            let mut guard = PINS.lock().unwrap();
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
        let guard = PINS.lock().unwrap();

        match guard[self.id as usize].shared.value {
            PinValue::Adc(value) => value,
            _ => unreachable!(),
        }
    }
}

impl<MODE> Drop for Pin<MODE> {
    fn drop(&mut self) {
        {
            let mut guard = PINS.lock().unwrap();

            guard[self.id as usize].shared.dropped = true;
            guard[self.id as usize].change.update(&Change::Updated);
        }

        (self.changed)();
    }
}

impl<MODE> ErrorType for Pin<MODE> {
    type Error = Infallible;
}

impl<MODE> InputPin for Pin<MODE>
where
    MODE: InputMode,
{
    fn is_high(&self) -> Result<bool, Self::Error> {
        Ok(Pin::is_high(self))
    }

    fn is_low(&self) -> Result<bool, Self::Error> {
        Ok(!Pin::is_high(self))
    }
}

#[cfg(feature = "nightly")]
impl<MODE> embedded_hal_async::digital::Wait for Pin<MODE>
where
    MODE: InputMode,
{
    async fn wait_for_high(&mut self) -> Result<(), Self::Error> {
        self.wait(WaitType::HighLevel).await;

        Ok(())
    }

    async fn wait_for_low(&mut self) -> Result<(), Self::Error> {
        self.wait(WaitType::LowLevel).await;

        Ok(())
    }

    async fn wait_for_rising_edge(&mut self) -> Result<(), Self::Error> {
        self.wait(WaitType::Edge).await; // TODO

        Ok(())
    }

    async fn wait_for_falling_edge(&mut self) -> Result<(), Self::Error> {
        self.wait(WaitType::Edge).await; // TODO

        Ok(())
    }

    async fn wait_for_any_edge(&mut self) -> Result<(), Self::Error> {
        self.wait(WaitType::Edge).await;

        Ok(())
    }
}

impl<MODE> InputPin02 for Pin<MODE>
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
    fn set_low(&mut self) -> Result<(), Self::Error> {
        Pin::set_output(self, false);
        Ok(())
    }

    fn set_high(&mut self) -> Result<(), Self::Error> {
        Pin::set_output(self, true);
        Ok(())
    }
}

impl<MODE> OutputPin02 for Pin<MODE>
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
    fn new(name: PinName, category: PinCategory, pin_type: PinType, value: PinValue) -> Self {
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
    notification: Arc<Notification>,
}

impl SharedPin {
    fn new(name: PinName, category: PinCategory, pin_type: PinType, value: PinValue) -> Self {
        Self {
            meta: PinMeta {
                name,
                category,
                pin_type,
            },
            value,
            dropped: false,
            callback: None,
            notification: Arc::new(Notification::new()),
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

    pub fn notification(&self) -> Arc<Notification> {
        self.notification.clone()
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

                self.notification.notify();
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

                self.notification.notify();
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
