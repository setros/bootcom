//! Serial port device manipulation.

use console::{style, Term};
use indicatif::{ProgressBar, ProgressStyle};
use log::{debug, info};
use serialport::{available_ports, SerialPort, SerialPortType};

use std::{
    sync::mpsc::{self, RecvTimeoutError},
    thread,
    time::Duration,
};

use crate::{utils::poll_escape, Settings};

//==============================================================================
// Public Interface
//==============================================================================

pub(crate) fn select_port() -> Option<String> {
    // If no specific device was requested, we'll present the list of connected
    // devices to the user to interactively select one. The user may cancel the
    // selection to request for another refresh of connected devices, probably
    // waiting for a specific device to be connected.
    //
    // We'll keep doing that until a device is selected.

    let mut found_ports;
    let mut attempt: usize = 1;
    let waiting_period: usize = 1;

    let pb = ProgressBar::new_spinner();
    pb.enable_steady_tick(120);
    pb.set_style(
        ProgressStyle::default_spinner()
            // For more spinners check out the cli-spinners project:
            // https://github.com/sindresorhus/cli-spinners/blob/master/spinners.json
            .tick_strings(&["⠋", "⠙", "⠚", "⠞", "⠖", "⠦", "⠴", "⠲", "⠳", "⠓"])
            .template("[BC] {spinner:.blue} {msg}"),
    );

    // Avoid cursor flicker during the waiting
    Term::stdout().hide_cursor().unwrap();
    // Enumerate connected USB serial devices until we have some.
    loop {
        found_ports = enumerate_usb_serial_ports();
        let num_ports = found_ports.len();
        if num_ports > 0 {
            pb.finish_with_message("Select a port to be used:");
            break;
        } else {
            let waited = attempt * waiting_period;
            pb.set_message(format!(
                "[{:03}s {}] ⌛ Waiting for USB serial controller to be connected...",
                style(waited).dim(),
                num_ports
            ));
            attempt += 1;
        }

        thread::sleep(Duration::from_secs(waiting_period as u64));
    }
    Term::stdout().show_cursor().unwrap();

    // Ask the user to confirm the port selection. If a port is confirmed, it is
    // then returned as the selected port for use; otherwise, we loop again
    // refreshing the list of available ports and requesting confirmation. This
    // allows to plug the other side of the serial link and refresh the list of
    // ports without restarting `bootcom`.
    let selection = select_port_interactive(&found_ports);
    match &selection {
        Some(path) => {
            pb.finish_with_message(format!("👍 Serial port {} is ready", style(path).green()));
        }
        None => {
            pb.finish_with_message("❌ Selection canceled -> refreshing...");
        }
    }
    selection
}

/// Check for a device with the given path in the system. If not immediately
/// found, enter into a waiting loop, checking every period of time whether the
/// device has been created or not. While waiting, the user can interactively
/// cancel waiting by pressing the `ESC` key.
///
/// The function will return `true` when the wait was cancelled by the user
/// hitting `Esc`.
pub(crate) fn wait_for_port(path: &str) -> bool {
    let pb = ProgressBar::new_spinner();
    pb.enable_steady_tick(120);
    pb.set_style(
        ProgressStyle::default_spinner()
            // For more spinners check out the cli-spinners project:
            // https://github.com/sindresorhus/cli-spinners/blob/master/spinners.json
            .tick_strings(&["⠋", "⠙", "⠚", "⠞", "⠖", "⠦", "⠴", "⠲", "⠳", "⠓"])
            .template("[BC] {spinner:.blue} {msg}"),
    );

    let mut found_ports: Vec<String> = [].into();
    let mut attempt: usize = 1;
    let waiting_period = 2;

    pb.set_message(format!(
        "[{:03}s {}] ⏳ Waiting for {} to be ready (ESC to cancel)...",
        style(waiting_period).dim(),
        found_ports.len(),
        style(path).cyan()
    ));

    // We'll be using the main thread and one additional one listening on the
    // `ESC` key to cancel the waiting. Both threads needs to coordinate their
    // termination:
    //
    //  - When the device is ready on the main thread, the cancelation thread
    //    should terminate.
    //  - When the `ESC` key is pressed, the cancelation thread will naturally
    //    terminate and the main thread should stop waiting and exit the waiting
    //    loop.
    //
    // In order to achieve this coordinated termination, two channels are used:
    // one channel for the cancelation condition, and another channel for device
    // readiness condition.

    // Cancellation channel, on which the cancellation thread will be the sender
    // and the main thread the receiver.
    let (cancel_tx, cancel_rx) = mpsc::channel();

    // The device ready channel, on which the main thread will be the sender and
    // the cancellation thread the receiver.
    let (done_tx, done_rx) = mpsc::channel();

    // Start the cancellation thread to check for the `ESC` key and listen for
    // the completion from the main thread.
    let cancelation_thread = thread::spawn(move || loop {
        // Check if we need to terminate because the serial device is ready.
        if done_rx.try_recv().is_ok() {
            // Terminate
            break;
        }
        // Poll for the Esc key, non blocking
        if let Ok(esc) = poll_escape() {
            if esc {
                cancel_tx
                    .send(1)
                    .expect("an unrecoverable error while sending over cancel_tx");
                break;
            }
        }
    });

    let mut cancelled = false;
    loop {
        found_ports = enumerate_usb_serial_ports();

        // If we are waiting specifically for a certain port, loop until
        // it is part of the detected ports.
        let found = check_requested_port(&found_ports, path);
        if found {
            // Notify the cancellation thread
            done_tx
                .send(1)
                .expect("an unrecoverable error while sending over done_tx");

            pb.finish_with_message(format!("👍 Serial port {} is ready", style(path).green()));
            break;
        }

        // Update the progress message and wait for some time (receiving until
        // timeout from the cancellation channel) before enumerating serial
        // devices again.
        let num_ports = found_ports.len();
        let waited = attempt * waiting_period;
        pb.set_message(format!(
            "[{:03}s {}] ⏳ Waiting for {} to be ready (ESC to cancel)...",
            style(waited).dim(),
            num_ports,
            style(path).cyan()
        ));

        match cancel_rx.recv_timeout(Duration::from_secs(waiting_period as u64)) {
            Ok(_) => {
                // we got cancelled
                pb.finish_with_message(format!(
                    "❌ Waiting on port {} canceled after {} seconds",
                    style(path).cyan(),
                    style(waited).dim()
                ));
                cancelled = true;
                break;
            }
            Err(RecvTimeoutError::Timeout) => {
                // try again after a timeout
            }
            Err(RecvTimeoutError::Disconnected) => {
                // no point in waiting anymore :'(
                cancelled = true;
                break;
            }
        }

        attempt += 1;
    }

    // Join the cancellation thread
    cancelation_thread
        .join()
        .expect("an unrecoverable error while joining the cancellation thread");

    cancelled
}

