use std::{thread, time};

use log::{debug, info};
use thiserror::Error;

use super::{part::*, util};
use crate::VerificationError;

extern crate hidapi;

use hidapi::{HidApi, HidDevice, HidError};

const MAX_RETRIES: usize = 10;

const GAMING_KB_VENDOR_ID: u16 = 0x0603;
const GAMING_KB_PRODUCT_ID: u16 = 0x1020;
const GAMING_KB_V2_PRODUCT_ID: u16 = 0x1021;

const COMMAND_LENGTH: usize = 6;

#[cfg(not(target_os = "linux"))]
const HID_ISP_USAGE_PAGE: u16 = 0xff00;
#[cfg(not(target_os = "linux"))]
const HID_ISP_USAGE: u16 = 0x0001;

const REPORT_ID_CMD: u8 = 0x05;
const REPORT_ID_XFER: u8 = 0x06;

const CMD_ISP_MODE: u8 = 0x75;
const CMD_ENABLE_FIRMWARE: u8 = 0x55;
const CMD_INIT_READ: u8 = 0x52;
const CMD_INIT_WRITE: u8 = 0x57;
const CMD_ERASE: u8 = 0x45;
const CMD_REBOOT: u8 = 0x5a;

const XFER_READ_PAGE: u8 = 0x72;
const XFER_WRITE_PAGE: u8 = 0x77;

pub struct ISPDevice {
    request_device: HidDevice,
    #[cfg(target_os = "windows")]
    data_device: HidDevice,
    part: Part,
}

