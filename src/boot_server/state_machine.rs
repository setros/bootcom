//! Serial port device selection and and state management.
//!
//! Bootcom operates over a serial port which can be specified at the command
//! line or can be selected out of the list of available ports on the system.
//! Due to the transient nature of the serial connection when devices are
//! plugged in or out, we need some flexibility in handling cases where the port
//! is not ready or when the device is removed and inserted again. Additionally
//! as multiple USB serial controllers can be used (e.g. UART and JTAG) and can
//! be removed and inserted at different orders, the port names may change and
//! we need flexibility to re-select the ports for `bootcom`.
//!
//! The following state diagram summarizes the different states and transitions
//! `bootcom` device management goes through:
//!
//! ```text
//!                            START
//!                              |
//!                              v
//!                          .-------.
//!                          | Init  |
//!                          '-------'
//!                              |
//!                              v
//!                    no  .----------.  yes
//!                  .----( port_name? )----.
//!      .-----.     |     '----------'     |
//!      |     |     v                      v
//!      |    .------------.         .-------------.
//!      '--->| SelectPort |<-----.--| WaitForPort |<---.
//!           '------------'      |  '-------------'    |
//!              |              port                    |
//!              |              ready                   |
//!              |                v                     |
//!             port     ******************             |
//!             ready    *    Service     *     port    |
//!              |       ******************     error   |
//!              '------>* Protocol State *-------------'
//!                      *    Machine     *
//!                      ******************
//!                               |
//!                               v
//!                              END
//! ```

use std::sync::{Arc, Mutex, Once};

use super::events::*;
use super::states::*;
use crate::settings::Settings;

// =============================================================================
// Public Interface
// =============================================================================

// -----------------------------------------------------------------------------
// Device Manager Singleton
// -----------------------------------------------------------------------------

pub trait DeviceManager {
    fn run(&mut self) -> i8;
}

/// Encapsulate the state machine creation and event loop to provide a concise
/// and simple public interface to the module users.
///
/// Only one instance of this struct exists, using the `singleton` pattern, and
/// which can accessed by calling the `singleton()` function.
#[derive(Clone)]
pub struct SingletonReader {
    // Since this can be used in many threads, we need to protect concurrent
    // access
    inner: Arc<Mutex<DeviceManagerStates>>,
}
impl DeviceManager for SingletonReader {
    /// The device manager event loop runs until the `Done` state is reached and
    /// its `should_exit` flag is set. At such point, the event loop terminates
    /// and returns an exit code indicating no errors when equal to **`0`**;
    /// otherwise a termination with error.
    ///
    /// The returned status code could be used as an exit code from `bootcom`.
    fn run(&mut self) -> i8 {
        loop {
            let mut data = self.inner.lock().unwrap();
            *data = data.step();
            if let DeviceManagerStates::Done(sm) = &*data {
                if sm.state.should_exit {
                    return if sm.state.with_error { 1 } else { 0 };
                }
            }
        }
    }
}

/// Returns the single instance of the device manager.
///
/// In order to use the singleton instance, proper locking needs to be observed.
/// The example below demonstrates an example usage scenario:
///
/// ```ignore
///     let settings = SettingsBuilder::new().finalize();
///     let mut s = singleton(settings);
///     s.run();
/// ```
pub fn singleton(settings: Settings) -> SingletonReader {
    // Initialize it to a null value
    static mut DM_SINGLETON: *const SingletonReader = 0 as *const SingletonReader;
    static DM_ONCE: Once = Once::new();

    unsafe {
        DM_ONCE.call_once(|| {
            // Make it
            let singleton = SingletonReader {
                inner: Arc::new(Mutex::new(DeviceManagerStates::Init(
                    DeviceManagerStateMachine::new(settings),
                ))),
            };

            // Put it in the heap so it can outlive this call
            DM_SINGLETON = std::mem::transmute(Box::new(singleton));
        });

        // Now we give out a copy of the data that is safe to use concurrently.
        (*DM_SINGLETON).clone()
    }
}

// =============================================================================
// Private stuff
// =============================================================================

// -----------------------------------------------------------------------------
// The State Machine
// -----------------------------------------------------------------------------

/// The state machine implementing `bootcom`'s management of serial port devices
/// lifecycle.
///
/// Note that using a generic type that holds the current state serves two
/// purposes. It allows for also having shared data by all states that is not
/// really part of state data (e.g. state machine parameters, statistics,
/// etc...). Additionally, it's nicer when debugging to see the state machine
/// and the current state it is holding at any time.
#[derive(Debug)]
struct DeviceManagerStateMachine<S: Runnable> {
    settings: Settings,
    state: S,
}
impl<S: Runnable> DeviceManagerStateMachine<S> {
    fn run(&mut self) -> Event {
        self.state.run(&self.settings)
    }
}

