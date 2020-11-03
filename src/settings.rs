//! Settings related to bootcom serial port and device/protocol implementation.
//!
//! Use the [builder](https://doc.rust-lang.org/1.0.0/style/ownership/builders.html)
//! pattern to set the configurable values.

pub use serialport::{DataBits, FlowControl, Parity, StopBits};

// =============================================================================
// Public Interface
// =============================================================================

/// Groups all settings related to the serial port used by `bootcom` and acts as
/// a [builder](https://doc.rust-lang.org/1.0.0/style/ownership/builders.html)
/// for the settings.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Settings {
    /// The port name, usually the device path.
    pub path: Option<String>,
    /// The baud rate in symbols-per-second.
    pub baud_rate: u32,
    /// Number of bits used to represent a character sent on the line.
    pub data_bits: DataBits,
    /// The type of signalling to use for controlling data transfer.
    pub flow_control: FlowControl,
    /// The type of parity to use for error checking.
    pub parity: Parity,
    /// Number of bits to use to signal the end of a character.
    pub stop_bits: StopBits,

    /// Path to the kernel image to be pushed. Optional, when not set, `bootcom`
    /// will look for `kernel8.img` in the current working directory and if none
    /// was found, it will offer the list of files ending with `.img` in the
    /// current working directory for selection by the user.
    pub kernel_image: Option<String>,

    /// Restrict creation of `Settings` instances unless through the
    /// `SettingsBuilder`.
    #[doc(hidden)]
    _private_use_builder: (),
}

/// The builder for the `Settings` values.
///
/// All values are optional and have default values that will be used if not
/// explicitly set.
///
/// **Example**
///
/// ```ignore
/// let settings = SettingsBuilder::new().path("/dev/ttyUSB0").finalize();
/// ```
pub struct SettingsBuilder {
    settings: Settings,
}
impl SettingsBuilder {
    /// Start building the settings using default values and no path for the
    /// port.
    pub fn new() -> Self {
        SettingsBuilder {
            settings: Settings {
                path: None,
                baud_rate: 230_400,
                data_bits: DataBits::Eight,
                flow_control: FlowControl::None,
                parity: Parity::None,
                stop_bits: StopBits::One,
                kernel_image: None,
                _private_use_builder: (),
            },
        }
    }

    /// Set the path to the serial port
    pub fn path<'a>(mut self, path: impl Into<std::borrow::Cow<'a, str>>) -> Self {
        self.settings.path = Some(path.into().as_ref().to_owned());
        self
    }

    /// Set the baud rate in symbols-per-second
    pub fn baud_rate(mut self, baud_rate: u32) -> Self {
        self.settings.baud_rate = baud_rate;
        self
    }

    /// Set the number of bits used to represent a character sent on the line
    pub fn data_bits(mut self, data_bits: DataBits) -> Self {
        self.settings.data_bits = data_bits;
        self
    }

    /// Set the type of signalling to use for controlling data transfer
    pub fn flow_control(mut self, flow_control: FlowControl) -> Self {
        self.settings.flow_control = flow_control;
        self
    }

    /// Set the type of parity to use for error checking
    pub fn parity(mut self, parity: Parity) -> Self {
        self.settings.parity = parity;
        self
    }

    /// Set the number of bits to use to signal the end of a character
    pub fn stop_bits(mut self, stop_bits: StopBits) -> Self {
        self.settings.stop_bits = stop_bits;
        self
    }

    /// Set the path to the serial port
    pub fn kernel_image<'a>(mut self, kernel_image: impl Into<std::borrow::Cow<'a, str>>) -> Self {
        self.settings.kernel_image = Some(kernel_image.into().as_ref().to_owned());
        self
    }

    pub fn finalize(self) -> Settings {
        self.settings
    }
}

// =============================================================================
// Unit Tests
// =============================================================================

#[test]
fn all_default() {
    let settings = SettingsBuilder::new().finalize();
    assert_eq!(
        settings,
        Settings {
            path: None,
            baud_rate: 230_400,
            data_bits: DataBits::Eight,
            flow_control: FlowControl::None,
            parity: Parity::None,
            stop_bits: StopBits::One,
            kernel_image: None,
            _private_use_builder: (),
        }
    )
}

#[test]
fn path() {
    let settings = SettingsBuilder::new().path("/dev/ttyUSB0").finalize();
    assert_eq!(settings.path.unwrap(), "/dev/ttyUSB0");
}

#[test]
fn baud_rate() {
    let baud_rate = 96_000;
    let settings = SettingsBuilder::new().baud_rate(baud_rate).finalize();
    assert_eq!(settings.baud_rate, baud_rate);
}

#[test]
fn data_bits() {
    let data_bits = DataBits::Seven;
    let settings = SettingsBuilder::new().data_bits(data_bits).finalize();
    assert_eq!(settings.data_bits, data_bits);
}

#[test]
fn flow_control() {
    let flow_control = FlowControl::Hardware;
    let settings = SettingsBuilder::new().flow_control(flow_control).finalize();
    assert_eq!(settings.flow_control, flow_control);
}

#[test]
fn stop_bits() {
    let stop_bits = StopBits::Two;
    let settings = SettingsBuilder::new().stop_bits(stop_bits).finalize();
    assert_eq!(settings.stop_bits, stop_bits);
}

#[test]
fn parity() {
    let parity = Parity::Even;
    let settings = SettingsBuilder::new().parity(parity).finalize();
    assert_eq!(settings.parity, parity);
}

#[test]
fn kernel_image() {
    let settings = SettingsBuilder::new()
        .kernel_image("test_kernel8.img")
        .finalize();
    assert_eq!(settings.kernel_image.unwrap(), "test_kernel8.img");
}
