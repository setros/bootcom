//! `bootcom` serial boot protocol state machine.
//!
//! The boot session using `bootcom` has two modes: terminal mode and
//! kernel-send mode. During terminal mode, `bootcom` operates similarly to a
//! simple terminal, printing whatever data it reads on the serial port
//! (stripping out special commands) and eventually taking commands from the
//! booting device and the user.
//!
//! The following state diagram summarizes the different states and transitions
//! `bootcom` device management goes through:
//!
//! ```text
//! TODO: add the state diagram
//! ```

use super::events::*;
use super::states::*;
use crate::settings::Settings;

// =============================================================================
// Public Interface
// =============================================================================

/// Represents the `bootcom` serial boot protocol state machine. Use the
/// `factory()` function to get an instance then run it by calling its `run()`
/// method.
pub struct SerialBootProtocol {
    sm: ProtocolStates,
}
impl SerialBootProtocol {
    /// The boot protocol state machine event loop runs until the `Done` state
    /// is reached and its `should_exit` flag is set. At such point, the event
    /// loop terminates and returns an exit code indicating no errors when equal
    /// to **`0`**; otherwise a termination with error.
    pub fn run(&mut self) -> i8 {
        loop {
            self.sm = self.sm.step();
            match &self.sm {
                ProtocolStates::Done(sm) => {
                    if sm.state.should_exit {
                        return if sm.state.with_error { 1 } else { 0 };
                    }
                }
                _ => {}
            }
        }
    }
}

/// Factory function for the `bootcom` serial boot protocol state machine. Use
/// it to get an instance of the state machine, which you can run by invoking
/// its `run()` method.
pub fn factory(settings: Settings) -> SerialBootProtocol {
    SerialBootProtocol {
        // The same machine naturally starts in the `Init` state.
        sm: ProtocolStates::Init(ProtocolSM::new(settings)),
    }
}

// =============================================================================
// Private stuff
// =============================================================================

/// The raw state machine implementing `bootcom`'s serial boot protocol.
///
/// This is a private interface, abstracted for a simpler and more intuitive use
/// in the public `SerialBootProtocol` interface.
///
/// Note that using a generic type that holds the current state serves two
/// purposes. It allows for also having shared data by all states that is not
/// really part of state data (e.g. state machine parameters, statistics,
/// etc...). Additionally, it's nicer when debugging to see the state machine
/// and the current state it is holding at any time.
#[derive(Debug)]
struct ProtocolSM<S: Runnable> {
    settings: Settings,
    state: S,
}
impl<S: Runnable> ProtocolSM<S> {
    fn run(&mut self) -> Event {
        self.state.run(&self.settings)
    }
}

/// The state machine starts in the `InitState`.
impl ProtocolSM<InitState> {
    fn new(settings: Settings) -> Self {
        ProtocolSM {
            settings,
            state: InitState {},
        }
    }
}

/// An enum wrapper around the states of the boot protocol state machine. It
/// provides a simpler and more intuitive model for manipulating states and
/// their transitions.
enum ProtocolStates {
    Init(ProtocolSM<InitState>),
    TerminalMode(ProtocolSM<TerminalModeState>),
    KernelSendMode(ProtocolSM<KernelSendModeState>),
    Done(ProtocolSM<DoneState>),
}
impl ProtocolStates {
    /// The unit of work in the state machine event loop. It checks the current
    /// state and the current event and decides the next transition. State
    /// transitions from events are implemented using the rust `From`/`Into`
    /// pattern. Most of the potential errors of state/event/transition
    /// mismatches can be caught at compile time.
    fn step(&mut self) -> Self {
        match self {
            ProtocolStates::Init(sm) => {
                let event = sm.run();
                match event {
                    Event::SwitchToTerminalMode(ev) => ProtocolStates::TerminalMode(ev.into()),
                    Event::Done(ev) => ProtocolStates::Done(ev.into()),
                    _ => unreachable!("illegal event {:#?} at current state {:#?}", event, sm),
                }
            }
            ProtocolStates::TerminalMode(sm) => {
                let event = sm.run();
                match event {
                    Event::SwitchToKernelSendMode(ev) => ProtocolStates::KernelSendMode(ev.into()),
                    Event::Done(ev) => ProtocolStates::Done(ev.into()),
                    _ => unreachable!("illegal event {:#?} at current state {:#?}", event, sm),
                }
            }
            ProtocolStates::Done(sm) => {
                let event = sm.run();
                match event {
                    Event::Exit(ev) => ProtocolStates::Done(ev.into()),
                    _ => unreachable!("illegal event {:#?} at current state {:#?}", event, sm),
                }
            }
            ProtocolStates::KernelSendMode(sm) => {
                let event = sm.run();
                match event {
                    Event::SwitchToTerminalMode(ev) => ProtocolStates::TerminalMode(ev.into()),
                    Event::Done(ev) => ProtocolStates::Done(ev.into()),
                    _ => unreachable!("illegal event {:#?} at current state {:#?}", event, sm),
                }
            }
        }
    }
}

// -----------------------------------------------------------------------------
// State from Event transitions
// -----------------------------------------------------------------------------

impl From<SwitchToTerminalModeEvent> for ProtocolSM<TerminalModeState> {
    fn from(event: SwitchToTerminalModeEvent) -> ProtocolSM<TerminalModeState> {
        // ... Logic prior to transition
        ProtocolSM {
            // ... attr: val.attr
            settings: event.settings,
            state: TerminalModeState {
                port: Some(event.port),
            },
        }
    }
}

impl From<SwitchToKernelSendModeEvent> for ProtocolSM<KernelSendModeState> {
    fn from(event: SwitchToKernelSendModeEvent) -> ProtocolSM<KernelSendModeState> {
        // ... Logic prior to transition
        ProtocolSM {
            // ... attr: val.attr
            settings: event.settings,
            state: KernelSendModeState {
                port: Some(event.port),
            },
        }
    }
}

impl From<DoneEvent> for ProtocolSM<DoneState> {
    fn from(event: DoneEvent) -> ProtocolSM<DoneState> {
        // ... Logic prior to transition
        ProtocolSM {
            // ... attr: val.attr
            settings: event.settings,
            state: DoneState {
                with_error: event.with_errors,
                should_exit: false,
            },
        }
    }
}
impl From<ExitEvent> for ProtocolSM<DoneState> {
    fn from(event: ExitEvent) -> ProtocolSM<DoneState> {
        // ... Logic prior to transition
        ProtocolSM {
            // ... attr: val.attr
            settings: event.settings,
            state: DoneState {
                with_error: event.with_error,
                should_exit: true,
            },
        }
    }
}
