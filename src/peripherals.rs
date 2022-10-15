use std::sync::Mutex;

extern crate alloc;
use alloc::sync::Arc;

use crate::adc::Adc;
use crate::display::{Displays, SharedDisplays};
use crate::gpio::{Pins, SharedPins};

pub type SharedPeripherals = (SharedPins, SharedDisplays);

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
    pub fn take(changed: impl Fn() + 'static) -> Result<Self, ()> {
        let mut taken = TAKEN.lock().unwrap();

        if *taken {
            Err(())
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

    pub fn shared(&self) -> SharedPeripherals {
        (self.pins.shared().clone(), self.displays.shared().clone())
    }
}
