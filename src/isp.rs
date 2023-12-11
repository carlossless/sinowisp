use std::{thread, time};

use hidapi::DeviceInfo;
use log::{debug, info, warn};
use thiserror::Error;

use super::{part::*, util};
use crate::VerificationError;

extern crate hidapi;

use hidapi::{HidApi, HidDevice, HidError};

const MAX_RETRIES: usize = 10;

const GAMING_KB_VENDOR_ID: u16 = 0x0603;
const GAMING_KB_PRODUCT_ID: u16 = 0x1020;

const COMMAND_LENGTH: usize = 6;

const REPORT_ID_CMD: u8 = 0x05;
const REPORT_ID_XFER: u8 = 0x06;

const CMD_ISP_MODE: u8 = 0x75;
const CMD_ENABLE_FIRMWARE: u8 = 0x55;
const CMD_INIT_READ: u8 = 0x52;
const CMD_INIT_WRITE: u8 = 0x57;
const CMD_ERASE: u8 = 0x45;

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
    #[error("Duplicate devices found")]
    DuplicateDevices(String, String),
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

    fn hidapi() -> HidApi {
        let api = HidApi::new().unwrap();

        #[cfg(target_os = "macos")]
        api.set_open_exclusive(false); // macOS will throw a privilege violation error otherwise

        api
    }

    fn open_isp_devices() -> Result<HIDDevices, ISPError> {
        let api = Self::hidapi();

        let mut request_device: Option<&DeviceInfo> = None;
        #[cfg(target_os = "windows")]
        let mut data_device: Option<&DeviceInfo> = None;

        for device_info in api.device_list() {
            if !(device_info.vendor_id() == GAMING_KB_VENDOR_ID
                && device_info.product_id() == GAMING_KB_PRODUCT_ID)
            {
                continue;
            }

            let path = device_info.path();
            let path_str = path.to_str().unwrap();

            debug!("Enumerating: {}", path_str);

            #[cfg(target_os = "windows")]
            {
                // Windows requires that we use specific devices for requests and data
                // https://learn.microsoft.com/en-us/windows-hardware/drivers/hid/hidclass-hardware-ids-for-top-level-collections
                if path_str.contains("Col02") {
                    if let Some(request_device) = request_device {
                        return Err(ISPError::DuplicateDevices(
                            request_device.path().to_str().unwrap().to_owned(),
                            path_str.to_owned(),
                        ));
                    }
                    request_device = Some(device_info);
                    continue;
                }

                if path_str.contains("Col03") {
                    if let Some(data_device) = data_device {
                        return Err(ISPError::DuplicateDevices(
                            data_device.path().to_str().unwrap().to_owned(),
                            path_str.to_owned(),
                        ));
                    }
                    data_device = Some(device_info);
                    continue;
                }
            };

            #[cfg(not(target_os = "windows"))]
            if let Some(request_device) = request_device {
                if request_device.path() != path {
                    warn!("Duplicate device found. Only the first one will be used");
                }
                continue;
            } else {
                request_device = Some(device_info);
                continue;
            };
        }

        if let Some(request_device) = request_device {
            debug!("Request device: {:?}", request_device.path());
            #[cfg(target_os = "windows")]
            if let Some(data_device) = data_device {
                debug!("Data device: {:?}", data_device.path());
                return Ok(HIDDevices {
                    request: api.open_path(request_device.path()).unwrap(),
                    data: api.open_path(data_device.path()).unwrap(),
                });
            } else {
                return Err(ISPError::NotFound);
            }

            #[cfg(not(target_os = "windows"))]
            return Ok(HIDDevices {
                request: api.open_path(request_device.path()).unwrap(),
            });
        } else {
            Err(ISPError::NotFound)
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
                d.vendor_id() == part.vendor_id
                    && d.product_id() == part.product_id
                    && d.interface_number() == 1
            })
            .find(|_d| {
                #[cfg(target_os = "windows")]
                {
                    return String::from_utf8_lossy(_d.path().to_bytes())
                        .to_string()
                        .contains("Col05");
                }
                #[cfg(not(target_os = "windows"))]
                true
            });

        let Some(request_device_info) = request_device_info else {
            info!("Regular device didn't come up...");
            return Err(ISPError::NotFound);
        };

        debug!("Opening: {:?}", request_device_info.path());

        let device = api.open_path(request_device_info.path()).unwrap();

        info!("Found regular device. Entering ISP mode...");
        Self::enter_isp_mode(&device)?;

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
        let cmd: [u8; COMMAND_LENGTH] =
            [REPORT_ID_CMD, CMD_ENABLE_FIRMWARE, 0, 0, 0, 0];

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
}
