#![allow(clippy::let_unit_value)]
#![feature(cfg_version)]
#![cfg_attr(feature = "web", feature(type_alias_impl_trait))]
#![cfg_attr(
    all(feature = "web", version("1.70")),
    feature(impl_trait_in_assoc_type)
)]
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
#[cfg(feature = "ws")]
pub mod ws;
