use core::time;
use std::{ffi::CStr, thread};

use hidapi::{BusType, DeviceInfo, HidDevice, HidError, MAX_REPORT_DESCRIPTOR_SIZE};
use hidparser::parse_report_descriptor;
use itertools::Itertools;
use log::{debug, error, info};
use thiserror::Error;

use crate::{
    hid_tree::{DeviceNode, InterfaceNode, ItemNode},
    is_expected_error, ISPDevice, Part,
};

const REPORT_ID_ISP: u8 = 0x05;
const CMD_ISP_MODE: u8 = 0x75;

#[cfg(target_os = "windows")]
const REPORT_ID_XFER: u8 = 0x06;

const GAMING_KB_VENDOR_ID: u16 = 0x0603;
const GAMING_KB_PRODUCT_ID: u16 = 0x1020;
const GAMING_KB_V2_PRODUCT_ID: u16 = 0x1021;
const GAMING_KB_IFACE: i32 = 0;

const COMMAND_LENGTH: usize = 6;

#[derive(Debug, Error)]
pub enum DeviceSelectorError {
    #[error("Device not found")]
    NotFound,
    #[error(transparent)]
    HidError(#[from] HidError),
    #[error("Failed to parse report descriptor {0:?}")]
    ReportDescriptorError(hidparser::report_descriptor_parser::ReportDescriptorError),
}

pub struct DeviceSelector {
    api: hidapi::HidApi,
}

impl DeviceSelector {
    pub fn new() -> Result<Self, DeviceSelectorError> {
        let api = hidapi::HidApi::new().map_err(DeviceSelectorError::from)?;

        #[cfg(target_os = "macos")]
        api.set_open_exclusive(false); // macOS will throw a privilege violation error otherwise

        Ok(Self { api })
    }

    fn sorted_usb_device_list(&self) -> Vec<&DeviceInfo> {
        let mut devices: Vec<_> = self
            .api
            .device_list()
            .filter(|d| d.bus_type() as u32 == BusType::Usb as u32)
            .collect();
        // TODO: move out the platform specific sorting
        devices.sort_by_key(|d| {
            #[cfg(not(target_os = "linux"))]
            return (
                d.vendor_id(),
                d.product_id(),
                d.path(),
                d.interface_number(),
                d.usage_page(),
                d.usage(),
            );
            #[cfg(target_os = "linux")]
            return (
                d.vendor_id(),
                d.product_id(),
                d.path(),
                d.interface_number(),
            );
        });
        devices
    }

    fn get_feature_report_ids_from_path(
        &self,
        path: &CStr,
    ) -> Result<Vec<u32>, DeviceSelectorError> {
        let dev = self
            .api
            .open_path(path)
            .map_err(DeviceSelectorError::from)?;
        self.get_feature_report_ids_from_device(&dev)
    }

    fn get_feature_report_ids_from_device(
        &self,
        dev: &HidDevice,
    ) -> Result<Vec<u32>, DeviceSelectorError> {
        let mut buf: [u8; MAX_REPORT_DESCRIPTOR_SIZE] = [0; MAX_REPORT_DESCRIPTOR_SIZE];
        let size: usize = dev
            .get_report_descriptor(&mut buf)
            .map_err(DeviceSelectorError::from)?;
        let descriptor = buf[..size].to_vec();
        self.get_feature_report_ids_from_descriptor(&descriptor)
    }

    fn get_feature_report_ids_from_descriptor(
        &self,
        descriptor: &[u8],
    ) -> Result<Vec<u32>, DeviceSelectorError> {
        let report_descriptor = parse_report_descriptor(descriptor)
            .map_err(DeviceSelectorError::ReportDescriptorError)?;
        let res = report_descriptor
            .features
            .iter()
            .filter_map(|item| item.report_id)
            .map(|report_id| report_id.into())
            .collect();
        Ok(res)
    }

    fn get_report_descriptor(&self, dev: &HidDevice) -> Result<Vec<u8>, DeviceSelectorError> {
        let mut buf: [u8; MAX_REPORT_DESCRIPTOR_SIZE] = [0; MAX_REPORT_DESCRIPTOR_SIZE];
        let size: usize = dev
            .get_report_descriptor(&mut buf)
            .map_err(DeviceSelectorError::from)?;
        Ok(buf[..size].to_vec())
    }

