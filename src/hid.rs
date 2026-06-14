//! HID backend selection.
//!
//! hidra exposes one `HidApi`/`HidDevice` regardless of backend. The default
//! is the per-OS native backend (hidraw on Linux); the `nusb` feature switches
//! hidra to its nusb-based USB transport (the equivalent of the old hidapi
//! `linux-static-libusb` setup). Every hidra I/O method returns a future;
//! this tool drives them synchronously with `MaybeFuture::wait`.

pub use hidra::{
    BusType, DeviceInfo, HidApi, HidDevice, HidError, MaybeFuture, MAX_REPORT_DESCRIPTOR_SIZE,
};
