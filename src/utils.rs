//! Helper functions to deal with serial ports.

mod kernel;
mod keyboard;
mod ports;

pub(crate) use kernel::send_kernel;
pub(crate) use keyboard::*;
pub(crate) use ports::{open_and_setup_port, select_port, wait_for_port};
