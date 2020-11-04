//! Bootcom command line interface.

use std::process;

use clap::{
    crate_authors, crate_description, crate_name, crate_version, value_t, App, AppSettings::*, Arg,
};
use console::style;
use log::{debug, trace, LevelFilter};
use serialport::{DataBits, FlowControl, Parity, StopBits};
use simplelog::*;

use bootcom::{self as bc, DeviceManager};

fn main() {
    println!("[BC] bootcom v{}", crate_version!());

    ctrlc::set_handler(move || {
        println!("🛑 received Ctrl+C!");
        process::exit(0);
    })
    .expect("Failed to install my Ctrl-C handler!");

    let matches = App::new(crate_name!())
        .version(format!("v{}", crate_version!()).as_str())
        .author(crate_authors!())
        .about(crate_description!())
        .long_about(
            "\n\
            Bootcom works in tandem with the bootloader to push a kernel \
            image over the serial port. When started, it goes into a simple \
            terminal mode. Any input it gets from stdin is passed to the board \
            on the other side of the serial line, and prints any data it gets \
            from the board to to stdout.\n\
            \n\
            The bootloader sends a series of 3 breaks (0x03) when it wants to \
            load the kernel image and Bootcom will answer that by switching to \
            kernel image sending mode: \n\
               \t* reads the file from disk \n\
               \t* send the kernel size as 4 bytes, lowest order first \n\
               \t* waits for 'OK' \n\
               \t* sends the kernel image \n\
            \n\
            After that it goes back into terminal mode.\n\
            \n\
            Bootcom can be started before or after the bootloader is running. \
            It can also properly manage unplugging and re-plugging of the USB \
            cable.\
        ",
        )
        .max_term_width(80)
        .setting(ColoredHelp)
        .setting(NextLineHelp)
        .arg(
            Arg::with_name("DEVICE_TTY")
                .help("the USB tty device to use")
                .long_help(
                    "the USB tty device to use; may change when the board \
                     is unplugged and re-plugged and may differ between \
                     systems. You can opt for selecting a new device while \
                     `bootcom` is running.",
                )
                .short("-t")
                .long("--tty")
                .takes_value(true)
                .require_equals(true),
        )
        .arg(
            Arg::with_name("BAUD_RATE")
                .help("serial port baud rate")
                .long_help("serial baud rate")
                .short("-b")
                .long("--baud-rate")
                .takes_value(true)
                .default_value("230400")
                .require_equals(true),
        )
        .arg(
            Arg::with_name("DATA_BITS")
                .help("number of bits per character")
                .short("-d")
                .long("--data-bits")
                .takes_value(true)
                .possible_values(&["5", "6", "7", "8"])
                .default_value("8")
                .require_equals(true),
        )
        .arg(
            Arg::with_name("STOP_BITS")
                .help("number of stop bits per byte")
                .short("-s")
                .long("--stop-bits")
                .takes_value(true)
                .possible_values(&["1", "2"])
                .default_value("1")
                .require_equals(true),
        )
        .arg(
            Arg::with_name("PARITY")
                .help("parity checking protocol")
                .short("-p")
                .long("--parity")
                .takes_value(true)
                .possible_values(&["none", "odd", "even"])
                .default_value("none")
                .require_equals(true),
        )
        .arg(
            Arg::with_name("FLOW_CONTROL")
                .help("flow control mode")
                .short("-f")
                .long("--flow-control")
                .takes_value(true)
                .possible_values(&["none", "soft", "hard"])
                .default_value("none")
                .require_equals(true),
        )
        .arg(
            Arg::with_name("KERNEL_IMAGE")
                .help("path to the kernel image to be pushed")
                .long_help(
                    "path to the kernel image to be pushed; when not \
                     set, `bootcom` will look for `kernel8.img` in the current \
                     working directory.",
                )
                .index(1),
        )
        .arg(Arg::with_name("v").short("v").multiple(true).help(
            "Sets the logging level of verbosity, repeat several times for \
                higher verbosity",
        ))
        .get_matches();

    // Vary the output based on how many times the user used the "verbose" flag
    // (i.e. 'bootcom -v -v -v' or 'bootcom -vvv' vs 'bootcom -v'
    let log_level: LevelFilter;
    match matches.occurrences_of("v") {
        0 => log_level = LevelFilter::Warn,
        1 => log_level = LevelFilter::Info,
        2 => log_level = LevelFilter::Debug,
        _ => log_level = LevelFilter::Trace,
    }

    TermLogger::init(log_level, Config::default(), TerminalMode::Mixed).unwrap();

    trace!("{:#?}", matches);

    // Arguments with default values ===========================================

    // It's safe to call unwrap on all command line arguments with default
    // values, because the value with either be what the user input at runtime
    // or the default value

    let baud_rate = value_t!(matches.value_of("BAUD_RATE"), u32).unwrap_or_else(|_| {
        println!(
            "{}: `{}` needs to be a numeric value",
            style("error").red(),
            style("baud-rate").cyan()
        );
        println!(
            "   {} `{}` is not a valid value",
            style("-->").cyan(),
            style(matches.value_of("BAUD_RATE").unwrap()).on_red()
        );
        process::exit(-1);
    });

    let data_bits = match matches.value_of("DATA_BITS").unwrap() {
        "5" => DataBits::Five,
        "6" => DataBits::Six,
        "7" => DataBits::Seven,
        "8" => DataBits::Eight,
        _ => unreachable!(),
    };

    let stop_bits = match matches.value_of("STOP_BITS").unwrap() {
        "1" => StopBits::One,
        "2" => StopBits::Two,
        _ => unreachable!(),
    };

    let parity = match matches.value_of("PARITY").unwrap() {
        "none" => Parity::None,
        "even" => Parity::Even,
        "odd" => Parity::Odd,
        _ => unreachable!(),
    };

    let flow_control = match matches.value_of("FLOW_CONTROL").unwrap() {
        "none" => FlowControl::None,
        "soft" => FlowControl::Software,
        "hard" => FlowControl::Hardware,
        _ => unreachable!(),
    };

    // END - Arguments with default values =====================================

    let mut settings = bc::SettingsBuilder::default()
        .baud_rate(baud_rate)
        .data_bits(data_bits)
        .stop_bits(stop_bits)
        .parity(parity)
        .flow_control(flow_control)
        .finalize();

    // START - Arguments with NO default values ================================

    if matches.is_present("DEVICE_TTY") {
        settings.path = Some(matches.value_of("DEVICE_TTY").unwrap().into());
    }

    if matches.is_present("KERNEL_IMAGE") {
        settings.kernel_image = Some(matches.value_of("KERNEL_IMAGE").unwrap().into());
    }

    // END - Arguments =========================================================

    // Run the state machine ===================================================

    let mut sdm = bc::singleton(settings);
    let exit_code = sdm.run();
    debug!("exit code: {}", exit_code);
    std::process::exit(exit_code.into());
}