#[derive(Debug, Error)]
pub enum ISPError {
    #[error("Unusual number of matching HID devices: {0}")]
    IrregularDeviceCount(usize),
    #[error("Device not found")]
    NotFound,
    #[error(transparent)]
    HidError(#[from] HidError),
    #[error(transparent)]
    VerificationError(#[from] VerificationError),
}

#[derive(Debug, Clone)]
pub enum ReadType {
    Normal,
    Bootloader,
    Full,
}

struct HIDDevices {
    request: HidDevice,
    #[cfg(target_os = "windows")]
    data: HidDevice,
}

impl ISPDevice {
    pub fn new(part: Part) -> Result<Self, ISPError> {
        let devices = Self::find_isp_device(part)?;
        Ok(Self {
            request_device: devices.request,
            #[cfg(target_os = "windows")]
            data_device: devices.data,
            part,
        })
    }

    /// Prints out all connected HID devices and their paths.
    pub fn print_connected_devices() -> Result<(), ISPError> {
        let api = ISPDevice::hidapi();

        info!("Listing all connected HID devices...");
        let mut devices: Vec<_> = api.device_list().collect();

        devices.sort_by_key(|d| d.path());

        for d in &devices {
            #[cfg(not(target_os = "linux"))]
            info!(
                "{:}: ID {:04x}:{:04x} manufacturer=\"{:}\" product=\"{:}\" usage_page={:#06x} usage={:#06x}",
                d.path().to_str().unwrap(),
                d.vendor_id(),
                d.product_id(),
                d.manufacturer_string().unwrap_or("None"),
                d.product_string().unwrap_or("None"),
                d.usage_page(),
                d.usage()
            );
            #[cfg(target_os = "linux")]
            info!(
                "{:}: ID {:04x}:{:04x} manufacturer=\"{:}\" product=\"{:}\"",
                d.path().to_str().unwrap(),
                d.vendor_id(),
                d.product_id(),
                d.manufacturer_string().unwrap_or("None"),
                d.product_string().unwrap_or("None")
            );
        }
        info!("Found {} devices", devices.len());

        Ok(())
    }

    fn hidapi() -> HidApi {
        let api = HidApi::new().unwrap();

        #[cfg(target_os = "macos")]
        api.set_open_exclusive(false); // macOS will throw a privilege violation error otherwise

        api
    }

    fn open_isp_devices() -> Result<HIDDevices, ISPError> {
        let api = Self::hidapi();

        let mut devices: Vec<_> = api
            .device_list()
            .filter(|d| {
                #[cfg(not(target_os = "linux"))]
                return d.vendor_id() == GAMING_KB_VENDOR_ID
                    && matches!(
                        d.product_id(),
                        GAMING_KB_PRODUCT_ID | GAMING_KB_V2_PRODUCT_ID
                    )
                    && d.interface_number() == 0
                    && d.usage_page() == HID_ISP_USAGE_PAGE
                    && d.usage() == HID_ISP_USAGE;
                #[cfg(target_os = "linux")]
                return d.vendor_id() == GAMING_KB_VENDOR_ID
                    && matches!(
                        d.product_id(),
                        GAMING_KB_PRODUCT_ID | GAMING_KB_V2_PRODUCT_ID
                    )
                    && d.interface_number() == 0;
            })
            .collect();

        devices.sort_by_key(|d| d.path());

        for d in &devices {
            #[cfg(not(target_os = "linux"))]
            debug!(
                "Found ISP Device: {:#06x} {:#06x} {:?} {:#06x} {:#06x}",
                d.vendor_id(),
                d.product_id(),
                d.path(),
                d.usage_page(),
                d.usage()
            );
            #[cfg(target_os = "linux")]
            debug!(
                "Found ISP Device: {:#06x} {:#06x} {:?}",
                d.vendor_id(),
                d.product_id(),
                d.path()
            );
        }

        let device_count = devices.len();
        if device_count == 0 {
            return Err(ISPError::NotFound);
        }

        #[cfg(not(target_os = "windows"))]
        if device_count == 1 {
            let request_device = devices.first().unwrap();
            debug!("Request device: {:?}", request_device.path());
            return Ok(HIDDevices {
                request: api.open_path(request_device.path()).unwrap(),
            });
        } else {
            return Err(ISPError::IrregularDeviceCount(device_count));
        }

        #[cfg(target_os = "windows")]
        if device_count == 1 {
            return Err(ISPError::IrregularDeviceCount(device_count));
        } else if device_count == 2 {
            let request_device = devices[0];
            let data_device = devices[1];
            debug!("Request device: {:?}", request_device.path());
            debug!("Data device: {:?}", data_device.path());
            return Ok(HIDDevices {
                request: api.open_path(request_device.path()).unwrap(),
                data: api.open_path(data_device.path()).unwrap(),
            });
        } else {
            return Err(ISPError::IrregularDeviceCount(device_count));
        }
    }

    fn switch_kb_device(part: Part) -> Result<HIDDevices, ISPError> {
        let api = Self::hidapi();

        info!(
            "Looking for vId:{:#06x} pId:{:#06x}",
            part.vendor_id, part.product_id
        );

        let request_device_info = api
            .device_list()
            .filter(|d| {
                #[cfg(not(target_os = "linux"))]
                return d.vendor_id() == part.vendor_id
                    && d.product_id() == part.product_id
                    && d.interface_number() == part.isp_iface_num as i32
                    && d.usage_page() == part.isp_usage_page
                    && d.usage() == part.isp_usage;
                #[cfg(target_os = "linux")]
                return d.vendor_id() == part.vendor_id
                    && d.product_id() == part.product_id
                    && d.interface_number() == part.isp_iface_num as i32;
            })
            .enumerate()
            .find_map(|(_i, d)| {
                #[cfg(not(target_os = "linux"))]
                debug!(
                    "Found Device: {:?} {:#06x} {:#06x}",
                    d.path(),
                    d.usage_page(),
                    d.usage()
                );
                #[cfg(target_os = "linux")]
                debug!("Found Device: {:?}", d.path());
                #[cfg(target_os = "windows")]
                if _i == part.isp_index {
                    return Some(d);
                } else {
                    return None;
                }
                #[cfg(not(target_os = "windows"))]
                Some(d)
            });

        let Some(request_device_info) = request_device_info else {
            info!("Regular device didn't come up...");
            return Err(ISPError::NotFound);
        };

        debug!("Opening: {:?}", request_device_info.path());

        let device = api.open_path(request_device_info.path()).unwrap();

        info!("Found regular device. Entering ISP mode...");
        if let Err(err) = Self::enter_isp_mode(&device) {
            debug!("Error: {:}", err);
            return Err(err);
        }

        info!("Waiting for ISP device...");
        thread::sleep(time::Duration::from_secs(2));

        let Ok(isp_device) = Self::open_isp_devices() else {
            info!("ISP device didn't come up...");
            return Err(ISPError::NotFound);
        };

        Ok(isp_device)
    }

    fn find_isp_device(part: Part) -> Result<HIDDevices, ISPError> {
        Self::find_isp_device_retry(part, MAX_RETRIES)
    }

    fn find_isp_device_retry(part: Part, retries: usize) -> Result<HIDDevices, ISPError> {
        for attempt in 1..retries + 1 {
            if attempt > 1 {
                thread::sleep(time::Duration::from_millis(500));
                info!("Retrying... Attempt {}/{}", attempt, retries);
            }

            if let Ok(devices) = Self::switch_kb_device(part) {
                info!("Connected!");
                return Ok(devices);
            }
            info!("Regular device not found. Trying ISP device...");
            if let Ok(devices) = Self::open_isp_devices() {
                info!("Connected!");
                return Ok(devices);
            }
        }
        Err(ISPError::NotFound)
    }

    fn enter_isp_mode(handle: &HidDevice) -> Result<(), ISPError> {
        let cmd: [u8; COMMAND_LENGTH] = [REPORT_ID_CMD, CMD_ISP_MODE, 0x00, 0x00, 0x00, 0x00];
        handle.send_feature_report(&cmd)?;
        Ok(())
    }

    pub fn read_cycle(&self, read_type: ReadType) -> Result<Vec<u8>, ISPError> {
        self.enable_firmware()?;

        let firmware = match read_type {
            ReadType::Normal => self.read(0, self.part.firmware_size)?,
            ReadType::Bootloader => {
                self.read(self.part.firmware_size, self.part.bootloader_size)?
            }
            ReadType::Full => self.read(0, self.part.firmware_size + self.part.bootloader_size)?,
        };

        if self.part.reboot {
            self.reboot()?;
        }

        return Ok(firmware);
    }

    pub fn write_cycle(&self, firmware: &mut Vec<u8>) -> Result<(), ISPError> {
        // ensure that the address at <firmware_size-4> is the same as the reset vector
        firmware.copy_within(1..3, self.part.firmware_size - 4);

        self.erase()?;
        self.write(0, firmware)?;

        // cleanup the address at <firmware_size-4>
        firmware[self.part.firmware_size - 4..self.part.firmware_size - 2].fill(0);

        let read_back = self.read(0, self.part.firmware_size)?;

        info!("Verifying...");
        util::verify(firmware, &read_back).map_err(ISPError::from)?;

        self.enable_firmware()?;

        if self.part.reboot {
            self.reboot()?;
        }

        Ok(())
    }

    fn data_device(&self) -> &HidDevice {
        #[cfg(target_os = "windows")]
        return &self.data_device;
        #[cfg(not(target_os = "windows"))]
        &self.request_device
    }

    fn read(&self, start_addr: usize, length: usize) -> Result<Vec<u8>, ISPError> {
        info!("Reading...");
        self.init_read(start_addr)?;

        let page_size = self.part.page_size;
        let num_page = length / page_size;
        let mut result: Vec<u8> = vec![];
        for i in 0..num_page {
            debug!(
                "Reading page {} @ offset {:#06x}",
                i,
                start_addr + i * page_size
            );
            self.read_page(&mut result)?;
        }
        Ok(result)
    }

    fn write(&self, start_addr: usize, buffer: &[u8]) -> Result<(), ISPError> {
        info!("Writing...");
        self.init_write(start_addr)?;

        let page_size = self.part.page_size;
        for i in 0..self.part.num_pages() {
            debug!("Writing page {} @ offset {:#06x}", i, i * page_size);
            self.write_page(&buffer[(i * page_size)..((i + 1) * page_size)])?;
        }
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
        self.request_device
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
        self.request_device
            .send_feature_report(&cmd)
            .map_err(ISPError::from)?;
        Ok(())
    }

    /// Reads one page of flash contents
    fn read_page(&self, buf: &mut Vec<u8>) -> Result<(), ISPError> {
        let page_size = self.part.page_size;
        let mut xfer_buf: Vec<u8> = vec![0; page_size + 2];
        xfer_buf[0] = REPORT_ID_XFER;
        xfer_buf[1] = XFER_READ_PAGE;
        self.data_device()
            .get_feature_report(&mut xfer_buf)
            .map_err(ISPError::from)?;
        buf.extend_from_slice(&xfer_buf[2..(page_size + 2)]);
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
        self.data_device()
            .send_feature_report(&xfer_buf)
            .map_err(ISPError::from)?;
        Ok(())
    }

    /// Sets a LJMP (0x02) opcode at <firmware_size-5>.
    /// This enables the main firmware by making the bootloader jump to it on reset.
    ///
    /// Side-effect: enables reading the firmware without erasing flash first.
    /// Credits to @gashtaan for finding this out.
    fn enable_firmware(&self) -> Result<(), ISPError> {
        info!("Enabling firmware...");
        let cmd: [u8; COMMAND_LENGTH] = [REPORT_ID_CMD, CMD_ENABLE_FIRMWARE, 0, 0, 0, 0];

        self.request_device.send_feature_report(&cmd)?;
        Ok(())
    }

    /// Erases everything in flash, except the ISP bootloader section itself and initializes the
    /// reset vector to jump to ISP.
    fn erase(&self) -> Result<(), ISPError> {
        info!("Erasing...");
        let cmd: [u8; COMMAND_LENGTH] = [REPORT_ID_CMD, CMD_ERASE, 0, 0, 0, 0];
        self.request_device
            .send_feature_report(&cmd)
            .map_err(ISPError::from)?;
        thread::sleep(time::Duration::from_millis(2000));
        Ok(())
    }

    /// Causes the device to start running the main firmware
    fn reboot(&self) -> Result<(), ISPError> {
        info!("Rebooting...");
        let cmd: [u8; COMMAND_LENGTH] = [REPORT_ID_CMD, CMD_REBOOT, 0, 0, 0, 0];
        if let Err(err) = self.request_device.send_feature_report(&cmd) {
            // only log failures
            debug!("Error: {:}", err);
        }
        thread::sleep(time::Duration::from_millis(2000));
        Ok(())
    }
}
