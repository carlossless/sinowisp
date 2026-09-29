//! An in-memory ISP bootloader for exercising [`ISPDevice`](crate::ISPDevice)
//! without hardware.
//!
//! [`FakeBootloader`] implements [`Transport`] for `&FakeBootloader`, so a test
//! keeps ownership and can inspect the flash and the reports sent after the
//! device is done with it:
//!
//! ```
//! # use sinowisp::{testing::FakeBootloader, ISPDevice, DEVICE_BASE_SH68F90};
//! # use hidra::MaybeFuture;
//! let fake = FakeBootloader::new(DEVICE_BASE_SH68F90);
//! let device = ISPDevice::with_transport(DEVICE_BASE_SH68F90, &fake, None);
//! device.erase().wait().unwrap();
//! assert_eq!(fake.sent().len(), 1);
//! ```

use std::{
    cell::{Cell, RefCell},
    ops::Range,
};

use hidra::HidError;

use crate::{
    isp_device::{
        CMD_ENABLE_FIRMWARE, CMD_ERASE, CMD_INIT_READ, CMD_INIT_WRITE, REPORT_ID_CMD,
        REPORT_ID_XFER, XFER_READ_PAGE, XFER_WRITE_PAGE,
    },
    DeviceSpec, Transport,
};

/// Emulates the bootloader's command and transfer reports over a flat flash array.
///
/// Models how the factory bootloaders keep physical `0x0000` pointing at
/// themselves while presenting the application's layout to the host:
///
/// - erase clears the application region and reprograms `0x0000-0x0002` as
///   `LJMP <firmware_size>`, the bootloader;
/// - writes to `0x0000` are dropped, writes to `0x0001-0x0002` land at
///   `<firmware_size-4>-<firmware_size-3>`, and writes at or above
///   `firmware_size` are ignored;
/// - enabling the firmware programs the `0x02` marker at `<firmware_size-5>`;
/// - reads of `0x0000-0x0002` return `02` and the relocated pair, and
///   `<firmware_size-5>-<firmware_size-3>` read as `00 00 00`.
///
/// It does not model the byte mangling some bootloaders apply
/// (`isp_transform`); transfer bytes are stored exactly as they arrive.
pub struct FakeBootloader {
    firmware_size: usize,
    flash: RefCell<Vec<u8>>,
    addr: Cell<usize>,
    read_type: Cell<u8>,
    sent: RefCell<Vec<Vec<u8>>>,
}

impl FakeBootloader {
    /// A blank bootloader sized for `spec`'s flash.
    pub fn new(spec: DeviceSpec) -> Self {
        Self::with_flash(spec, vec![0; spec.total_flash_size()])
    }

    /// A bootloader whose physical flash starts out as `flash`.
    pub fn with_flash(spec: DeviceSpec, flash: Vec<u8>) -> Self {
        Self {
            firmware_size: spec.platform.firmware_size,
            flash: RefCell::new(flash),
            addr: Cell::new(0),
            read_type: Cell::new(XFER_READ_PAGE),
            sent: RefCell::new(vec![]),
        }
    }

    /// The physical flash contents, without the read redirection applied.
    pub fn flash(&self) -> Vec<u8> {
        self.flash.borrow().clone()
    }

    /// Every feature report sent to the device, in order.
    pub fn sent(&self) -> Vec<Vec<u8>> {
        self.sent.borrow().clone()
    }

    /// Makes transfer reads answer with `read_type` in place of the read-page marker.
    pub fn set_read_type(&self, read_type: u8) {
        self.read_type.set(read_type);
    }

    fn masked(&self) -> Range<usize> {
        self.firmware_size - 5..self.firmware_size - 2
    }

    fn physical(&self, addr: usize) -> usize {
        match addr {
            1 | 2 => self.firmware_size - 4 + (addr - 1),
            _ => addr,
        }
    }

    fn program(&self, addr: usize, byte: u8) {
        if addr != 0 && addr < self.firmware_size {
            self.flash.borrow_mut()[self.physical(addr)] = byte;
        }
    }

    fn read(&self, addr: usize) -> u8 {
        if addr == 0 {
            0x02
        } else if self.masked().contains(&addr) {
            0
        } else {
            self.flash.borrow()[self.physical(addr)]
        }
    }

    fn receive(&self, data: &[u8]) {
        self.sent.borrow_mut().push(data.to_vec());
        match (data[0], data[1]) {
            (REPORT_ID_CMD, CMD_INIT_READ | CMD_INIT_WRITE) => {
                self.addr
                    .set(u16::from_le_bytes([data[2], data[3]]) as usize);
            }
            (REPORT_ID_CMD, CMD_ERASE) => {
                let mut flash = self.flash.borrow_mut();
                flash[..self.firmware_size].fill(0);
                flash[0] = 0x02;
                flash[1..3].copy_from_slice(&(self.firmware_size as u16).to_be_bytes());
            }
            (REPORT_ID_CMD, CMD_ENABLE_FIRMWARE) => {
                self.flash.borrow_mut()[self.firmware_size - 5] = 0x02;
            }
            (REPORT_ID_XFER, XFER_WRITE_PAGE) => {
                let start = self.addr.get();
                for (i, byte) in data[2..].iter().enumerate() {
                    self.program(start + i, *byte);
                }
                self.addr.set(start + data.len() - 2);
            }
            _ => {}
        }
    }

    fn respond(&self, buf: &mut [u8]) -> usize {
        assert_eq!(
            buf[0], REPORT_ID_XFER,
            "transfer read with the wrong report id"
        );
        buf[1] = self.read_type.get();
        let start = self.addr.get();
        for (i, byte) in buf[2..].iter_mut().enumerate() {
            *byte = self.read(start + i);
        }
        self.addr.set(start + buf.len() - 2);
        buf.len()
    }
}

impl Transport for &FakeBootloader {
    async fn send_feature_report(&self, data: &[u8]) -> Result<(), HidError> {
        self.receive(data);
        Ok(())
    }

    async fn get_feature_report(&self, buf: &mut [u8]) -> Result<usize, HidError> {
        Ok(self.respond(buf))
    }
}
