//! States for the `bootcom` serial boot protocol state machine.
//!
//! This modules is private and restricted to the
//! [`boot_protocol`](crate::boot_protocol) scope. The public interface of the
//! serial boot protocol state machine is provided by
//! [`boot_protocol`](crate::boot_protocol).
//!
//! ```ignore
//! use super::states::*;
//! ```
//!
//! Refer to the [`state_machine`](super::state_machine) module for an overview
//! of states, events and transitions.

use std::{fmt, thread, time::Duration};

use console::style;
use log::{info, log_enabled, trace, Level::Debug};
use serialport::SerialPort;

use super::events::*;

use crate::utils::open_and_setup_port;
use crate::{settings::Settings, utils::send_kernel};

// =============================================================================
// Crate-Public Interface
// =============================================================================

/// Trait adding the ability for a state to be `run` after a transition into it.
pub(crate) trait Runnable {
    /// A state implements this method so it can be `run` after the state
    /// machine transitions into it.
    ///
    /// During this call, the state can do any work that needs to be done and
    /// when finished, requests a transition to a `new state` by returning the
    /// appropriate `event`. The `state` and the `event` are consumed to create
    /// the `new state` using the corresponding [`From`] trait implementation
    /// (provided such implementation exists).
    fn run(&mut self, settings: &Settings) -> Event;
}

// Init State ==================================================================

/// The initial state of the boot protocol state machine.
///
/// From the `InitState`, the state machine can evolve via the following
/// transitions:
///
///  * **[`SwitchToTerminalModeEvent`] => [`TerminalModeState`]** which happens
///    after the serial port is initialized and connected,
///  * **[`DoneEvent`] => [`DoneState`]** when the serial boot session is
///    finished due to the user action or to any other interruption caused by
///    unrecoverable errors, disconnection, etc.
#[derive(Debug)]
pub(crate) struct InitState {}
impl Runnable for InitState {
    fn run(&mut self, settings: &Settings) -> Event {
        info!("=> Init");
        assert_ne!(settings.path, None);

        match open_and_setup_port(&settings) {
            Ok(port) => Event::SwitchToTerminalMode(SwitchToTerminalModeEvent {
                settings: settings.clone(),
                port,
            }),
            Err(_) => {
                // This is fatal for the protocol state machine, but not for
                // `bootcom`. Terminate with error so that `bootcom` device
                // manager can go back into waiting for the device to be ready
                // or select a new one.
                Event::Done(DoneEvent {
                    settings: settings.clone(),
                    with_errors: true,
                })
            }
        }
    }
}

// TerminalMode State ==========================================================

/// A `state` of the boot protocol state machine where `bootcom` reads data from
/// the booting device, displays it on the terminal and can take commands from
/// the user or the booting device.
///
/// The currenlty implemented commands are:
/// * **`send_kernel`**: initiated by the boot device consecutively sending
///   **`0x03`** **three(3)** times.
///
/// The booting device is not allowed to send a command before a response to the
/// previous one was received.
///
/// This state can tranisition to another state as following:
///
///  * **[`SwitchToKernelSendModeEvent`] => [`KernelSendModeState`]** upon
///    reception of the `send_kernel` command from the booting device,
///  * **[`DoneEvent`] => [`DoneState`]** when the serial boot session is
///    finished due to the user action or to any other interruption caused by
///    errors, disconnection, etc.
pub(crate) struct TerminalModeState {
    /// The serial port to be used, already configured and open.
    ///
    /// Consumed and moved upon the transition to [`KernelSendModeState`].
    pub port: Option<Box<dyn SerialPort>>,
}
impl Runnable for TerminalModeState {
    fn run(&mut self, settings: &Settings) -> Event {
        use hexplay::HexViewBuilder;
        use std::io::{self, Write};

        info!("=> Terminal Mode");
        let mut got_errors = false;
        let mut send_kernel = false;

        if let Some(mut port) = self.port.take() {
            loop {
                // To handle the unreliable behavior of blocking/non-blocking of
                // reads over the serial port, we'll first check the available
                // data in the port's input buffer, and we only read the exact
                // number of available bytes (up to a certain maximum amount).
                // That way we can always know that read will return
                // immediately.
                match port.bytes_to_read() {
                    Ok(available) => {
                        trace!("Bytes available to read: {}", available);
                        if available > 0 {
                            // We'll read 4K maximum each time
                            let mut serial_buf: Vec<u8> =
                                vec![0; std::cmp::min(available, 4096) as usize];
                            match port.read(serial_buf.as_mut_slice()) {
                                Ok(mut t) => {
                                    // The data may contain a command at the end
                                    // and only at the end.
                                    let command: Vec<u8> = serial_buf[..t]
                                        .iter()
                                        .rev()
                                        .take_while(|b| **b == 3)
                                        .cloned()
                                        .collect();

                                    if command == [3, 3, 3] {
                                        // We got a `send_kernel` command
                                        t -= 3;
                                        send_kernel = true;
                                    }

                                    io::stdout().write_all(&serial_buf[..t]).unwrap();
                                    println!();

                                    // Dump the received data in a hex table for
                                    // debugging
                                    if log_enabled!(Debug) {
                                        let view = HexViewBuilder::new(&serial_buf[..t])
                                            .address_offset(0)
                                            .row_width(16)
                                            .finish();
                                        println!("{}", view);
                                    }

                                    if send_kernel {
                                        break;
                                    };
                                }
                                Err(ref e) => {
                                    info!("error: {:?}", e.to_string());
                                    got_errors = true;
                                    break;
                                }
                            }
                        }

                        thread::sleep(Duration::from_millis(100));
                    }
                    Err(ref e) => {
                        info!("error: {:?}", e.to_string());
                        got_errors = true;
                        break;
                    }
                }
            }
            // Check commands
            if send_kernel {
                return Event::SwitchToKernelSendMode(SwitchToKernelSendModeEvent {
                    settings: settings.clone(),
                    port,
                });
            }

            return Event::Done(DoneEvent {
                settings: settings.clone(),
                with_errors: got_errors,
            });
        }

        // We should never reach here!
        unreachable!()
    }
}
impl fmt::Debug for TerminalModeState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.port {
            Some(port) => debug_fmt_serialport!(port, f).finish(),
            None => f.debug_tuple("TerminalModeState").finish(),
        }
    }
}