    fn get_descriptor_with_features(
        &self,
        path: &CStr,
    ) -> (
        Result<Vec<u8>, DeviceSelectorError>,
        Result<Vec<u32>, DeviceSelectorError>,
    ) {
        let descriptor: Result<Vec<u8>, DeviceSelectorError>;
        let feature_report_ids: Result<Vec<u32>, DeviceSelectorError>;
        match self.api.open_path(path) {
            Ok(ref dev) => {
                descriptor = self.get_report_descriptor(dev);
                match descriptor {
                    Ok(ref report) => {
                        feature_report_ids = self.get_feature_report_ids_from_descriptor(report);
                    }
                    Err(_) => {
                        feature_report_ids = Err(DeviceSelectorError::NotFound);
                    }
                }
            }
            Err(err) => {
                descriptor = Err(DeviceSelectorError::from(err));
                feature_report_ids = Err(DeviceSelectorError::NotFound);
            }
        }
        (descriptor, feature_report_ids)
    }

    fn get_device_for_report_id<'a, I: IntoIterator<Item = &'a DeviceInfo>>(
        &self,
        devices: I,
        report_id: u32,
    ) -> Result<&'a DeviceInfo, DeviceSelectorError> {
        for d in devices {
            let ids = self.get_feature_report_ids_from_path(d.path())?;
            for id in ids {
                if id == report_id {
                    return Ok(d);
                }
            }
        }
        Err(DeviceSelectorError::NotFound)
    }

    fn open_isp_devices(&self, part: Part) -> Result<ISPDevice, DeviceSelectorError> {
        let sorted_devices = self.sorted_usb_device_list();
        let isp_devices: Vec<_> = sorted_devices
            .clone()
            .into_iter()
            .filter(|d| {
                d.vendor_id() == GAMING_KB_VENDOR_ID
                    && matches!(
                        d.product_id(),
                        GAMING_KB_PRODUCT_ID | GAMING_KB_V2_PRODUCT_ID
                    )
                    && d.interface_number() == GAMING_KB_IFACE
            })
            .collect();

        let device_count = isp_devices.len();
        if device_count == 0 {
            return Err(DeviceSelectorError::NotFound);
        }

        let s = isp_devices.clone();
        // TODO: check for both feature report IDs in macOS and Linux and get that device
        // TODO: check for each feature report IDs in Windows and get each of those devices
        let cmd_device = self.get_device_for_report_id(s, REPORT_ID_ISP as u32)?;
        debug!("CMD device: {}", cmd_device.info());
        #[cfg(not(target_os = "windows"))]
        return Ok(ISPDevice::new(
            part,
            self.api.open_path(cmd_device.path()).unwrap(),
        ));

        #[cfg(target_os = "windows")]
        {
            let xfer_device =
                self.get_device_for_report_id(isp_devices.clone(), REPORT_ID_XFER as u32)?;
            debug!("XFER device: {}", xfer_device.info());
            return Ok(ISPDevice::new(
                part,
                self.api.open_path(cmd_device.path()).unwrap(),
                self.api.open_path(xfer_device.path()).unwrap(),
            ));
        }
    }

    fn switch_kb_device(&mut self, part: Part) -> Result<ISPDevice, DeviceSelectorError> {
        info!(
            "Looking for vId:{:#06x} pId:{:#06x}",
            part.vendor_id, part.product_id
        );

        let filtered_devices = self.sorted_usb_device_list().into_iter().filter(|d| {
            d.vendor_id() == part.vendor_id
                && d.product_id() == part.product_id
                && d.interface_number() == part.isp_iface_num
        });

        let mut cmd_device_info: Option<&DeviceInfo> = None;
        for d in filtered_devices {
            let ids = self
                .get_feature_report_ids_from_path(d.path())
                .map_err(|_| DeviceSelectorError::NotFound)?;
            for id in ids {
                if id == part.isp_report_id {
                    cmd_device_info = Some(d);
                }
            }
        }

        let Some(cmd_device_info) = cmd_device_info else {
            info!("Regular device didn't come up...");
            return Err(DeviceSelectorError::NotFound);
        };

        debug!("Opening: {:?}", cmd_device_info.path());
        let device = self
            .api
            .open_path(cmd_device_info.path())
            .map_err(DeviceSelectorError::from)?;

        info!("Found regular device. Entering ISP mode...");
        if let Err(err) = self.enter_isp_mode(&device) {
            debug!("Error: {:}", err);
            match err {
                DeviceSelectorError::HidError(err) if is_expected_error(&err) => {}
                _ => {
                    error!("Unexpected: {:}", err);
                    info!("Waiting...");
                    thread::sleep(time::Duration::from_secs(2));
                    return Err(err);
                }
            }
        }

        info!("Waiting for ISP device...");
        thread::sleep(time::Duration::from_secs(2));

        self.api.refresh_devices()?;

        let Ok(isp_device) = self.open_isp_devices(part) else {
            info!("ISP device didn't come up...");
            return Err(DeviceSelectorError::NotFound);
        };
        Ok(isp_device)
    }

