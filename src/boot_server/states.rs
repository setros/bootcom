//! States for the `bootcom` boot server state machine.
//!
//! This modules is private and restricted to the
//! [`boot_server`](crate::boot_server) scope. The public interface of the state
//! machine is provided by [`boot_server`](crate::boot_server).
//!
//! ```ignore
//! use super::events::*;
//! ```
//!
//! Refer to the [`state_machine`](super::state_machine) module for an overview
//! of states, events and transitions.

use log::info;

use crate::utils;
use crate::{
    boot_protocol::{self as bpsm},
    settings::Settings,
};

use super::events::*;

// =============================================================================
// Crate-Public Interface
// =============================================================================

/// Trait adding the ability for a state to be `run` after a transition into it.
pub(crate) trait Runnable {
    /// A state implements this method so it can be `run` after the state
    /// machine transitions into it.
    ///
    /// During this call, the state can do any work that needs to be done and
    /// when finished, requests transition to a new state by returning the
    /// appropriate `event`. The `event` is then consumed to create the new
    /// `state` using the corresponding `From` trait implementation if avaiable.
    fn run(&mut self, settings: &Settings) -> Event;
}

// Init State ==================================================================

/// Represents the initial state of the device manager state machine.
///
/// From the `InitState`, the state machine can evolve via the following
/// transitions:
///
///  * **`WaitForPortEvent` => `WaitForPortState`** when a specific device path
///    was provided in the settings,
///  * **`SelectPortEvent` => `SelectPortState`** when no device path was
///    provided in the settings.
#[derive(Debug)]
pub(crate) struct InitState {}
impl Runnable for InitState {
    /// At the `Init` state, check if the provided `settings` have a device
    /// path, and if yes, transition to the `WaitForPort` state; otherwise
    /// transition to the `SelectPort` state.
    fn run(&mut self, settings: &Settings) -> Event {
        info!("=> Init");
        match settings.path {
            Some(_) => Event::WaitForPort(WaitForPortEvent {
                settings: settings.clone(),
            }),
            None => Event::SelectPort(SelectPortEvent {
                settings: settings.clone(),
            }),
        }
    }
}

// WaitForPortState ============================================================

#[derive(Debug)]
pub(crate) struct WaitForPortState {}
impl Runnable for WaitForPortState {
    fn run(&mut self, settings: &Settings) -> Event {
        let path = settings.path.as_ref().unwrap();
        info!("=> WaitForPort");
        let canceled = utils::wait_for_port(path);
        if canceled {
            Event::SelectPort(SelectPortEvent {
                settings: settings.clone(),
            })
        } else {
            // The wait for port to be ready completed without cancellation. Fire
            // the `PortReady` event to trigger the transition to the next state.
            Event::PortReady(PortReadyEvent {
                settings: settings.clone(),
            })
        }
    }
}

// SelectPortState =============================================================

#[derive(Debug)]
pub(crate) struct SelectPortState {}
impl Runnable for SelectPortState {
    fn run(&mut self, settings: &Settings) -> Event {
        info!("=> SelectPort");
        let selection = crate::utils::select_port();
        match selection {
            // We have a serial port device path that we now need to update in
            // the settings and then trigger the transition via the `PortReady`
            // event.
            Some(path) => {
                let mut cloned_settings = settings.clone();
                cloned_settings.path = Some(path);
                Event::PortReady(PortReadyEvent {
                    settings: cloned_settings,
                })
            }
            None => Event::SelectPort(SelectPortEvent {
                settings: settings.clone(),
            }),
        }
    }
}

// ServiceState ================================================================

#[derive(Debug)]
pub(crate) struct ServiceState {}
impl Runnable for ServiceState {
    fn run(&mut self, settings: &Settings) -> Event {
        info!("=> Service");

        let mut bpsm = bpsm::factory(settings.clone());
        match bpsm.run() {
            // Normal termination -> we're done.
            0 => Event::Done(DoneEvent {
                settings: settings.clone(),
                with_errors: false,
            }),
            // A port error inside the boot protocol state machine -> wait for
            // the device to be ready again
            _ => Event::PortError(PortErrorEvent {
                settings: settings.clone(),
            }),
        }
    }
}

// Done State ==================================================================

// State B goes and breaks up that String into words.
#[derive(Debug, Copy, Clone)]
pub(crate) struct DoneState {
    pub with_error: bool,
    pub should_exit: bool,
}
impl Runnable for DoneState {
    fn run(&mut self, settings: &Settings) -> Event {
        info!(
            "=> Done with{}errors",
            if self.with_error { " " } else { " no " }
        );
        Event::Exit(ExitEvent {
            settings: settings.clone(),
            with_error: self.with_error,
        })
    }
}
