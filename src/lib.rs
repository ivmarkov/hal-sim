#![feature(cfg_version)]
#![cfg_attr(
    all(feature = "nightly", not(version("1.65"))),
    feature(generic_associated_types)
)]
#![cfg_attr(feature = "nightly", feature(type_alias_impl_trait))]

pub mod adc;
pub mod display;
pub mod dto;
pub mod gpio;
pub mod notification;
pub mod web;
pub mod ws;
