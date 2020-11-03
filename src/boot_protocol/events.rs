//! States for the `bootcom` serial boot protocol state machine.
//!
//! This modules is private and restricted to the
//! [`boot_protocol`](crate::boot_protocol) scope. The public interface of the
//! serial boot protocol state machine is provided by
//! [`boot_protocol`](crate::boot_protocol).
//!
//! ```ignore
//! use super::events::*;
//! ```
//!
//! Refer to the [`state_machine`](super::state_machine) module for an overview
//! of states, events and transitions.

use std::fmt;

use serialport::SerialPort;

use crate::Settings;

// =============================================================================
// Crate-Public Interface
// =============================================================================

// SwitchToTerminalModeEvent ===================================================

/// Event fired to trigger a transition to [`TerminalModeState`].
///
/// This event can happen under one of the following circumstances:
///
///  1. While at the [`InitState`] and after a serial port has been successfully
///     opened and configured.
///  2. While at the [`KernelModeState`] after the kernel image has been
///     successfully pushed.
pub struct SwitchToTerminalModeEvent {
    pub settings: Settings,
    /// The serial port to be used in the next state. Consumed and moved to the
    /// next state.
    pub port: Box<dyn SerialPort>,
}
impl fmt::Debug for SwitchToTerminalModeEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let port = &self.port;
        debug_fmt_serialport!(port, f).finish()
    }
}

// SwitchToKernelModeEvent =====================================================

/// Event fired to trigger a transition to [`KernelSendModeState`].
///
/// This event can happen under one of the following circumstances:
///
///  1. While at the [`TerminalModeState`] upon reception of the `send_kernel`
///     command from the booting device.
pub struct SwitchToKernelSendModeEvent {
    pub settings: Settings,
    /// The serial port to be used in the next state. Consumed and moved to the
    /// next state.
    pub port: Box<dyn SerialPort>,
}
impl fmt::Debug for SwitchToKernelSendModeEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let port = &self.port;
        debug_fmt_serialport!(port, f).finish()
    }
}

// DoneState ===================================================================

/// Event fired when the boot protocol execution completes and is about to
/// terminate. It triggers a transition to the `Done` state.
///
/// This event can heppen at any state due to normal termination, user initiated
/// termination or abnormal termination caused by an unrecoverable error.
#[derive(Debug)]
pub(crate) struct DoneEvent {
    pub settings: Settings,
    /// When `true`, indicates an abnormal completion caused by an error.
    pub with_errors: bool,
}

// ExitEvent ===================================================================

/// The last event that can be triggered in the boot protocol state machine and
/// will result in the event loop terminating with an `exit status`, handing
/// back the control to the original caller that started the state machine event
/// loop.
///
/// The returned `status code` can be interpreted as whether the completion was
/// normal or abnormal.
///
/// **Example**
/// ```ignore
/// use crate::settings::*;
/// use crate::boot_protocol as bpsm;
///
/// let settings = SettingsBuilder::new().finalize();
/// let mut sm = bpsm::factory(settings);
/// let status = sm.run(); // status code returned after the `Exit` event
/// println!("status: {}", status);
/// ```
#[derive(Debug)]
pub(crate) struct ExitEvent {
    pub settings: Settings,
    pub with_error: bool,
}

// Events enum ==================================================================

/// Events that can be triggered within the serial boot protocol state machine
/// of `bootcom`.
///
/// Each possible value holds an `event`, which in turn may hold additional data
/// for the state transition. Such data is passed by the origin state for
/// potential use by the target state.
#[derive(Debug)]
pub(crate) enum Event {
    SwitchToTerminalMode(SwitchToTerminalModeEvent),
    SwitchToKernelSendMode(SwitchToKernelSendModeEvent),
    Done(DoneEvent),
    Exit(ExitEvent),
}
