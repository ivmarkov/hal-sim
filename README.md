# hal-sim - [embedded-hal](https://github.com/rust-embedded/embedded-hal) Simulator

[![CI](https://github.com/ivmarkov/hal-sim/actions/workflows/ci.yml/badge.svg)](https://github.com/ivmarkov/hal-sim/actions/workflows/ci.yml)
[![Documentation](https://docs.rs/hal-sim/badge.svg)](https://docs.rs/hal-sim)

(WIP - UNFINISHED)

This crate simulates a small portion of the embedded-hal traits. Namely:
* GPIO
* ADC

Additionally, it also contains an [embedded-graphics](https://github.com/embedded-graphics/embedded-graphics) Display driver simulator.

The purpose of this simulator is to ease embedded development by enabling cross-compilation of embedded projects on a X86 target (PC) or for WASM.