/// The device management state machine starts in the `InitState`.
impl DeviceManagerStateMachine<InitState> {
    fn new(settings: Settings) -> Self {
        DeviceManagerStateMachine {
            settings,
            state: InitState {},
        }
    }
}

/// Wraps the state machine and its various states into a simple enum, which can
/// also be used for pattern matching during state transitions.
enum DeviceManagerStates {
    Init(DeviceManagerStateMachine<InitState>),
    WaitForPort(DeviceManagerStateMachine<WaitForPortState>),
    SelectPort(DeviceManagerStateMachine<SelectPortState>),
    Service(DeviceManagerStateMachine<ServiceState>),
    Done(DeviceManagerStateMachine<DoneState>),
}
impl DeviceManagerStates {
    fn step(&mut self) -> Self {
        match self {
            DeviceManagerStates::Init(sm) => {
                let event = sm.run();
                match event {
                    Event::WaitForPort(ev) => DeviceManagerStates::WaitForPort(ev.into()),
                    Event::SelectPort(ev) => DeviceManagerStates::SelectPort(ev.into()),
                    _ => unreachable!("illegal event {:#?} at current state {:#?}", event, sm),
                }
            }
            DeviceManagerStates::WaitForPort(sm) => {
                let event = sm.run();
                match event {
                    Event::PortReady(ev) => DeviceManagerStates::Service(ev.into()),
                    Event::SelectPort(ev) => DeviceManagerStates::SelectPort(ev.into()),
                    _ => unreachable!("illegal event {:#?} at current state {:#?}", event, sm),
                }
            }
            DeviceManagerStates::SelectPort(sm) => {
                let event = sm.run();
                match event {
                    Event::SelectPort(ev) => DeviceManagerStates::SelectPort(ev.into()),
                    Event::PortReady(ev) => DeviceManagerStates::Service(ev.into()),
                    _ => unreachable!("illegal event {:#?} at current state {:#?}", event, sm),
                }
            }
            DeviceManagerStates::Service(sm) => {
                let event = sm.run();
                match event {
                    Event::Done(ev) => DeviceManagerStates::Done(ev.into()),
                    Event::PortError(ev) => DeviceManagerStates::WaitForPort(ev.into()),
                    _ => unreachable!("illegal event {:#?} at current state {:#?}", event, sm),
                }
            }
            DeviceManagerStates::Done(sm) => {
                let event = sm.run();
                match event {
                    Event::Exit(ev) => DeviceManagerStates::Done(ev.into()),
                    _ => unreachable!("illegal event {:#?} at current state {:#?}", event, sm),
                }
            }
        }
    }
}

// -----------------------------------------------------------------------------
// State from Event transitions
// -----------------------------------------------------------------------------

impl From<WaitForPortEvent> for DeviceManagerStateMachine<WaitForPortState> {
    fn from(event: WaitForPortEvent) -> DeviceManagerStateMachine<WaitForPortState> {
        // ... Logic prior to transition
        DeviceManagerStateMachine {
            // ... attr: val.attr
            settings: event.settings,
            state: WaitForPortState {},
        }
    }
}
impl From<PortErrorEvent> for DeviceManagerStateMachine<WaitForPortState> {
    fn from(event: PortErrorEvent) -> DeviceManagerStateMachine<WaitForPortState> {
        // ... Logic prior to transition
        DeviceManagerStateMachine {
            // ... attr: val.attr
            settings: event.settings,
            state: WaitForPortState {},
        }
    }
}

impl From<SelectPortEvent> for DeviceManagerStateMachine<SelectPortState> {
    fn from(event: SelectPortEvent) -> DeviceManagerStateMachine<SelectPortState> {
        // ... Logic prior to transition
        DeviceManagerStateMachine {
            // ... attr: val.attr
            settings: event.settings,
            state: SelectPortState {},
        }
    }
}

impl From<PortReadyEvent> for DeviceManagerStateMachine<ServiceState> {
    fn from(event: PortReadyEvent) -> DeviceManagerStateMachine<ServiceState> {
        // ... Logic prior to transition
        DeviceManagerStateMachine {
            // ... attr: val.attr
            settings: event.settings,
            state: ServiceState {},
        }
    }
}

impl From<DoneEvent> for DeviceManagerStateMachine<DoneState> {
    fn from(event: DoneEvent) -> DeviceManagerStateMachine<DoneState> {
        // ... Logic prior to transition
        DeviceManagerStateMachine {
            // ... attr: val.attr
            settings: event.settings,
            state: DoneState {
                with_error: event.with_errors,
                should_exit: false,
            },
        }
    }
}
impl From<ExitEvent> for DeviceManagerStateMachine<DoneState> {
    fn from(event: ExitEvent) -> DeviceManagerStateMachine<DoneState> {
        // ... Logic prior to transition
        DeviceManagerStateMachine {
            // ... attr: val.attr
            settings: event.settings,
            state: DoneState {
                with_error: event.with_error,
                should_exit: true,
            },
        }
    }
}
