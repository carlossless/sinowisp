use core::panic;
use std::{future::Future, str::FromStr};

use hidra::{HidDevice, HidError};
#[cfg(not(target_arch = "wasm32"))]
use hidra::{NativeDevice, NusbDevice};
use thiserror::Error;

use crate::{device_spec::*, VerificationError};

const COMMAND_LENGTH: usize = 6;

pub(crate) const REPORT_ID_CMD: u8 = 0x05;
pub(crate) const REPORT_ID_XFER: u8 = 0x06;

pub(crate) const CMD_ENABLE_FIRMWARE: u8 = 0x55;
pub(crate) const CMD_INIT_READ: u8 = 0x52;
pub(crate) const CMD_INIT_WRITE: u8 = 0x57;
pub(crate) const CMD_ERASE: u8 = 0x45;
pub(crate) const CMD_REBOOT: u8 = 0x5a;

pub(crate) const XFER_READ_PAGE: u8 = 0x72;
pub(crate) const XFER_WRITE_PAGE: u8 = 0x77;

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

/// The HID feature-report calls the ISP protocol makes.
///
/// [`ISPHandle`] implements it for real devices. Implement it yourself to run
/// [`ISPDevice`] over something else, such as the in-memory bootloader in
/// `sinowisp::testing` (behind the `testing` feature).
pub trait Transport {
    fn send_feature_report(&self, data: &[u8]) -> impl Future<Output = Result<(), HidError>>;
    fn get_feature_report(&self, buf: &mut [u8]) -> impl Future<Output = Result<usize, HidError>>;
}

impl Transport for ISPHandle {
    fn send_feature_report(&self, data: &[u8]) -> impl Future<Output = Result<(), HidError>> {
        ISPHandle::send_feature_report(self, data)
    }

    fn get_feature_report(&self, buf: &mut [u8]) -> impl Future<Output = Result<usize, HidError>> {
        ISPHandle::get_feature_report(self, buf)
    }
}

/// One open connection to a device in ISP bootloader mode.
///
/// The methods are the individual protocol operations; they perform no
/// sequencing, delays, or progress reporting. Callers compose them into full
/// read/write cycles (and insert the settle delays after [`erase`](Self::erase)
/// and [`reboot`](Self::reboot)).
pub struct ISPDevice<T: Transport = ISPHandle> {
    cmd_device: T,
    /// Some platforms (Windows) expose the transfer report on a separate HID
    /// handle; everywhere else it is the same handle as `cmd_device`.
    xfer_device: Option<T>,
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
        Self::with_transport(device_spec, cmd_device.into(), xfer_device)
    }
}

impl<T: Transport> ISPDevice<T> {
    /// Like [`ISPDevice::new`], over any [`Transport`].
    pub fn with_transport(device_spec: DeviceSpec, cmd_device: T, xfer_device: Option<T>) -> Self {
        Self {
            cmd_device,
            xfer_device,
            device_spec,
        }
    }

    /// The spec this device was opened with (firmware/page sizes, reboot flag).
    pub fn device_spec(&self) -> &DeviceSpec {
        &self.device_spec
    }

