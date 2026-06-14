use core::panic;
use core::time::Duration;
use std::str::FromStr;

use hidra::{HidDevice, HidError};
use log::{debug, error};
use thiserror::Error;

use crate::sleep::sleep;
use crate::{device_spec::*, is_expected_error, util, VerificationError};

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

const SETTLE_DELAY: Duration = Duration::from_millis(2000);

/// Progress events emitted while reading or writing.
///
/// The protocol carries no UI of its own; a caller passes a `&dyn Fn(Progress)`
/// and turns these into whatever it needs (a CLI progress bar, log lines, web
/// UI updates, or nothing at all).
#[derive(Debug, Clone)]
pub enum Progress {
    /// A one-off status message for the step that is about to run.
    Status(&'static str),
    /// A counted task is starting (e.g. reading `total` pages).
    TaskStart { label: &'static str, total: usize },
    /// One unit of the current counted task finished.
    TaskAdvance,
    /// The current counted task finished.
    TaskFinish,
}

pub struct ISPDevice {
    cmd_device: HidDevice,
    /// Some platforms (Windows) expose the transfer report on a separate HID
    /// handle; everywhere else it is the same handle as `cmd_device`.
    xfer_device: Option<HidDevice>,
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
    /// Builds an ISP device from one or two open HID handles.
    ///
    /// Pass `xfer_device = None` when the command and transfer reports live on
    /// the same handle (Linux, macOS, WebHID). Pass a separate handle for
    /// platforms that split them across HID collections (Windows).
    pub fn new(
        device_spec: DeviceSpec,
        cmd_device: HidDevice,
        xfer_device: Option<HidDevice>,
    ) -> Self {
        Self {
            cmd_device,
            xfer_device,
            device_spec,
        }
    }

    pub async fn read_cycle(
        &self,
        read_fragment: ReadSection,
        progress: &dyn Fn(Progress),
    ) -> Result<Vec<u8>, ISPError> {
        self.enable_firmware(progress).await?;

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

        let firmware = self.read(start_addr, length, progress).await?;

        if self.device_spec.reboot {
            self.reboot(progress).await;
        }

        Ok(firmware)
    }

    pub async fn write_cycle(
        &self,
        firmware: &mut [u8],
        progress: &dyn Fn(Progress),
    ) -> Result<(), ISPError> {
        // ensure that the address at <firmware_size-4> is the same as the reset vector
        firmware.copy_within(1..3, self.device_spec.platform.firmware_size - 4);

        self.erase(progress).await?;
        self.write(0, firmware, progress).await?;

        // cleanup the address at <firmware_size-4>
        firmware[self.device_spec.platform.firmware_size - 4
            ..self.device_spec.platform.firmware_size - 2]
            .fill(0);

        let read_back = self
            .read(0, self.device_spec.platform.firmware_size, progress)
            .await?;

        progress(Progress::Status("Verifying..."));
        util::verify(firmware, &read_back).map_err(ISPError::from)?;

        self.enable_firmware(progress).await?;

        if self.device_spec.reboot {
            self.reboot(progress).await;
        }

        Ok(())
    }

    fn xfer_device(&self) -> &HidDevice {
        self.xfer_device.as_ref().unwrap_or(&self.cmd_device)
    }

    async fn read(
        &self,
        start_addr: usize,
        length: usize,
        progress: &dyn Fn(Progress),
    ) -> Result<Vec<u8>, ISPError> {
        let page_size = self.device_spec.platform.page_size;
        let num_page = length / page_size;
        let mut result: Vec<u8> = vec![];

        progress(Progress::TaskStart {
            label: "Reading...",
            total: num_page,
        });

        self.init_read(start_addr).await?;

        for i in 0..num_page {
            progress(Progress::TaskAdvance);
            debug!(
                "Reading page {} @ offset {:#06x}",
                i,
                start_addr + i * page_size
            );
            self.read_page(&mut result).await?;
        }
        progress(Progress::TaskFinish);
        Ok(result)
    }

    async fn write(
        &self,
        start_addr: usize,
        buffer: &[u8],
        progress: &dyn Fn(Progress),
    ) -> Result<(), ISPError> {
        progress(Progress::TaskStart {
            label: "Writing...",
            total: self.device_spec.num_pages(),
        });
        self.init_write(start_addr).await?;

        let page_size = self.device_spec.platform.page_size;
        for i in 0..self.device_spec.num_pages() {
            progress(Progress::TaskAdvance);
            debug!("Writing page {} @ offset {:#06x}", i, i * page_size);
            self.write_page(&buffer[(i * page_size)..((i + 1) * page_size)])
                .await?;
        }
        progress(Progress::TaskFinish);
        Ok(())
    }

    /// Initializes the read operation / sets the initial read address
    async fn init_read(&self, start_addr: usize) -> Result<(), ISPError> {
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
            .await
            .map_err(ISPError::from)?;
        Ok(())
    }

    /// Initializes the write operation / sets the initial write address
    async fn init_write(&self, start_addr: usize) -> Result<(), ISPError> {
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
            .await
            .map_err(ISPError::from)?;
        Ok(())
    }

    /// Reads one page of flash contents
    async fn read_page(&self, buf: &mut Vec<u8>) -> Result<(), ISPError> {
        let page_size = self.device_spec.platform.page_size;
        let mut xfer_buf: Vec<u8> = vec![0; page_size + 2];
        xfer_buf[0] = REPORT_ID_XFER;
        self.xfer_device()
            .get_feature_report(&mut xfer_buf)
            .await
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
    async fn write_page(&self, buf: &[u8]) -> Result<(), ISPError> {
        let length = buf.len() + 2;
        let mut xfer_buf: Vec<u8> = vec![0; length];
        xfer_buf[0] = REPORT_ID_XFER;
        xfer_buf[1] = XFER_WRITE_PAGE;
        xfer_buf[2..length].clone_from_slice(buf);
        self.xfer_device()
            .send_feature_report(&xfer_buf)
            .await
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
    async fn enable_firmware(&self, progress: &dyn Fn(Progress)) -> Result<(), ISPError> {
        progress(Progress::Status("Enabling firmware..."));
        let cmd: [u8; COMMAND_LENGTH] = [REPORT_ID_CMD, CMD_ENABLE_FIRMWARE, 0, 0, 0, 0];

        self.cmd_device.send_feature_report(&cmd).await?;
        Ok(())
    }

    /// Erases everything in flash, except the ISP bootloader section itself and initializes the
    /// reset vector to jump to ISP.
    async fn erase(&self, progress: &dyn Fn(Progress)) -> Result<(), ISPError> {
        progress(Progress::Status("Erasing..."));
        let cmd: [u8; COMMAND_LENGTH] = [REPORT_ID_CMD, CMD_ERASE, 0, 0, 0, 0];
        self.cmd_device
            .send_feature_report(&cmd)
            .await
            .map_err(ISPError::from)?;
        sleep(SETTLE_DELAY).await;
        Ok(())
    }

    /// Causes the device to start running the main firmware
    async fn reboot(&self, progress: &dyn Fn(Progress)) {
        progress(Progress::Status("Rebooting..."));
        let cmd: [u8; COMMAND_LENGTH] = [REPORT_ID_CMD, CMD_REBOOT, 0, 0, 0, 0];
        if let Err(err) = self.cmd_device.send_feature_report(&cmd).await {
            debug!("Error: {:}", err);
            if !is_expected_error(&err) {
                error!("Unexpected error: {:}", err);
            }
        }
        sleep(SETTLE_DELAY).await;
    }
}
