//! Async primitives for the Sinowealth 8051 HID ISP bootloader protocol.
//!
//! Each protocol operation ([`ISPDevice::enable_firmware`], [`ISPDevice::erase`],
//! [`ISPDevice::read_page`], ...) is an `async` method over a [`hidra::HidDevice`],
//! so the exact same code drives a native CLI (by calling
//! [`hidra::MaybeFuture::wait`] on the returned futures) and a web-based flasher
//! (by `.await`-ing them on `wasm32` with the WebHID backend).
//!
//! The crate carries no orchestration, timing, or UI: composing these
//! primitives into full read/write cycles, inserting the post-erase/reboot
//! settle delays, and reporting progress are the caller's responsibility (see
//! the `sinowealth-kb-tool` CLI for a reference implementation).

mod device_spec;
mod ihex;
mod isp_device;
mod platform_spec;
mod util;

pub use device_spec::*;
pub use ihex::*;
pub use isp_device::*;
pub use platform_spec::*;
pub use util::*;

// hidra types that appear in this crate's public API, re-exported so callers
// use the same backend-selected types this crate was built against.
pub use hidra::{HidDevice, HidError};