    fn xfer_device(&self) -> &T {
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

    /// Initializes the read operation / sets the initial read address
    pub async fn init_read(&self, start_addr: usize) -> Result<(), ISPError> {
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
    pub async fn init_write(&self, start_addr: usize) -> Result<(), ISPError> {
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

        self.init_read(start_addr).await?;

        let mut result: Vec<u8> = Vec::with_capacity(num_page * page_size);
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

        self.init_write(start_addr).await?;

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

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use hidra::MaybeFuture;

    use super::*;
    use crate::testing::FakeBootloader;

    const SPEC: DeviceSpec = DEVICE_BASE_SH68F90;
    const PAGE: usize = SPEC.platform.page_size;

    fn add_one(_offset: usize, byte: u8) -> u8 {
        byte.wrapping_add(1)
    }

    fn xor_offset(offset: usize, byte: u8) -> u8 {
        byte ^ offset as u8
    }

    const TRANSFORMED: DeviceSpec = DeviceSpec {
        isp_transform: Some(IspTransform {
            read: xor_offset,
            write: add_one,
        }),
        ..SPEC
    };

    fn pattern(len: usize) -> Vec<u8> {
        (0..len).map(|i| (i * 7 + i / 256) as u8).collect()
    }

    #[test]
    fn test_commands() {
        let fake = FakeBootloader::new(SPEC);
        let device = ISPDevice::with_transport(SPEC, &fake, None);

        device.enable_firmware().wait().unwrap();
        device.init_read(0x1234).wait().unwrap();
        device.init_write(0xf000).wait().unwrap();
        device.erase().wait().unwrap();
        device.reboot().wait().unwrap();

        assert_eq!(
            fake.sent(),
            vec![
                vec![0x05, 0x55, 0x00, 0x00, 0x00, 0x00],
                vec![0x05, 0x52, 0x34, 0x12, 0x00, 0x00],
                vec![0x05, 0x57, 0x00, 0xf0, 0x00, 0x00],
                vec![0x05, 0x45, 0x00, 0x00, 0x00, 0x00],
                vec![0x05, 0x5a, 0x00, 0x00, 0x00, 0x00],
            ]
        );
    }

    #[test]
    fn test_read() {
        let flash = pattern(SPEC.total_flash_size());
        let fake = FakeBootloader::with_flash(SPEC, flash.clone());
        let device = ISPDevice::with_transport(SPEC, &fake, None);
        let progress = RefCell::new(vec![]);

        let result = device
            .read(0x800, 3 * PAGE, &|done, total| {
                progress.borrow_mut().push((done, total))
            })
            .wait()
            .unwrap();

        assert_eq!(result, flash[0x800..0x800 + 3 * PAGE]);
        assert_eq!(*progress.borrow(), vec![(1, 3), (2, 3), (3, 3)]);
        assert_eq!(fake.sent(), vec![vec![0x05, 0x52, 0x00, 0x08, 0x00, 0x00]]);
    }

    #[test]
    fn test_read_page_rejects_wrong_transfer_type() {
        let fake = FakeBootloader::new(SPEC);
        fake.set_read_type(XFER_WRITE_PAGE);
        let device = ISPDevice::with_transport(SPEC, &fake, None);

        let result = device.read_page(&mut vec![]).wait();

        assert!(matches!(result, Err(ISPError::ReadWriteMismatch)));
    }

    #[test]
    fn test_read_page_applies_isp_transform() {
        let flash = pattern(SPEC.total_flash_size());
        let fake = FakeBootloader::with_flash(SPEC, flash.clone());
        let device = ISPDevice::with_transport(TRANSFORMED, &fake, None);

        device.init_read(0x800).wait().unwrap();
        let mut page = vec![];
        device.read_page(&mut page).wait().unwrap();

        let expected: Vec<u8> = flash[0x800..0x800 + PAGE]
            .iter()
            .enumerate()
            .map(|(i, b)| xor_offset(i, *b))
            .collect();
        assert_eq!(page, expected);
    }

    #[test]
    fn test_write() {
        let firmware = pattern(SPEC.platform.firmware_size);
        let fake = FakeBootloader::new(SPEC);
        let device = ISPDevice::with_transport(SPEC, &fake, None);
        let progress = RefCell::new(vec![]);

        device
            .write(0, &firmware, &|done, _total| {
                progress.borrow_mut().push(done)
            })
            .wait()
            .unwrap();

        let sent = fake.sent();
        assert_eq!(sent[0], vec![0x05, 0x57, 0x00, 0x00, 0x00, 0x00]);
        assert_eq!(sent.len(), 1 + SPEC.num_pages());
        for (i, report) in sent[1..].iter().enumerate() {
            assert_eq!(report[..2], [0x06, 0x77]);
            assert_eq!(report[2..], firmware[i * PAGE..(i + 1) * PAGE]);
        }
        assert_eq!(
            *progress.borrow(),
            (1..=SPEC.num_pages()).collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_write_page_applies_isp_transform() {
        let fake = FakeBootloader::new(SPEC);
        let device = ISPDevice::with_transport(TRANSFORMED, &fake, None);

        device.write_page(&[0x00, 0x7f, 0xff]).wait().unwrap();

        assert_eq!(fake.sent(), vec![vec![0x06, 0x77, 0x01, 0x80, 0x00]]);
    }

    #[test]
    fn test_transfers_use_xfer_handle() {
        let cmd = FakeBootloader::new(SPEC);
        let xfer = FakeBootloader::with_flash(SPEC, vec![0xaa; SPEC.total_flash_size()]);
        let device = ISPDevice::with_transport(SPEC, &cmd, Some(&xfer));

        device.init_read(0x800).wait().unwrap();
        let mut page = vec![];
        device.read_page(&mut page).wait().unwrap();
        device.write_page(&[1, 2, 3]).wait().unwrap();

        assert!(page[1..].iter().all(|b| *b == 0xaa));
        assert_eq!(cmd.sent(), vec![vec![0x05, 0x52, 0x00, 0x08, 0x00, 0x00]]);
        assert_eq!(xfer.sent(), vec![vec![0x06, 0x77, 1, 2, 3]]);
    }

    #[test]
    fn test_read_section_round_trip() {
        for name in ReadSection::available_sections() {
            assert_eq!(ReadSection::from_str(name).unwrap().to_str(), name);
        }
    }
}
