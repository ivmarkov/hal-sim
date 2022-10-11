#![feature(cfg_version)]
#![cfg_attr(
    all(feature = "web", not(version("1.65"))),
    feature(generic_associated_types)
)]
#![cfg_attr(feature = "web", feature(type_alias_impl_trait))]

#[cfg(feature = "sim")]
pub mod adc;
#[cfg(feature = "sim")]
pub mod display;
pub mod dto;
#[cfg(feature = "sim")]
pub mod gpio;
pub mod notification;
#[cfg(feature = "ui")]
pub mod ui;
#[cfg(feature = "web")]
pub mod web;
#[cfg(feature = "ws")]
pub mod ws;