    pub fn find_isp_device(
        &mut self,
        part: Part,
        retries: usize,
    ) -> Result<ISPDevice, DeviceSelectorError> {
        for attempt in 1..retries + 1 {
            self.api.refresh_devices()?;
            if attempt > 1 {
                thread::sleep(time::Duration::from_millis(500));
                info!("Retrying... Attempt {}/{}", attempt, retries);
            }

            if let Ok(devices) = self.switch_kb_device(part) {
                info!("Connected!");
                return Ok(devices);
            }
            info!("Regular device not found. Trying ISP device...");
            if let Ok(devices) = self.open_isp_devices(part) {
                info!("Connected!");
                return Ok(devices);
            }
        }
        Err(DeviceSelectorError::NotFound)
    }

    fn enter_isp_mode(&self, handle: &HidDevice) -> Result<(), DeviceSelectorError> {
        let cmd: [u8; COMMAND_LENGTH] = [REPORT_ID_ISP, CMD_ISP_MODE, 0x00, 0x00, 0x00, 0x00];
        handle.send_feature_report(&cmd)?;
        Ok(())
    }

    pub fn connected_devices_tree(&self) -> Result<Vec<DeviceNode>, DeviceSelectorError> {
        let devices: Vec<_> = self.sorted_usb_device_list();

        let id_chunks = devices.iter().chunk_by(|d| {
            return (
                d.vendor_id(),
                d.product_id(),
                d.manufacturer_string().unwrap_or("None"),
                d.product_string().unwrap_or("None"),
            );
        });

        let mut device_tree_devices: Vec<DeviceNode> = vec![];

        for ((vid, pid, manufacturer, product), devices) in &id_chunks {
            let mut node = DeviceNode {
                product_id: pid,
                vendor_id: vid,
                product_string: product.to_string(),
                manufacturer_string: manufacturer.to_string(),
                children: vec![],
            };

            let path_chunks = devices.chunk_by(|d| {
                #[cfg(any(target_os = "macos", target_os = "linux"))]
                return (d.path(), d.interface_number());
                #[cfg(target_os = "windows")]
                return (d.path(), d.interface_number(), d.usage_page(), d.usage());
            });

            for (key, devices) in &path_chunks {
                #[cfg(any(target_os = "macos", target_os = "linux"))]
                let (path, interface_number) = key;
                #[cfg(target_os = "windows")]
                let (path, interface_number, usage_page, usage) = key;

                let mut children: Vec<ItemNode> = vec![];

                for d in devices {
                    #[cfg(target_os = "macos")]
                    children.push(ItemNode {
                        usage_page: d.usage_page(),
                        usage: d.usage(),
                    });
                    #[cfg(target_os = "windows")]
                    {
                        let (descriptor, feature_report_ids) =
                            self.get_descriptor_with_features(path);
                        children.push(ItemNode {
                            path: path.to_str().unwrap().to_string(),
                            usage_page: d.usage_page(),
                            usage: d.usage(),
                            descriptor,
                            feature_report_ids,
                        });
                    }
                }

                let (descriptor, feature_report_ids) = self.get_descriptor_with_features(path);
                let interface_node = InterfaceNode {
                    #[cfg(any(target_os = "macos", target_os = "linux"))]
                    path: path.to_str().unwrap().to_string(),
                    interface_number,
                    #[cfg(any(target_os = "macos", target_os = "linux"))]
                    descriptor,
                    #[cfg(any(target_os = "macos", target_os = "linux"))]
                    feature_report_ids,
                    #[cfg(any(target_os = "macos", target_os = "windows"))]
                    children,
                };

                node.children.push(interface_node);
            }

            device_tree_devices.push(node);
        }
        Ok(device_tree_devices)
    }
}

trait PlatformSpecificInfo {
    fn info(&self) -> String;
}

impl PlatformSpecificInfo for DeviceInfo {
    fn info(&self) -> String {
        #[cfg(not(target_os = "linux"))]
        return format!(
            "Found ISP Device: {:#06x} {:#06x} {:?} {} {:#06x} {:#06x}",
            self.vendor_id(),
            self.product_id(),
            self.path(),
            self.interface_number(),
            self.usage_page(),
            self.usage()
        );
        #[cfg(target_os = "linux")]
        format!(
            "Found ISP Device: {:#06x} {:#06x} {:?}",
            self.vendor_id(),
            self.product_id(),
            self.path()
        )
    }
}
