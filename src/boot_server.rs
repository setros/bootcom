//! `bootcom` boot (over serial port) server.
//!
//! **Example** - Executing the state machine the event loop:
//! ```no_run
//! use bootcom::{self as bc, DeviceManager};
//!
//! let settings = bc::SettingsBuilder::default().finalize();
//! let mut sdm = bc::singleton(settings);
//! let status = sdm.run(); // status code returned after the `Exit` event
//! println!("status: {}", status);
//! std::process::exit(0);
//! ```

mod events;
mod state_machine;
mod states;

pub use state_machine::{singleton, DeviceManager};
