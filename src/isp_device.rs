use core::panic;
use std::{str::FromStr, thread, time};

use indicatif::ProgressBar;
use log::{debug, error};
use thiserror::Error;

use crate::{device_spec::*, is_expected_error, util, VerificationError};

extern crate hidapi;

use hidapi::{HidDevice, HidError};

const COMMAND_LENGTH: usize = 6;

const REPORT_ID_CMD: u8 = 0x05;
const REPORT_ID_XFER: u8 = 0x06;

const CMD_ENABLE_FIRMWARE: u8 = 0x55;
const CMD_INIT_READ: u8 = 0x52;
const CMD_INIT_WRITE: u8 = 0x57;
const CMD_ERASE: u8 = 0x45;
const CMD_REBOOT: u8 = 0x5a;

const XFER_READ_PAGE: u8 = 0x72;
const XFER_WRITE_PAGE: u8 = 0x77;

pub struct ISPDevice {
    cmd_device: HidDevice,
    #[cfg(target_os = "windows")]
    xfer_device: HidDevice,
    device_spec: DeviceSpec,
}

#[derive(Debug, Error)]
pub enum ISPError {
    #[error(transparent)]
    HidError(#[from] HidError),
    #[error(transparent)]
    VerificationError(#[from] VerificationError),
    #[error("Read/Write operation mistmatch")]
    ReadWriteMismatch,
}

#[derive(Debug, Clone)]
pub enum ReadSection {
    Firmware,
    Bootloader,
    Full,
}

impl ReadSection {
    pub fn to_str(&self) -> &'static str {
        match self {
            ReadSection::Firmware => "firmware",
            ReadSection::Bootloader => "bootloader",
            ReadSection::Full => "full",
        }
    }

    pub fn available_sections() -> Vec<&'static str> {
        vec![
            ReadSection::Firmware.to_str(),
            ReadSection::Bootloader.to_str(),
            ReadSection::Full.to_str(),
        ]
    }
}

impl FromStr for ReadSection {
    type Err = ();
    fn from_str(section: &str) -> Result<Self, Self::Err> {
        Ok(match section {
            "bootloader" => ReadSection::Bootloader,
            "full" => ReadSection::Full,
            "firmware" => ReadSection::Firmware,
            _ => panic!("Invalid read section: {}", section),
        })
    }
}

impl ISPDevice {
    #[cfg(not(target_os = "windows"))]
    pub fn new(device_spec: DeviceSpec, device: HidDevice) -> Self {
        Self {
            cmd_device: device,
            device_spec,
        }
    }

    #[cfg(target_os = "windows")]
    pub fn new(device_spec: DeviceSpec, cmd_device: HidDevice, xfer_device: HidDevice) -> Self {
        Self {
            cmd_device,
            xfer_device,
            device_spec,
        }
    }

    pub fn read_cycle(&self, read_fragment: ReadSection) -> Result<Vec<u8>, ISPError> {
        self.enable_firmware()?;

        let (start_addr, length) = match read_fragment {
            ReadSection::Firmware => (0, self.device_spec.platform.firmware_size),
            ReadSection::Bootloader => (
                self.device_spec.platform.firmware_size,
                self.device_spec.platform.bootloader_size,
            ),
            ReadSection::Full => (
                0,
                self.device_spec.platform.firmware_size + self.device_spec.platform.bootloader_size,
            ),
        };

        let firmware = self.read(start_addr, length)?;

        if self.device_spec.reboot {
            self.reboot();
        }

        Ok(firmware)
    }

    pub fn write_cycle(&self, firmware: &mut [u8]) -> Result<(), ISPError> {
        // ensure that the address at <firmware_size-4> is the same as the reset vector
        firmware.copy_within(1..3, self.device_spec.platform.firmware_size - 4);

        self.erase()?;
        self.write(0, firmware)?;

        // cleanup the address at <firmware_size-4>
        firmware[self.device_spec.platform.firmware_size - 4
            ..self.device_spec.platform.firmware_size - 2]
            .fill(0);

        let read_back = self.read(0, self.device_spec.platform.firmware_size)?;

        eprintln!("Verifying...");
        util::verify(firmware, &read_back).map_err(ISPError::from)?;

        self.enable_firmware()?;

        if self.device_spec.reboot {
            self.reboot();
        }

        Ok(())
    }

    fn xfer_device(&self) -> &HidDevice {
        #[cfg(target_os = "windows")]
        return &self.xfer_device;
        #[cfg(not(target_os = "windows"))]
        &self.cmd_device
    }

    fn read(&self, start_addr: usize, length: usize) -> Result<Vec<u8>, ISPError> {
        let page_size = self.device_spec.platform.page_size;
        let num_page = length / page_size;
        let mut result: Vec<u8> = vec![];

        eprintln!("Reading...");
        let bar = ProgressBar::new(num_page as u64);

        self.init_read(start_addr)?;

        for i in 0..num_page {
            bar.inc(1);
            debug!(
                "Reading page {} @ offset {:#06x}",
                i,
                start_addr + i * page_size
            );
            self.read_page(&mut result)?;
        }
        bar.finish();
        Ok(result)
    }

