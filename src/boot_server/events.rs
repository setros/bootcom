//! Events for the `bootcom` boot server state machine.
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

use crate::settings::Settings;

// =============================================================================
// Crate-Public Interface
// =============================================================================

// WaitForPortEvent ============================================================

/// Event fired to trigger a transition to the `WaitForPort` state.
///
/// This event can happen under one of the following circumstances:
///
///  1. While at the `Init` state and a port name was provided. In such case,
///     port selection is skipped and we just want to hold-on until the port is
///     created (meaning the device is plugged).
///  2. When an unrecoverable port error occurs while at the `Service` state.
///     This usually results from the device being removed and would require a
///     new port to be opened.
#[derive(Debug)]
pub(crate) struct WaitForPortEvent {
    pub settings: Settings,
}

// SelectPortEvent =============================================================

/// Event fired to trigger the transition to the `SelectPort` state.
///
/// This event can happen under one of the following circumstances:
///
///  1. If the program is started with no specific device path provided. In such
///     case, `bootcom` will immediately transition into the port delection
///     state from the initial state.
///  2. If the program was started with a specific device path provided, but the
///     device is not ready and `bootcom` is waiting for it, and the user
///     cancels the wait by pressing the `ESC` key. In such case, `bootcom`
///     transitions into the port selection state for the user to select a
///     device out of the available ones.
///  3. If the program is in the port selection state and the user decides to
///     not select any device (by hitting the `ESC` key) to refresh the list and
///     be presented with an update list of connected devices.
#[derive(Debug)]
pub(crate) struct SelectPortEvent {
    pub settings: Settings,
}

// PortReadyEvent ==============================================================

/// Event fired when we have a serial port with a valid device path on the
/// system. This would be the result of either the port we were waiting on has
/// come up or a port was selected from the list of detected ports.
///
/// This event can be fired from the `WaitForPort` or `SelectPort` states states
/// and triggers a transition to the `Service` state.
#[derive(Debug)]
pub(crate) struct PortReadyEvent {
    pub settings: Settings,
}

// PortErrorEvent ==============================================================

/// Event fired when an error related to the serial port (usually a
/// communication error resulting from the device being removed) occurs.
///
/// This event can be fired only from the `Service` state and triggers a
/// transition into the `EaitForPort` state.
#[derive(Debug)]
pub(crate) struct PortErrorEvent {
    pub settings: Settings,
}

// DoneEvent ===================================================================

/// Event fired when the program completes and is about to terminate. It
/// triggers a transition to the `Done` state.
///
/// TODO: Exhaustive list of places where this could happen
#[derive(Debug)]
pub(crate) struct DoneEvent {
    pub settings: Settings,
    pub with_errors: bool,
}

// ExitEvent ===================================================================

/// The last event that can be triggered in `bootcom` and will result in the
/// event loop terminating with an `exit status`, handing back the control to
/// the original caller that started the event loop.
///
/// The returned `status code` can be used as an exit code from the `main`
/// function.
///
/// **Example**
/// ```no_run
/// use bootcom::{self as bc, DeviceManager};
///
/// let settings = bc::SettingsBuilder::new().finalize();
/// let mut sdm = bc::singleton(settings);
/// let status = sdm.run(); // status code returned after the `Exit` event
/// println!("status: {}", status);
/// std::process::exit(0);
/// ```
#[derive(Debug)]
pub(crate) struct ExitEvent {
    pub settings: Settings,
    pub with_error: bool,
}

// Events enum ==================================================================

/// Events that can be triggered within the device management state machine of
/// `bootcom`.
///
/// Each possible value holds an `event`, which in turn may hold additional data
/// for the state transition. Such data is passed by the origin state for
/// potential use by the target state.
#[derive(Debug)]
pub(crate) enum Event {
    WaitForPort(WaitForPortEvent),
    SelectPort(SelectPortEvent),
    PortReady(PortReadyEvent),
    PortError(PortErrorEvent),
    Done(DoneEvent),
    Exit(ExitEvent),
}
