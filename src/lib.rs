#![allow(clippy::let_unit_value)]
#![allow(stable_features)]
#![allow(unknown_lints)]
#![cfg_attr(feature = "nightly", feature(async_fn_in_trait))]
#![cfg_attr(feature = "nightly", allow(async_fn_in_trait))]
#![cfg_attr(feature = "nightly", feature(impl_trait_projections))]
#![cfg_attr(feature = "ui", recursion_limit = "1024")]

#[cfg(feature = "sim")]
pub mod adc;
#[cfg(feature = "sim")]
pub mod display;
pub mod dto;
#[cfg(feature = "sim")]
pub mod gpio;
#[cfg(feature = "sim")]
pub mod peripherals;
#[cfg(feature = "ui")]
pub mod ui;
#[cfg(feature = "web")]
pub mod web;
