#![allow(clippy::let_unit_value)]
#![allow(unknown_lints)]
#![allow(async_fn_in_trait)]
#![cfg_attr(feature = "ui", recursion_limit = "1024")]

#[cfg(feature = "sim")]
pub mod adc;
#[cfg(feature = "sim")]
pub mod display;
pub mod dto;
#[cfg(feature = "sim")]
pub mod gpio;
#[cfg(feature = "io")]
pub mod io;
#[cfg(feature = "sim")]
pub mod peripherals;
#[cfg(feature = "ui")]
pub mod ui;