pub(crate) fn open_and_setup_port(
    settings: &Settings,
) -> Result<Box<dyn SerialPort>, serialport::Error> {
    use retry::{delay, retry_with_index};

    let result = retry_with_index(
        delay::Fixed::from_millis(1000).take(4),
        |index| -> Result<Box<dyn SerialPort>, serialport::Error> {
            debug!("Trying to connect {}", index);
            // Open the port
            let path = settings.path.clone().unwrap();
            let builder = serialport::new(&path, settings.baud_rate)
                .data_bits(settings.data_bits)
                .stop_bits(settings.stop_bits)
                .parity(settings.parity)
                .flow_control(settings.flow_control);
            builder.open()
        },
    );
    match result {
        Ok(mut port) => {
            // Configure the port with the values in `settings`. TODO: This is
            // probably temporary until `serialport` configures the port after
            // `open` by itself.
            port.set_baud_rate(settings.baud_rate)?;
            port.set_data_bits(settings.data_bits)?;
            port.set_stop_bits(settings.stop_bits)?;
            port.set_parity(settings.parity)?;
            port.set_flow_control(settings.flow_control)?;

            info!(
                "Connected to {} at {} baud",
                port.name().unwrap(),
                port.baud_rate().unwrap()
            );
            debug!("data_bits    : {:#?}", port.data_bits().unwrap());
            debug!("stop_bits    : {:#?}", port.stop_bits().unwrap());
            debug!("parity       : {:#?}", port.parity().unwrap());
            debug!("flow control : {:#?}", port.flow_control().unwrap());

            assert_eq!(
                settings.baud_rate,
                port.baud_rate().unwrap(),
                "\n\n\
                 --> Failed to set the baud rate to the desired value {} which\n    \
                 is probably because it is not a valid one.\n    \
                 Change it to a good one in the command line arguments, or\n    \
                 don't specify it at all. The default value will be used.\n\
                 \n",
                settings.baud_rate
            );
            assert_eq!(settings.data_bits, port.data_bits().unwrap());
            assert_eq!(settings.stop_bits, port.stop_bits().unwrap());
            assert_eq!(settings.parity, port.parity().unwrap());

            Ok(port)
        }
        Err(err) => match err {
            retry::Error::Operation {
                error,
                total_delay,
                tries,
            } => {
                info!(
                    "Failed to open the port after {:?} and {} tries: {}",
                    total_delay, tries, error,
                );
                Err(error)
            }
            retry::Error::Internal(_) => {
                info!("Internal retry error while opening port");
                Err(serialport::Error::new(
                    serialport::ErrorKind::Unknown,
                    "internal eror while retrying to open the port",
                ))
            }
        },
    }
}

//==============================================================================
// Private stuff
//==============================================================================

fn check_requested_port(ports: &[String], path: &str) -> bool {
    for detected_port in ports {
        if detected_port.starts_with(path) {
            return true;
        }
    }
    false
}

/// Enumerates serial devices of type USB on the system
fn enumerate_usb_serial_ports() -> Vec<String> {
    let mut usb_ports = vec![];
    match available_ports() {
        Ok(ports) => {
            for p in ports {
                match p.port_type {
                    // USB ports give us more info about the connected serial
                    // controller
                    SerialPortType::UsbPort(info) => {
                        let extended_name = format!(
                            "{}: ({} / {})",
                            p.port_name,
                            info.manufacturer.as_ref().map_or("", String::as_str),
                            info.product.as_ref().map_or("", String::as_str)
                        );
                        usb_ports.push(extended_name);
                    }
                    // We're also interested in the other devices, such as
                    // virtual ports for testing
                    _ => {
                        usb_ports.push(p.port_name);
                    }
                }
            }
        }
        Err(ref e) => {
            info!("error: {}", e.to_string());
        }
    }
    usb_ports
}

fn select_port_interactive(ports: &[String]) -> Option<String> {
    use dialoguer::{theme::ColorfulTheme, Select};

    // If we are waiting specifically for a certain port (name in
    // `requested_port`, check if it is part of the detected ports; otherwise
    // present the list of detected ports to the user to optionally select one
    // out of them.

    let term = Term::buffered_stderr();
    let theme = ColorfulTheme::default();

    let mut select = Select::with_theme(&theme);
    // select.with_prompt("Confirm which port to use:");
    for item in ports {
        select.item(item);
    }

    let selection = select.default(0).interact_on_opt(&term).unwrap();
    selection.map(|x| String::from(ports.get(x).unwrap().split(':').next().unwrap()))
}
