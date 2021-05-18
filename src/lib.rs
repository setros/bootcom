//! Bootcom is a utility to simplify kernel development for custom boards by
//! enabling the kernel to be pushed to a bootloader over the serial port
//! connection. This is a simple and fast process for rapid iteration over the
//! kernel development and testing.
//!
//! The approach is similar to what has been implemented in
//! [`raspbootin`](https://github.com/mrvn/raspbootin) or in
//! [`rust-embedded`](https://github.com/rust-embedded/rust-raspberrypi-OS-tutorials),
//! but using rust and enhancing the boot protocol with more flexibility and
//! interaction.
//!
//! Bootcom offers interactive selection menus to chose the serial port to be
//! used, can easily switch from one port to another, handle disconnection and
//! various errors, all without restarting.
//!
//! Most of the functionality in `bootcom` is implemented as state machines.
//! State machines are implemented in terms of **states** and **transitions**
//! between them with the following characteristics:
//!
//! * Can only be in one state at any time.
//! * Each state can have its own associated data if needed.
//! * It is possible to have some shared data between **all** states.
//! * Transitions between states are triggered via typed **events**and follow
//!   defined semantics.
//! * Only explicitly defined transitions should be permitted and as many errors
//!   should be detected at **compile-time**.
//! * Transitioning from one state to another consumes the original state and
//!   renders it unusable. Any transition back to that state would create a new
//!   state.
//! * Data can be transferred from one state to the next by attaching it to the
//!   transition event. Such data is statically defined as part of the event
//!   type.
//!
//! The implementation of state transitions leverages `rust`'s `From` and `Into`
//! pattern. The `From` trait allows for a type to define how to create itself
//! from another type, hence providing us an intuitive and simple mechanism for
//! converting `events` into new `states`.
//!
//! The `From` and `Into` traits are inherently linked and reciprocal.
//! Implementing one of them is enough. We'll be implementing the `From` trait
//! to convert from `event` types to `state` types following the semantics of
//! the state machine transitions. Only transitions for which the `From` trait
//! is implemented are authorized and any other transition would be detected at
//! compile-time as an error.

mod boot_protocol;
mod boot_server;
mod settings;
mod utils;

pub use boot_server::{singleton, DeviceManager};
pub use settings::{Settings, SettingsBuilder};
