//! HID backend selection.
//!
//! On Linux the backend is chosen at build time: the default `hidraw` backend,
//! or the nusb-based USB backend when built with the `nusb` feature (the
//! equivalent of the old hidapi `linux-static-libusb` setup). macOS and
//! Windows always use the platform's native HID stack. Every backend exposes
//! the same method surface.

#[cfg(all(target_os = "linux", feature = "nusb"))]
pub use hidra::usb::{UsbHidApi as HidApi, UsbHidDevice as HidDevice};

#[cfg(not(all(target_os = "linux", feature = "nusb")))]
pub use hidra::{HidApi, HidDevice};

pub use hidra::{BusType, DeviceInfo, HidError, MAX_REPORT_DESCRIPTOR_SIZE};
