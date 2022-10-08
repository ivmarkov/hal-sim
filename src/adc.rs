use core::convert::Infallible;
use core::marker::PhantomData;

use embedded_hal::adc::{Channel, OneShot};

use crate::gpio::Pin;

pub trait AdcTrait {
    fn channel() -> u8;
}

pub struct Adc<const ID: u8>(PhantomData<u8>);

impl<const ID: u8> Adc<ID> {
    pub const fn new() -> Self {
        Self(PhantomData)
    }
}

impl<const ID: u8> AdcTrait for Adc<ID> {
    fn channel() -> u8 {
        ID
    }
}

impl<ADC: AdcTrait> Channel<ADC> for Pin<ADC> {
    type ID = u8;

    fn channel() -> Self::ID {
        ADC::channel()
    }
}

impl<const ID: u8> OneShot<Adc<ID>, u16, Pin<Adc<ID>>> for Adc<ID> {
    type Error = Infallible;

    fn read(&mut self, pin: &mut Pin<Adc<ID>>) -> nb::Result<u16, Self::Error> {
        Ok(Pin::get_input(pin))
    }
}
