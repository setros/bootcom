//! Helper macros for the boot protocol state machine modules.

/// Generate debug formatting code for a [`SerialPort`](serialport::SerialPort)
/// like struct.
#[macro_export]
macro_rules! debug_fmt_serialport {
    ($port:ident, $f:ident) => {
        $f.debug_tuple("")
            .field(&$port.name())
            .field(&$port.baud_rate())
            .field(&$port.data_bits())
            .field(&$port.stop_bits())
            .field(&$port.parity())
            .field(&$port.flow_control())
    };
}
