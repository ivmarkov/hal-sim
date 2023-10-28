# hal-sim - [embedded-hal](https://github.com/rust-embedded/embedded-hal) Simulator

[![CI](https://github.com/ivmarkov/hal-sim/actions/workflows/ci.yml/badge.svg)](https://github.com/ivmarkov/hal-sim/actions/workflows/ci.yml)
![crates.io](https://img.shields.io/crates/v/hal-sim.svg)
[![Documentation](https://docs.rs/hal-sim/badge.svg)](https://docs.rs/hal-sim)

Go to [this page](https://github.com/ivmarkov/ruwm) and click the "DEMO" there link to see the simulator in action!

This crate simulates a small portion of the embedded-hal traits. Namely:
* GPIO (both e-hal V0.2 and e-hal V1.0 traits, including the async `Wait` trait)
* ADC (only e-hal V0.2, as there are no standard traits for ADC in e-hal V1.0 yet)

Additionally, it also contains an [embedded-graphics](https://github.com/embedded-graphics/embedded-graphics) Display driver simulator.

The purpose of this simulator is to ease embedded development by enabling cross-compilation of embedded projects on a X86 target (PC) or for WASM.