    fn write(&self, start_addr: usize, buffer: &[u8]) -> Result<(), ISPError> {
        eprintln!("Writing...");
        let bar = ProgressBar::new(self.device_spec.num_pages() as u64);
        self.init_write(start_addr)?;

        let page_size = self.device_spec.platform.page_size;
        for i in 0..self.device_spec.num_pages() {
            bar.inc(1);
            debug!("Writing page {} @ offset {:#06x}", i, i * page_size);
            self.write_page(&buffer[(i * page_size)..((i + 1) * page_size)])?;
        }
        bar.finish();
        Ok(())
    }

    /// Initializes the read operation / sets the initial read address
    fn init_read(&self, start_addr: usize) -> Result<(), ISPError> {
        let cmd: [u8; COMMAND_LENGTH] = [
            REPORT_ID_CMD,
            CMD_INIT_READ,
            (start_addr & 0xff) as u8,
            (start_addr >> 8) as u8,
            0,
            0,
        ];
        self.cmd_device
            .send_feature_report(&cmd)
            .map_err(ISPError::from)?;
        Ok(())
    }

    /// Initializes the write operation / sets the initial write address
    fn init_write(&self, start_addr: usize) -> Result<(), ISPError> {
        let cmd: [u8; COMMAND_LENGTH] = [
            REPORT_ID_CMD,
            CMD_INIT_WRITE,
            (start_addr & 0xff) as u8,
            (start_addr >> 8) as u8,
            0,
            0,
        ];
        self.cmd_device
            .send_feature_report(&cmd)
            .map_err(ISPError::from)?;
        Ok(())
    }

    /// Reads one page of flash contents
    fn read_page(&self, buf: &mut Vec<u8>) -> Result<(), ISPError> {
        let page_size = self.device_spec.platform.page_size;
        let mut xfer_buf: Vec<u8> = vec![0; page_size + 2];
        xfer_buf[0] = REPORT_ID_XFER;
        self.xfer_device()
            .get_feature_report(&mut xfer_buf)
            .map_err(ISPError::from)?;
        buf.extend_from_slice(&xfer_buf[2..(page_size + 2)]);
        if xfer_buf[1] != XFER_READ_PAGE {
            return Err(ISPError::ReadWriteMismatch);
        }
        Ok(())
    }

    /// Writes one page to flash
    ///
    /// Note: The first 3 bytes at address 0x0000 (first-page) are skipped. Instead the second and
    /// third bytes (firmware's reset vector LJMP destination address) are written to address
    /// <firmware_size-4> and will later be part of the LJMP instruction after the firmware is
    /// enabled (`enable_firmware`). This only works once after an erase operation.
    fn write_page(&self, buf: &[u8]) -> Result<(), ISPError> {
        let length = buf.len() + 2;
        let mut xfer_buf: Vec<u8> = vec![0; length];
        xfer_buf[0] = REPORT_ID_XFER;
        xfer_buf[1] = XFER_WRITE_PAGE;
        xfer_buf[2..length].clone_from_slice(buf);
        self.xfer_device()
            .send_feature_report(&xfer_buf)
            .map_err(ISPError::from)?;
        if xfer_buf[1] != XFER_WRITE_PAGE {
            return Err(ISPError::ReadWriteMismatch);
        }
        Ok(())
    }

    /// Sets a LJMP (0x02) opcode at <firmware_size-5>.
    /// This enables the main firmware by making the bootloader jump to it on reset.
    ///
    /// Side-effect: enables reading the firmware without erasing flash first.
    /// Credits to @gashtaan for finding this out.
    fn enable_firmware(&self) -> Result<(), ISPError> {
        eprintln!("Enabling firmware...");
        let cmd: [u8; COMMAND_LENGTH] = [REPORT_ID_CMD, CMD_ENABLE_FIRMWARE, 0, 0, 0, 0];

        self.cmd_device.send_feature_report(&cmd)?;
        Ok(())
    }

    /// Erases everything in flash, except the ISP bootloader section itself and initializes the
    /// reset vector to jump to ISP.
    fn erase(&self) -> Result<(), ISPError> {
        eprintln!("Erasing...");
        let cmd: [u8; COMMAND_LENGTH] = [REPORT_ID_CMD, CMD_ERASE, 0, 0, 0, 0];
        self.cmd_device
            .send_feature_report(&cmd)
            .map_err(ISPError::from)?;
        thread::sleep(time::Duration::from_millis(2000));
        Ok(())
    }

    /// Causes the device to start running the main firmware
    fn reboot(&self) {
        eprintln!("Rebooting...");
        let cmd: [u8; COMMAND_LENGTH] = [REPORT_ID_CMD, CMD_REBOOT, 0, 0, 0, 0];
        if let Err(err) = self.cmd_device.send_feature_report(&cmd) {
            debug!("Error: {:}", err);
            if !is_expected_error(&err) {
                error!("Unexpected error: {:}", err);
            }
        }
        thread::sleep(time::Duration::from_millis(2000));
    }
}