// KernelSendMode State ========================================================

/// A `state` of the boot protocol state machine where `bootcom` reads the
/// content of the kernel image and send it to the boot device.
///
/// The kernel image size is limited to a maximum of 0xFFFFFFFF (i.e. can fit in
/// a 32 bit unsigned integer). The size is sent first, in **[`little
/// endian`](https://en.wikipedia.org/wiki/Endianness)** format, then `bootcom`
/// expects a response from the boot device with the bytes `'O'` `'K'`, before
/// finally pushing the entire content of the kernel image.
///
///  * **[`SwitchToTerminalModeEvent`] => [`TerminalModeState`]** upon
///    completion of the kernel image push,
///  * **[`DoneEvent`] => [`DoneState`]** when the serial boot session is
///    interrupted due to unrecoverable errors, disconnection, etc.
pub(crate) struct KernelSendModeState {
    /// The serial port to be used, already configured and open.
    ///
    /// Consumed and moved upon the transition to [`TerminalModeState`].
    pub port: Option<Box<dyn SerialPort>>,
}
impl Runnable for KernelSendModeState {
    fn run(&mut self, settings: &Settings) -> Event {
        info!("=> Kernel Send Mode");

        if let Some(mut port) = self.port.take() {
            // Try to send the kernel data. If the operation fails, we'll go
            // back to terminal mode just waiting for the bootloader to notice
            // the failure and eventually restart the request to send the
            // kernel.
            // TODO: Implement this error recovery in the bootloader

            loop {
                match send_kernel(&mut port, settings) {
                    Ok(_) => {
                        break;
                    }
                    Err(ref e) => {
                        info!("error: {:?}", e.to_string());
                        println!("{}", style("[BC] 💥 Failed to send kernel image!").red());
                    }
                }
            }

            // Go back to terminal mode.
            return Event::SwitchToTerminalMode(SwitchToTerminalModeEvent {
                settings: settings.clone(),
                port,
            });
        }

        // We should never reach here!
        unreachable!()
    }
}
impl fmt::Debug for KernelSendModeState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.port {
            Some(port) => debug_fmt_serialport!(port, f).finish(),
            None => f.debug_tuple("TerminalModeState").finish(),
        }
    }
}

// Done State ==================================================================

/// Reached when the boot protocol state machine completes its execution and is
/// about to terminate (normally or abnormally).
///
/// This state goes into a 2-phase execution. During the initial phase, it runs
/// like any other state to do its own things like printing some information,
/// cleaning up etc. It then triggers the [`ExitEvent`] to cause the boot
/// protocol state machine to terminate and exit.
///
/// Termination due to errors is indicated with the `with_error` field in the
/// state. This condition can be used to set the return value from the boot
/// protocol state machine event loop.
#[derive(Debug, Copy, Clone)]
pub(crate) struct DoneState {
    /// When `true`, indicates an abnormal completion caused by an error.
    pub with_error: bool,
    /// When `true` instructs the boot protocol state machine to exit its event
    /// loop.
    pub should_exit: bool,
}
impl Runnable for DoneState {
    fn run(&mut self, settings: &Settings) -> Event {
        info!(
            "=> Done with{}errors",
            if self.with_error { " " } else { " no " }
        );
        // Report errors
        if self.with_error {
            println!(
                "{}",
                style("[BC] 💥 Unrecoverable error on the serial port!").red()
            );
            println!("[BC] 🔌 Disconnect and reconnect the device!");
        }

        Event::Exit(ExitEvent {
            settings: settings.clone(),
            with_error: self.with_error,
        })
    }
}
