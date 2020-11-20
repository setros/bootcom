//! Helper functions to send the kernel data over the serial port.

use std::fs;
use std::{convert::TryInto, io::prelude::*};
use std::{error::Error, fs::File};

use console::{style, Term};
use dialoguer::{theme::ColorfulTheme, Select};
use indicatif::{ProgressBar, ProgressStyle};
use log::{debug, error, info, log_enabled, trace, Level::Debug};
use serialport::{ClearBuffer, SerialPort};

use hexplay::HexViewBuilder;
use std::io::Write;

use crate::Settings;

pub(crate) fn send_kernel(
    port: &mut Box<dyn SerialPort>,
    settings: &Settings,
) -> Result<usize, Box<dyn Error>> {
    let image_path = match &settings.kernel_image {
        Some(value) => value.clone(),
        None => "kernel8.img".into(),
    };

    let mut open_result = File::open(&image_path);
    if let Err(e) = open_result {
        debug!("`{}` error: {}", &image_path, e);
        debug!("Looking for an image file in current directory");

        loop {
            match select_image_file_interactive() {
                Some(ref name) => {
                    if name.ends_with("cancel and go back...") {
                        return Ok(0);
                    }
                    open_result = File::open(name);
                    if let Err(ref e) = open_result {
                        debug!("`{}` error: {}", name, e);
                        println!(
                            "{}",
                            style(format!("[BC] 🙁 could not open `{}`, try again...", name))
                                .yellow()
                        );
                    } else {
                        break;
                    }
                }
                None => {
                    debug!("No kernel image file was selected!");
                    // Try again with arefreshed list of files
                }
            }
        }
    }

    let mut file = open_result?;

    let size = file.metadata()?.len();
    if size > 0xffffffff {
        // The kernel file is too big for the current bootloader protocol which
        // only allows for 4 bytes to be sent for the kernel size.
        return Err(serialport::Error {
            kind: serialport::ErrorKind::InvalidInput,
            description: "kernel file is too big".into(),
        }
        .into());
    }

    write_kernel_size(port, size as u32)?;

    write_kernel_image(port, &mut file, size as u32)?;

    Ok(0)
}

fn write_kernel_size(port: &mut Box<dyn SerialPort>, size: u32) -> Result<(), Box<dyn Error>> {
    use retry::{delay, retry};

    // Clear the port input buffer
    port.clear(ClearBuffer::Input)?;

    // Write the 4 bytes for the size in little endian
    let bytes = size.to_le_bytes();
    port.write_all(&bytes)?;

    // Expect a response with 'O''K' coming back from the bootloader
    let mut ok: Vec<u8> = vec![0; 2];
    let result = retry(
        delay::Fixed::from_millis(1000).take(9),
        || -> Result<usize, Box<dyn Error>> {
            let available = port.bytes_to_read()?;
            trace!("Bytes available to read: {}", available);

            if available >= 2 {
                port.read_exact(ok.as_mut_slice()).unwrap();
                return Ok(2);
            }

            Err(serialport::Error {
                kind: serialport::ErrorKind::Unknown,
                description: "did not receive OK in time".into(),
            }
            .into())
        },
    );

    match result {
        Ok(2) => {
            // Dump the received data in a hex table for
            // debugging
            if log_enabled!(Debug) {
                let view = HexViewBuilder::new(&ok)
                    .address_offset(0)
                    .row_width(16)
                    .finish();
                println!("{}", view);
            }
            Ok(())
        }
        _ => {
            info!("error: {:?}", result.unwrap_err());
            Err(serialport::Error {
                kind: serialport::ErrorKind::InvalidInput,
                description: "kernel size was not confirmed with `OK`".into(),
            }
            .into())
        }
    }
}

fn write_kernel_image(
    port: &mut Box<dyn SerialPort>,
    file: &mut File,
    size: u32,
) -> Result<(), serialport::Error> {
    let mut written: usize = 0;
    let mut chunk: Vec<u8> = vec![0; 1024];

    let pb = ProgressBar::new(size.into());
    pb.set_style(ProgressStyle::default_bar()
        .template("[BC] ⏩ Pushing [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})")
        .progress_chars("=>-"));

    while (written as u32) < size {
        let bytes_in = file.read(&mut chunk)?;
        trace!("{} bytes read from input file", { bytes_in });
        loop {
            match port.write(&chunk[..bytes_in]) {
                Ok(bytes_out) => {
                    trace!("{} bytes written to serial port", { bytes_out });
                    assert_eq!(bytes_in, bytes_out);

                    written += bytes_in;
                    pb.set_position(written.try_into().unwrap());
                    break;
                }
                Err(err) => {
                    if err.kind() == std::io::ErrorKind::TimedOut {
                    } else {
                        error!("{}", err);
                        return Err(err.into());
                    }
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
    }
    pb.finish_with_message("[BC] Kernel uploaded");

    Ok(())
}

fn select_image_file_interactive() -> Option<String> {
    // List files ending with ".img" in the current working directory and
    // ask the user to select one out of them.
    match fs::read_dir(".") {
        Ok(files) => {
            let mut items: Vec<String> = Vec::new();
            files
                .filter_map(Result::ok)
                .filter(|f| f.path().extension().unwrap_or_default() == "img")
                .for_each(|f| {
                    let name = f.file_name();
                    items.push(name.to_str().unwrap().into());
                });

            if items.is_empty() {
                debug!("There are no image files in the current directory");
            }

            items.push("🔙cancel and go back...".into());

            let selection = Select::with_theme(&ColorfulTheme::default())
                .items(&items)
                .with_prompt(format!(
                    "Select a kernel image file to push (`{}` to refresh):",
                    style("Esc").cyan()
                ))
                .default(0)
                .interact_on_opt(&Term::stdout());

            match selection {
                Ok(Some(index)) => Some(items[index].clone()),
                Ok(None) => {
                    debug!("user did not select any kernel image file");
                    None
                }
                Err(ref e) => {
                    info!("error: {}", e.to_string());
                    None
                }
            }
        }
        Err(ref e) => {
            info!("error: {}", e.to_string());
            None
        }
    }
}
