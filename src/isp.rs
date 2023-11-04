use std::{thread, time};

use hidapi::DeviceInfo;
use log::*;
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
const CMD_MAGIC_SAUCE: u8 = 0x55; // uncertain how this command works, hence the name
const CMD_INIT_READ: u8 = 0x52;
const CMD_INIT_WRITE: u8 = 0x57;
const CMD_ERASE: u8 = 0x45;

const XFER_READ_PAGE: u8 = 0x72;
const XFER_WRITE_PAGE: u8 = 0x77;

const LJMP_OPCODE: u8 = 0x02;

pub struct ISPDevice<'a> {
    request_device: HidDevice,
    #[cfg(target_os = "windows")]
    data_device: HidDevice,
    part: &'a Part,
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

impl ISPDevice<'static> {
    pub fn new(part: &'static Part) -> Result<Self, ISPError> {
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

    fn switch_kb_device(part: &Part) -> Result<HIDDevices, ISPError> {
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

        info!("Found Regular device. Entering ISP mode...");
        Self::enter_isp_mode(&device)?;

        info!("Waiting for ISP device...");
        thread::sleep(time::Duration::from_secs(2));

        let Ok(isp_device) = Self::open_isp_devices() else {
            info!("ISP device didn't come up...");
            return Err(ISPError::NotFound);
        };

        Ok(isp_device)
    }

    fn find_isp_device(part: &Part) -> Result<HIDDevices, ISPError> {
        Self::find_isp_device_retry(part, MAX_RETRIES)
    }

    fn find_isp_device_retry(part: &Part, retries: usize) -> Result<HIDDevices, ISPError> {
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
        self.magic_sauce()?;

        match read_type {
            ReadType::Normal => self.read(0, self.part.flash_size),
            ReadType::Bootloader => self.read(self.part.flash_size, self.part.bootloader_size),
            ReadType::Full => self.read(0, self.part.flash_size + self.part.bootloader_size),
        }
    }

    pub fn write_cycle(&self, firmware: &mut Vec<u8>) -> Result<(), ISPError> {
        let length = firmware.len();

        self.erase()?;
        self.write(firmware)?;
        let written = self.read(0, self.part.flash_size)?;

        // ARCANE: the ISP will copy the LJMP instruction (if existing) from the end to the very start of memory.
        // We need to make modifications to the expected payload to account for this.
        if firmware[length - 5] == LJMP_OPCODE {
            firmware[0] = LJMP_OPCODE;
            firmware.copy_within((length - 4)..(length - 2), 1); // Copy LJMP address
            firmware[(length - 5)..(length - 2)].fill(0); // Cleanup
        }

        info!("Verifying...");
        util::verify(firmware, &written).map_err(ISPError::from)?;
        self.finalize()?;
        Ok(())
    }

    pub fn erase_cycle(&self) -> Result<(), ISPError> {
        info!("Erasing...");
        self.erase()?;
        self.finalize()?;
        Ok(())
    }

    fn data_device(&self) -> &HidDevice {
        #[cfg(target_os = "windows")]
        return &self.data_device;
        #[cfg(not(target_os = "windows"))]
        &self.request_device
    }

    /// Allows firmware to be read prior to erasing it
    fn magic_sauce(&self) -> Result<(), ISPError> {
        let cmd: [u8; COMMAND_LENGTH] = [
            REPORT_ID_CMD,
            CMD_MAGIC_SAUCE,
            0x00,
            0x00,
            (self.part.flash_size & 0xff) as u8,
            (self.part.flash_size >> 8) as u8,
        ];

        self.request_device.send_feature_report(&cmd)?;
        Ok(())
    }

    fn read(&self, start_addr: usize, length: usize) -> Result<Vec<u8>, ISPError> {
        let cmd: [u8; COMMAND_LENGTH] = [
            REPORT_ID_CMD,
            CMD_INIT_READ,
            (start_addr & 0xff) as u8,
            (start_addr >> 8) as u8,
            (length & 0xff) as u8,
            (length >> 8) as u8,
        ];
        self.request_device.send_feature_report(&cmd)?;

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

    fn write(&self, buffer: &[u8]) -> Result<(), ISPError> {
        info!("Writing...");
        let cmd: [u8; COMMAND_LENGTH] = [
            REPORT_ID_CMD,
            CMD_INIT_WRITE,
            0,
            0,
            (self.part.flash_size & 0xff) as u8,
            (self.part.flash_size >> 8) as u8,
        ];

        self.request_device
            .send_feature_report(&cmd)
            .map_err(ISPError::from)?;

        let page_size = self.part.page_size;
        for i in 0..self.part.num_pages() {
            debug!("Writting page {} @ offset {:#06x}", i, i * page_size);
            self.write_page(&buffer[(i * page_size)..((i + 1) * page_size)])?;
        }
        Ok(())
    }

    fn write_page(&self, buf: &[u8]) -> Result<(), ISPError> {
        let page_size = self.part.page_size;
        let mut xfer_buf: Vec<u8> = vec![0; page_size + 2];
        xfer_buf[0] = REPORT_ID_XFER;
        xfer_buf[1] = XFER_WRITE_PAGE;
        xfer_buf[2..page_size + 2].clone_from_slice(buf);
        self.data_device()
            .send_feature_report(&xfer_buf)
            .map_err(ISPError::from)?;
        Ok(())
    }

    fn erase(&self) -> Result<(), ISPError> {
        info!("Erasing...");
        let cmd: [u8; COMMAND_LENGTH] = [
            REPORT_ID_CMD,
            CMD_ERASE,
            CMD_ERASE,
            CMD_ERASE,
            CMD_ERASE,
            CMD_ERASE,
        ];
        self.request_device
            .send_feature_report(&cmd)
            .map_err(ISPError::from)?;
        thread::sleep(time::Duration::from_millis(2000));
        Ok(())
    }

    fn finalize(&self) -> Result<(), ISPError> {
        info!("Finalizing...");
        let cmd: [u8; COMMAND_LENGTH] = [
            REPORT_ID_CMD,
            CMD_MAGIC_SAUCE,
            CMD_MAGIC_SAUCE,
            CMD_MAGIC_SAUCE,
            CMD_MAGIC_SAUCE,
            CMD_MAGIC_SAUCE,
        ];
        self.request_device
            .send_feature_report(&cmd)
            .map_err(ISPError::from)?;
        Ok(())
    }
}
