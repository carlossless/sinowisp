use core::panic;
use std::str::FromStr;

use hidra::{HidDevice, HidError};
#[cfg(not(target_arch = "wasm32"))]
use hidra::{NativeDevice, NusbDevice};
use thiserror::Error;

use crate::{device_spec::*, platform_spec::InitOperand, VerificationError};

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

/// A HID handle an [`ISPDevice`] talks through.
///
/// hidra backends are types, so holding either is the caller's job; the
/// protocol needs three methods and this forwards them. On wasm there is only
/// WebHID, so it is a plain newtype.
#[cfg(not(target_arch = "wasm32"))]
pub enum ISPHandle {
    /// A handle on the per-OS backend.
    Native(HidDevice<NativeDevice>),
    /// A handle on the raw-USB backend.
    Nusb(HidDevice<NusbDevice>),
}

/// A HID handle an [`ISPDevice`] talks through.
#[cfg(target_arch = "wasm32")]
pub struct ISPHandle(HidDevice);

#[cfg(not(target_arch = "wasm32"))]
impl From<HidDevice<NativeDevice>> for ISPHandle {
    fn from(device: HidDevice<NativeDevice>) -> Self {
        ISPHandle::Native(device)
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl From<HidDevice<NusbDevice>> for ISPHandle {
    fn from(device: HidDevice<NusbDevice>) -> Self {
        ISPHandle::Nusb(device)
    }
}

#[cfg(target_arch = "wasm32")]
impl From<HidDevice> for ISPHandle {
    fn from(device: HidDevice) -> Self {
        ISPHandle(device)
    }
}

/// Forwards the three methods the protocol needs to whichever backend the
/// handle came from.
macro_rules! forward {
    ($self:ident, $call:ident($($arg:expr),*)) => {{
        #[cfg(not(target_arch = "wasm32"))]
        match $self {
            ISPHandle::Native(d) => d.$call($($arg),*).await,
            ISPHandle::Nusb(d) => d.$call($($arg),*).await,
        }
        #[cfg(target_arch = "wasm32")]
        $self.0.$call($($arg),*).await
    }};
}

impl ISPHandle {
    pub async fn send_feature_report(&self, data: &[u8]) -> Result<(), HidError> {
        forward!(self, send_feature_report(data))
    }

    pub async fn get_report_descriptor(&self, buf: &mut [u8]) -> Result<usize, HidError> {
        forward!(self, get_report_descriptor(buf))
    }

    pub async fn get_feature_report(&self, buf: &mut [u8]) -> Result<usize, HidError> {
        forward!(self, get_feature_report(buf))
    }
}

/// One open connection to a device in ISP bootloader mode.
///
/// The methods are the individual protocol operations; they perform no
/// sequencing, delays, or progress reporting. Callers compose them into full
/// read/write cycles (and insert the settle delays after [`erase`](Self::erase)
/// and [`reboot`](Self::reboot)).
pub struct ISPDevice {
    cmd_device: ISPHandle,
    /// Some platforms (Windows) expose the transfer report on a separate HID
    /// handle; everywhere else it is the same handle as `cmd_device`.
    xfer_device: Option<ISPHandle>,
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
        cmd_device: impl Into<ISPHandle>,
        xfer_device: Option<ISPHandle>,
    ) -> Self {
        Self {
            cmd_device: cmd_device.into(),
            xfer_device,
            device_spec,
        }
    }

    /// The spec this device was opened with (firmware/page sizes, reboot flag).
    pub fn device_spec(&self) -> &DeviceSpec {
        &self.device_spec
    }

    fn xfer_device(&self) -> &ISPHandle {
        self.xfer_device.as_ref().unwrap_or(&self.cmd_device)
    }

    /// Sets a LJMP (0x02) opcode at <firmware_size-5>.
    /// This enables the main firmware by making the bootloader jump to it on reset.
    ///
    /// Side-effect: enables reading the firmware without erasing flash first.
    /// Credits to @gashtaan for finding this out.
    pub async fn enable_firmware(&self) -> Result<(), ISPError> {
        let cmd: [u8; COMMAND_LENGTH] = [REPORT_ID_CMD, CMD_ENABLE_FIRMWARE, 0, 0, 0, 0];
        self.cmd_device.send_feature_report(&cmd).await?;
        Ok(())
    }

    fn init_command(&self, command: u8, start_addr: usize, length: usize) -> [u8; COMMAND_LENGTH] {
        match self.device_spec.platform.init_operand {
            InitOperand::Address => [
                REPORT_ID_CMD,
                command,
                (start_addr & 0xff) as u8,
                (start_addr >> 8) as u8,
                0,
                0,
            ],
            InitOperand::Length => [
                REPORT_ID_CMD,
                command,
                0,
                0,
                (length & 0xff) as u8,
                (length >> 8) as u8,
            ],
        }
    }

    /// Initializes the read operation / sets the initial read address or the
    /// total read length, depending on the platform's [`InitOperand`].
    pub async fn init_read(&self, start_addr: usize, length: usize) -> Result<(), ISPError> {
        let cmd = self.init_command(CMD_INIT_READ, start_addr, length);
        self.cmd_device
            .send_feature_report(&cmd)
            .await
            .map_err(ISPError::from)?;
        Ok(())
    }

    /// Initializes the write operation / sets the initial write address or the
    /// total write length, depending on the platform's [`InitOperand`].
    pub async fn init_write(&self, start_addr: usize, length: usize) -> Result<(), ISPError> {
        let cmd = self.init_command(CMD_INIT_WRITE, start_addr, length);
        self.cmd_device
            .send_feature_report(&cmd)
            .await
            .map_err(ISPError::from)?;
        Ok(())
    }

    /// Reads one page of flash contents, appending it to `buf`.
    pub async fn read_page(&self, buf: &mut Vec<u8>) -> Result<(), ISPError> {
        let page_size = self.device_spec.platform.page_size;
        let mut xfer_buf: Vec<u8> = vec![0; page_size + 2];
        xfer_buf[0] = REPORT_ID_XFER;
        self.xfer_device()
            .get_feature_report(&mut xfer_buf)
            .await
            .map_err(ISPError::from)?;
        let page = &xfer_buf[2..(page_size + 2)];
        match self.device_spec.isp_transform {
            Some(transform) => buf.extend(
                page.iter()
                    .enumerate()
                    .map(|(i, b)| (transform.read)(i, *b)),
            ),
            None => buf.extend_from_slice(page),
        }
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
    pub async fn write_page(&self, buf: &[u8]) -> Result<(), ISPError> {
        let length = buf.len() + 2;
        let mut xfer_buf: Vec<u8> = vec![0; length];
        xfer_buf[0] = REPORT_ID_XFER;
        xfer_buf[1] = XFER_WRITE_PAGE;
        match self.device_spec.isp_transform {
            Some(transform) => {
                for (i, (dst, src)) in xfer_buf[2..length].iter_mut().zip(buf).enumerate() {
                    *dst = (transform.write)(i, *src);
                }
            }
            None => xfer_buf[2..length].clone_from_slice(buf),
        }
        self.xfer_device()
            .send_feature_report(&xfer_buf)
            .await
            .map_err(ISPError::from)?;
        if xfer_buf[1] != XFER_WRITE_PAGE {
            return Err(ISPError::ReadWriteMismatch);
        }
        Ok(())
    }

    /// Reads `length` bytes starting at `start_addr` by looping over pages.
    ///
    /// `progress` is invoked after each page with `(pages_done, pages_total)`;
    /// pass `&|_, _| {}` if you do not need it. This is mechanical protocol with
    /// no delays, so it stays in the library; sequencing it into a full read
    /// cycle (and the surrounding settle delays) is the caller's job.
    pub async fn read(
        &self,
        start_addr: usize,
        length: usize,
        progress: &dyn Fn(usize, usize),
    ) -> Result<Vec<u8>, ISPError> {
        let page_size = self.device_spec.platform.page_size;
        let num_page = length / page_size;

        self.init_read(start_addr, length).await?;

        let mut result: Vec<u8> = vec![];
        for i in 0..num_page {
            self.read_page(&mut result).await?;
            progress(i + 1, num_page);
        }
        Ok(result)
    }

    /// Writes `num_pages` pages from `buffer`, starting at `start_addr`.
    ///
    /// `progress` is invoked after each page with `(pages_done, pages_total)`;
    /// pass `&|_, _| {}` if you do not need it.
    pub async fn write(
        &self,
        start_addr: usize,
        buffer: &[u8],
        progress: &dyn Fn(usize, usize),
    ) -> Result<(), ISPError> {
        let page_size = self.device_spec.platform.page_size;
        let num_page = self.device_spec.num_pages();

        self.init_write(start_addr, num_page * page_size).await?;

        for i in 0..num_page {
            self.write_page(&buffer[(i * page_size)..((i + 1) * page_size)])
                .await?;
            progress(i + 1, num_page);
        }
        Ok(())
    }

    /// Erases everything in flash, except the ISP bootloader section itself and initializes the
    /// reset vector to jump to ISP.
    ///
    /// The device needs time to settle afterwards; the caller is responsible for
    /// the delay before issuing further commands.
    pub async fn erase(&self) -> Result<(), ISPError> {
        let cmd: [u8; COMMAND_LENGTH] = [REPORT_ID_CMD, CMD_ERASE, 0, 0, 0, 0];
        self.cmd_device
            .send_feature_report(&cmd)
            .await
            .map_err(ISPError::from)?;
        Ok(())
    }

    /// Causes the device to start running the main firmware.
    ///
    /// This drops the device off the bus, so the write often fails with a
    /// disconnect-class error even on success (see [`crate::is_expected_error`]);
    /// the caller decides how to treat the result and how long to wait.
    pub async fn reboot(&self) -> Result<(), ISPError> {
        let cmd: [u8; COMMAND_LENGTH] = [REPORT_ID_CMD, CMD_REBOOT, 0, 0, 0, 0];
        self.cmd_device
            .send_feature_report(&cmd)
            .await
            .map_err(ISPError::from)?;
        Ok(())
    }
}
