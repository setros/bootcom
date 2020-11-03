//! `bootcom` serial port protocol.
//!
//! **Example** - Importing the public interfaces through boot_protocol:
//! ```ignore
//! use crate::{
//!     boot_protocol::{self as bpsm},
//!     settings::Settings,
//! };
//! ```
//!
//! **Example** - Executing the state machine the event loop:
//! ```ignore
//! let settings = SettingsBuilder::new()
//!     .path("COM4")
//!     .baud_rate(230_400)
//!     .finalize();
//! let mut bpsm = bpsm::factory(settings);
//! bpsm.run();
//! ```

#[macro_use]
mod macros;

mod events;
mod state_machine;
mod states;

pub use state_machine::{factory, SerialBootProtocol};
