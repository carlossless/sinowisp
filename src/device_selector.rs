use std::{thread, time::Duration};

use hidparser::parse_report_descriptor;
use hidra::{
    BusType, DeviceInfo, HidError, Hidra, MaybeFuture, Native, Nusb, MAX_REPORT_DESCRIPTOR_SIZE,
};
use indicatif::ProgressBar;
use itertools::Itertools;
use log::{debug, error, info};
use sinowisp::{is_expected_error, DeviceSpec, ISPDevice, ISPHandle, Transport};
use thiserror::Error;

use crate::hid_tree::{DeviceNode, InterfaceNode};

use crate::hid_tree::ItemNode;

const REPORT_ID_ISP: u8 = 0x05;
const CMD_ISP_MODE: u8 = 0x75;

const REPORT_ID_XFER: u8 = 0x06;

const GAMING_KB_VENDOR_ID: u16 = 0x0603;
const GAMING_KB_PRODUCT_ID: u16 = 0x1020;
const GAMING_KB_V2_PRODUCT_ID: u16 = 0x1021;
const GAMING_KB_IFACE: i32 = 0;

const COMMAND_LENGTH: usize = 6;

const ISP_SWITCH_DELAY: Duration = if cfg!(test) {
    Duration::ZERO
} else {
    Duration::from_secs(2)
};
const RETRY_DELAY: Duration = if cfg!(test) {
    Duration::ZERO
} else {
    Duration::from_secs(1)
};

#[derive(Debug, Error)]
pub enum DeviceSelectorError {
    #[error("Device not found")]
    NotFound,
    #[error(transparent)]
    HidError(#[from] HidError),
    #[error("Failed to parse report descriptor {0:?}")]
    ReportDescriptorError(hidparser::report_descriptor_parser::ReportDescriptorError),
    #[error("Unexpected device count")]
    UnexpectedDeviceCount,
}

pub trait HidInfo {
    fn path(&self) -> &str;
    fn vendor_id(&self) -> u16;
    fn product_id(&self) -> u16;
    fn interface_number(&self) -> i32;
    fn usage_page(&self) -> u16;
    fn usage(&self) -> u16;
    fn manufacturer_string(&self) -> Option<&str>;
    fn product_string(&self) -> Option<&str>;
    fn is_usb(&self) -> bool;

    fn sort_key(&self) -> impl Ord + '_ {
        (
            self.vendor_id(),
            self.product_id(),
            self.interface_number(),
            self.path(),
            self.usage_page(),
            self.usage(),
        )
    }

    fn info(&self) -> String {
        format!(
            "{:#06x} {:#06x} {:?} {} {:#06x} {:#06x}",
            self.vendor_id(),
            self.product_id(),
            self.path(),
            self.interface_number(),
            self.usage_page(),
            self.usage()
        )
    }
}

impl HidInfo for DeviceInfo {
    fn path(&self) -> &str {
        DeviceInfo::path(self)
    }
    fn vendor_id(&self) -> u16 {
        DeviceInfo::vendor_id(self)
    }
    fn product_id(&self) -> u16 {
        DeviceInfo::product_id(self)
    }
    fn interface_number(&self) -> i32 {
        DeviceInfo::interface_number(self)
    }
    fn usage_page(&self) -> u16 {
        DeviceInfo::usage_page(self)
    }
    fn usage(&self) -> u16 {
        DeviceInfo::usage(self)
    }
    fn manufacturer_string(&self) -> Option<&str> {
        DeviceInfo::manufacturer_string(self)
    }
    fn product_string(&self) -> Option<&str> {
        DeviceInfo::product_string(self)
    }
    fn is_usb(&self) -> bool {
        self.bus_type() == BusType::Usb
    }
}

pub trait HidBackend {
    type Info: HidInfo;
    type Handle: Transport;

    fn devices(&self) -> Vec<&Self::Info>;
    fn open(&self, path: &str) -> Result<Self::Handle, DeviceSelectorError>;
    fn report_descriptor(&self, handle: &Self::Handle, buf: &mut [u8]) -> Result<usize, HidError>;
    fn refresh(&mut self) -> Result<(), DeviceSelectorError>;
    fn next_backend(&mut self) -> Result<(), DeviceSelectorError>;
}

pub struct HidraBackend {
    api: Api,
    backend: usize,
}

impl HidraBackend {
    fn new() -> Result<Self, DeviceSelectorError> {
        Ok(Self {
            api: Api::open(BACKENDS[0])?,
            backend: 0,
        })
    }
}

impl HidBackend for HidraBackend {
    type Info = DeviceInfo;
    type Handle = ISPHandle;

    fn devices(&self) -> Vec<&DeviceInfo> {
        self.api.device_list()
    }

    fn open(&self, path: &str) -> Result<ISPHandle, DeviceSelectorError> {
        self.api.open_path(path)
    }

    fn report_descriptor(&self, handle: &ISPHandle, buf: &mut [u8]) -> Result<usize, HidError> {
        handle.get_report_descriptor(buf).wait()
    }

    fn refresh(&mut self) -> Result<(), DeviceSelectorError> {
        self.api.refresh_devices()
    }

    /// Next backend: each sees devices the other cannot, and ISP mode swaps which.
    fn next_backend(&mut self) -> Result<(), DeviceSelectorError> {
        self.backend = (self.backend + 1) % BACKENDS.len();
        let backend = BACKENDS[self.backend];
        info!("Trying the {backend} backend...");
        self.api = Api::open(backend)?;
        Ok(())
    }
}

/// The two hidra backends, so one field can hold either.
///
/// Backends are types, so this is the caller's to declare. It forwards the
/// three methods the selector uses; each sees devices the other cannot, and
/// ISP mode swaps which.
enum Api {
    Native(Hidra<Native>),
    Nusb(Hidra<Nusb>),
}

impl Api {
    fn open(which: Backend) -> Result<Self, DeviceSelectorError> {
        Ok(match which {
            Backend::Native => {
                let api = Hidra::<Native>::builder().build()?;
                // macOS refuses an exclusive open with a privilege violation.
                #[cfg(target_os = "macos")]
                api.set_open_exclusive(false);
                Api::Native(api)
            }
            Backend::Nusb => Api::Nusb(Hidra::<Nusb>::builder().build()?),
        })
    }

    fn refresh_devices(&mut self) -> Result<(), DeviceSelectorError> {
        match self {
            Api::Native(api) => api.refresh_devices()?,
            Api::Nusb(api) => api.refresh_devices()?,
        }
        Ok(())
    }

    fn device_list(&self) -> Vec<&DeviceInfo> {
        match self {
            Api::Native(api) => api.device_list().collect(),
            Api::Nusb(api) => api.device_list().collect(),
        }
    }

    fn open_path(&self, path: &str) -> Result<ISPHandle, DeviceSelectorError> {
        Ok(match self {
            Api::Native(api) => ISPHandle::from(api.open_path(path).wait()?),
            Api::Nusb(api) => ISPHandle::from(api.open_path(path).wait()?),
        })
    }
}

/// Which backend to open. `Api` is the value; this is the choice.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Backend {
    Native,
    Nusb,
}

impl core::fmt::Display for Backend {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(match self {
            Backend::Native => "native",
            Backend::Nusb => "nusb",
        })
    }
}

const BACKENDS: [Backend; 2] = [Backend::Native, Backend::Nusb];

pub struct DeviceSelector<B: HidBackend = HidraBackend> {
    api: B,
}

impl DeviceSelector {
    pub fn new() -> Result<Self, DeviceSelectorError> {
        Ok(Self::with_backend(HidraBackend::new()?))
    }
}

impl<B: HidBackend> DeviceSelector<B> {
    pub fn with_backend(api: B) -> Self {
        Self { api }
    }

    fn sorted_usb_device_list(&self) -> Vec<&B::Info> {
        let mut devices = self.api.devices();
        devices.retain(|d| d.is_usb());
        devices.sort_by_key(|d| d.sort_key());
        devices
    }

    fn unique_usb_device_list(&self) -> Vec<&B::Info> {
        let mut devices: Vec<_> = self.sorted_usb_device_list();
        devices.dedup_by_key(|d| {
            (
                d.vendor_id(),
                d.product_id(),
                d.interface_number(),
                d.path(),
            )
        });
        devices
    }

    fn get_feature_report_ids_from_path(
        &self,
        path: &str,
    ) -> Result<Vec<u32>, DeviceSelectorError> {
        let dev = self.api.open(path)?;
        self.get_feature_report_ids_from_device(&dev)
    }

    fn get_feature_report_ids_from_device(
        &self,
        dev: &B::Handle,
    ) -> Result<Vec<u32>, DeviceSelectorError> {
        let mut buf: [u8; MAX_REPORT_DESCRIPTOR_SIZE] = [0; MAX_REPORT_DESCRIPTOR_SIZE];
        let size: usize = self
            .api
            .report_descriptor(dev, &mut buf)
            .map_err(DeviceSelectorError::from)?;
        parse_feature_report_ids(&buf[..size])
    }

    fn get_report_descriptor(&self, dev: &B::Handle) -> Result<Vec<u8>, DeviceSelectorError> {
        let mut buf: [u8; MAX_REPORT_DESCRIPTOR_SIZE] = [0; MAX_REPORT_DESCRIPTOR_SIZE];
        let size: usize = self
            .api
            .report_descriptor(dev, &mut buf)
            .map_err(DeviceSelectorError::from)?;
        Ok(buf[..size].to_vec())
    }

    fn get_descriptor_with_features(
        &self,
        path: &str,
    ) -> (
        Result<Vec<u8>, DeviceSelectorError>,
        Result<Vec<u32>, DeviceSelectorError>,
    ) {
        let descriptor: Result<Vec<u8>, DeviceSelectorError>;
        let feature_report_ids: Result<Vec<u32>, DeviceSelectorError>;
        match self.api.open(path) {
            Ok(ref dev) => {
                descriptor = self.get_report_descriptor(dev);
                match descriptor {
                    Ok(ref report) => {
                        feature_report_ids = parse_feature_report_ids(report);
                    }
                    Err(_) => {
                        feature_report_ids = Err(DeviceSelectorError::NotFound);
                    }
                }
            }
            Err(err) => {
                descriptor = Err(err);
                feature_report_ids = Err(DeviceSelectorError::NotFound);
            }
        }
        (descriptor, feature_report_ids)
    }

    #[cfg(target_os = "windows")]
    fn get_devices_for_report_ids<'a, I: IntoIterator<Item = &'a B::Info>>(
        &self,
        devices: I,
        report_ids: &[u32],
    ) -> Result<Vec<&'a B::Info>, DeviceSelectorError> {
        let mut matched_devices: Vec<Option<&B::Info>> = vec![None; report_ids.len()];

        for d in devices {
            let retrieved_ids = self.get_feature_report_ids_from_path(d.path())?;
            for id in retrieved_ids {
                for (i, expected_id) in report_ids.iter().enumerate() {
                    if id == *expected_id {
                        if matched_devices[i].is_some() {
                            return Err(DeviceSelectorError::UnexpectedDeviceCount);
                        }
                        matched_devices[i] = Some(d);
                    }
                }
            }
        }

        matched_devices
            .into_iter()
            .collect::<Option<Vec<_>>>()
            .ok_or(DeviceSelectorError::NotFound)
    }

    #[cfg(not(target_os = "windows"))]
    fn get_device_for_report_ids<'a, I: IntoIterator<Item = &'a B::Info>>(
        &self,
        devices: I,
        report_ids: &[u32],
    ) -> Result<&'a B::Info, DeviceSelectorError> {
        let mut matching_devices = vec![];

        for d in devices {
            let retrieved_ids = self.get_feature_report_ids_from_path(d.path())?;
            if report_ids.iter().all(|id| retrieved_ids.contains(id)) {
                matching_devices.push(d);
            }
        }

        match matching_devices.len() {
            1 => Ok(matching_devices[0]),
            len if len > 1 => Err(DeviceSelectorError::UnexpectedDeviceCount),
            _ => Err(DeviceSelectorError::NotFound),
        }
    }

    fn find_isp_device(
        &self,
        device_spec: DeviceSpec,
    ) -> Result<ISPDevice<B::Handle>, DeviceSelectorError> {
        let mut isp_devices = self.unique_usb_device_list();
        isp_devices.retain(|d| {
            d.vendor_id() == GAMING_KB_VENDOR_ID
                && matches!(
                    d.product_id(),
                    GAMING_KB_PRODUCT_ID | GAMING_KB_V2_PRODUCT_ID
                )
                && d.interface_number() == GAMING_KB_IFACE
        });

        let device_count = isp_devices.len();
        if device_count == 0 {
            return Err(DeviceSelectorError::NotFound);
        }

        #[cfg(not(target_os = "windows"))]
        return {
            let device = self.get_device_for_report_ids(
                isp_devices,
                &[REPORT_ID_ISP as u32, REPORT_ID_XFER as u32],
            )?;
            debug!("ISP device: {}", device.info());

            let handle = self.api.open(device.path())?;

            Ok(ISPDevice::with_transport(device_spec, handle, None))
        };

        #[cfg(target_os = "windows")]
        return {
            let devices = self.get_devices_for_report_ids(
                isp_devices,
                &[REPORT_ID_ISP as u32, REPORT_ID_XFER as u32],
            )?;

            let cmd_device = devices[0];
            debug!("ISP CMD device: {}", cmd_device.info());

            let xfer_device = devices[1];
            debug!("ISP XFER device: {}", xfer_device.info());

            let cmd_handle = self.api.open(cmd_device.path())?;
            let xfer_handle = self.api.open(xfer_device.path())?;

            Ok(ISPDevice::with_transport(
                device_spec,
                cmd_handle,
                Some(xfer_handle),
            ))
        };
    }

    fn find_device(&self, device_spec: DeviceSpec) -> Result<B::Handle, DeviceSelectorError> {
        let filtered_devices = self.unique_usb_device_list().into_iter().filter(|d| {
            d.vendor_id() == device_spec.vendor_id
                && d.product_id() == device_spec.product_id
                && d.interface_number() == device_spec.isp_iface_num
        });

        let mut cmd_device_info: Option<&B::Info> = None;
        for d in filtered_devices {
            let ids = self
                .get_feature_report_ids_from_path(d.path())
                .map_err(|_| DeviceSelectorError::NotFound)?;
            for id in ids {
                if id == device_spec.isp_report_id {
                    cmd_device_info = Some(d);
                }
            }
        }

        let Some(cmd_device_info) = cmd_device_info else {
            info!("Device didn't come up...");
            return Err(DeviceSelectorError::NotFound);
        };

        debug!("Opening: {:?}", cmd_device_info.path());
        let device = self.api.open(cmd_device_info.path())?;
        Ok(device)
    }

    fn switch_to_isp_device(
        &mut self,
        device: B::Handle,
        device_spec: DeviceSpec,
    ) -> Result<ISPDevice<B::Handle>, DeviceSelectorError> {
        if let Err(err) = self.enter_isp_mode(&device) {
            debug!("Error: {err:}");
            match err {
                DeviceSelectorError::HidError(err) if is_expected_error(&err) => {}
                _ => {
                    error!("Unexpected: {err:}");
                    info!("Waiting...");
                    thread::sleep(ISP_SWITCH_DELAY);
                    return Err(err);
                }
            }
        }

        info!("Waiting for ISP device...");
        thread::sleep(ISP_SWITCH_DELAY);

        self.api.refresh()?;

        let Ok(isp_device) = self.find_isp_device(device_spec) else {
            info!("ISP device didn't come up...");
            return Err(DeviceSelectorError::NotFound);
        };
        Ok(isp_device)
    }

    pub fn try_fetch_isp_device(
        &mut self,
        device_spec: DeviceSpec,
        retries: usize,
    ) -> Result<ISPDevice<B::Handle>, DeviceSelectorError> {
        eprintln!(
            "Looking for {:04x}:{:04x} (isp_iface_num={} isp_report_id={})",
            device_spec.vendor_id,
            device_spec.product_id,
            device_spec.isp_iface_num,
            device_spec.isp_report_id
        );

        let bar = ProgressBar::new_spinner()
            .with_message(format!("Searching for device... Attempt {}/{}", 1, retries));
        bar.enable_steady_tick(Duration::from_millis(100));

        for attempt in 1..retries + 1 {
            if attempt > 1 {
                bar.set_message(format!("Retrying... Attempt {attempt}/{retries}"));
                info!("Retrying... Attempt {attempt}/{retries}");
                self.api.next_backend()?;
                thread::sleep(RETRY_DELAY);
            }

            match self.find_device(device_spec) {
                Ok(device) => {
                    bar.set_message("Device found. Switching to ISP mode...");
                    match self.switch_to_isp_device(device, device_spec) {
                        Ok(isp_device) => {
                            bar.finish_and_clear();
                            eprintln!("Connected!");
                            return Ok(isp_device);
                        }
                        Err(DeviceSelectorError::NotFound) => {}
                        Err(err) => {
                            return Err(err);
                        }
                    }
                }
                Err(DeviceSelectorError::NotFound) => {}
                Err(err) => {
                    return Err(err);
                }
            }

            info!("Device not found. Trying ISP device...");
            match self.find_isp_device(device_spec) {
                Ok(isp_device) => {
                    bar.finish_and_clear();
                    eprintln!("Connected!");
                    return Ok(isp_device);
                }
                Err(DeviceSelectorError::NotFound) => {}
                Err(err) => {
                    return Err(err);
                }
            }
        }
        bar.finish_and_clear();
        Err(DeviceSelectorError::NotFound)
    }

    fn enter_isp_mode(&self, handle: &B::Handle) -> Result<(), DeviceSelectorError> {
        let cmd: [u8; COMMAND_LENGTH] = [REPORT_ID_ISP, CMD_ISP_MODE, 0x00, 0x00, 0x00, 0x00];
        handle.send_feature_report(&cmd).wait()?;
        Ok(())
    }

    pub fn connected_devices_tree(&self) -> Result<Vec<DeviceNode>, DeviceSelectorError> {
        let devices: Vec<_> = self.sorted_usb_device_list();

        let id_chunks = devices
            .into_iter()
            .chunk_by(|d| (d.vendor_id(), d.product_id()));

        let mut device_tree_devices: Vec<DeviceNode> = vec![];

        for (key, devices) in &id_chunks {
            let (vid, pid) = key;

            let mut interface_nodes: Vec<InterfaceNode> = vec![];

            // for some reason on linux-libusb the same device might not have the same manufacturer string in some cases
            let mut manufacturer_string: Option<String> = None;
            let mut product_string: Option<String> = None;

            let path_chunks = devices.chunk_by(|d| (d.path(), d.interface_number()));

            for (key, devices) in &path_chunks {
                let (path, interface_number) = key;

                let mut children: Vec<ItemNode> = vec![];

                for d in devices {
                    if manufacturer_string.is_none() {
                        manufacturer_string = d.manufacturer_string().map(str::to_string);
                    }
                    if product_string.is_none() {
                        product_string = d.product_string().map(str::to_string);
                    }
                    #[cfg(not(target_os = "windows"))]
                    children.push(ItemNode {
                        usage_page: d.usage_page(),
                        usage: d.usage(),
                    });
                    #[cfg(target_os = "windows")]
                    {
                        let (descriptor, feature_report_ids) =
                            self.get_descriptor_with_features(path);
                        children.push(ItemNode {
                            path: path.to_string(),
                            usage_page: d.usage_page(),
                            usage: d.usage(),
                            descriptor,
                            feature_report_ids,
                        });
                    }
                }

                #[cfg(not(target_os = "windows"))]
                let (descriptor, feature_report_ids) = self.get_descriptor_with_features(path);
                let interface_node = InterfaceNode {
                    #[cfg(not(target_os = "windows"))]
                    path: path.to_string(),
                    interface_number,
                    #[cfg(not(target_os = "windows"))]
                    descriptor,
                    #[cfg(not(target_os = "windows"))]
                    feature_report_ids,
                    children,
                };

                interface_nodes.push(interface_node);
            }

            device_tree_devices.push(DeviceNode {
                vendor_id: vid,
                product_id: pid,
                manufacturer_string: manufacturer_string.unwrap_or_else(|| "None".to_string()),
                product_string: product_string.unwrap_or_else(|| "None".to_string()),
                children: interface_nodes,
            });
        }
        Ok(device_tree_devices)
    }
}

fn parse_feature_report_ids(descriptor: &[u8]) -> Result<Vec<u32>, DeviceSelectorError> {
    let report_descriptor =
        parse_report_descriptor(descriptor).map_err(DeviceSelectorError::ReportDescriptorError)?;
    let res = report_descriptor
        .features
        .iter()
        .filter_map(|item| item.report_id)
        .map(|report_id| report_id.into())
        .collect();
    Ok(res)
}

#[cfg(test)]
mod tests {
    use hidra::MaybeFuture;
    use sinowisp::DEVICE_BASE_SH68F90;
    use sinowisp_testing::FakeBootloader;

    use super::*;
    use crate::fake_hid::{
        FakeDevice, FakeHid, ISP_DESCRIPTOR, ISP_PATH, KEYBOARD_DESCRIPTOR, VENDOR_DESCRIPTOR,
    };

    const SPEC: DeviceSpec = DeviceSpec {
        vendor_id: 0x05ac,
        product_id: 0x024f,
        ..DEVICE_BASE_SH68F90
    };
    const ISP_MODE: [u8; 6] = [0x05, 0x75, 0, 0, 0, 0];

    fn keyboard() -> Vec<FakeDevice> {
        vec![
            FakeDevice::new("kbd0", 0x05ac, 0x024f, 0),
            FakeDevice::new("kbd1", 0x05ac, 0x024f, 1).descriptor(VENDOR_DESCRIPTOR),
        ]
    }

    #[test]
    fn test_parse_feature_report_ids() {
        assert_eq!(
            parse_feature_report_ids(KEYBOARD_DESCRIPTOR).unwrap(),
            vec![]
        );
        assert_eq!(
            parse_feature_report_ids(VENDOR_DESCRIPTOR).unwrap(),
            vec![5]
        );
        assert_eq!(
            parse_feature_report_ids(ISP_DESCRIPTOR).unwrap(),
            vec![5, 6]
        );
    }

    #[test]
    fn test_parse_feature_report_ids_rejects_malformed_descriptor() {
        let pop_without_push = &[0xb4];
        assert!(matches!(
            parse_feature_report_ids(pop_without_push),
            Err(DeviceSelectorError::ReportDescriptorError(_))
        ));
    }

    #[test]
    fn test_switches_keyboard_into_isp_mode() {
        let bootloader = FakeBootloader::new(SPEC);
        let hid = FakeHid::new(&bootloader, keyboard(), vec![FakeDevice::isp()]);
        let state = hid.state();
        let mut selector = DeviceSelector::with_backend(hid);

        let device = selector.try_fetch_isp_device(SPEC, 1).unwrap();
        device.erase().wait().unwrap();

        assert_eq!(
            state.sent(),
            vec![
                ("kbd1".to_string(), ISP_MODE.to_vec()),
                (ISP_PATH.to_string(), vec![0x05, 0x45, 0, 0, 0, 0]),
            ]
        );
    }

    #[test]
    fn test_uses_device_already_in_isp_mode() {
        let bootloader = FakeBootloader::new(SPEC);
        let hid = FakeHid::new(&bootloader, vec![FakeDevice::isp()], vec![]);
        let state = hid.state();
        let mut selector = DeviceSelector::with_backend(hid);

        selector.try_fetch_isp_device(SPEC, 1).unwrap();

        assert_eq!(state.sent(), vec![]);
    }

    #[test]
    fn test_tolerates_disconnect_while_switching() {
        let bootloader = FakeBootloader::new(SPEC);
        let hid = FakeHid::new(&bootloader, keyboard(), vec![FakeDevice::isp()])
            .failing_isp_switch(|| HidError::Disconnected);
        let mut selector = DeviceSelector::with_backend(hid);

        assert!(selector.try_fetch_isp_device(SPEC, 1).is_ok());
    }

    #[test]
    fn test_fails_on_unexpected_switch_error() {
        let bootloader = FakeBootloader::new(SPEC);
        let hid = FakeHid::new(&bootloader, keyboard(), vec![FakeDevice::isp()])
            .failing_isp_switch(|| HidError::DeviceNotFound);
        let mut selector = DeviceSelector::with_backend(hid);

        assert!(matches!(
            selector.try_fetch_isp_device(SPEC, 1),
            Err(DeviceSelectorError::HidError(HidError::DeviceNotFound))
        ));
    }

    #[test]
    fn test_tries_every_backend_before_giving_up() {
        let bootloader = FakeBootloader::new(SPEC);
        let hid = FakeHid::new(&bootloader, keyboard(), vec![]);
        let state = hid.state();
        let mut selector = DeviceSelector::with_backend(hid);

        let result = selector.try_fetch_isp_device(SPEC, 3);

        assert!(matches!(result, Err(DeviceSelectorError::NotFound)));
        assert_eq!(state.backend_switches(), 2);
    }

    #[test]
    fn test_ignores_interfaces_without_isp_report() {
        let bootloader = FakeBootloader::new(SPEC);
        let devices = vec![
            FakeDevice::new("kbd0", 0x05ac, 0x024f, 0).descriptor(VENDOR_DESCRIPTOR),
            FakeDevice::new("kbd1", 0x05ac, 0x024f, 1),
        ];
        let hid = FakeHid::new(&bootloader, devices, vec![]);
        let state = hid.state();
        let mut selector = DeviceSelector::with_backend(hid);

        let result = selector.try_fetch_isp_device(SPEC, 1);

        assert!(matches!(result, Err(DeviceSelectorError::NotFound)));
        assert_eq!(state.sent(), vec![]);
    }

    #[cfg(not(target_os = "windows"))]
    #[test]
    fn test_rejects_two_isp_devices() {
        let bootloader = FakeBootloader::new(SPEC);
        let second = FakeDevice {
            path: "isp2".to_string(),
            ..FakeDevice::isp()
        };
        let hid = FakeHid::new(&bootloader, vec![FakeDevice::isp(), second], vec![]);
        let mut selector = DeviceSelector::with_backend(hid);

        assert!(matches!(
            selector.try_fetch_isp_device(SPEC, 1),
            Err(DeviceSelectorError::UnexpectedDeviceCount)
        ));
    }

    #[cfg(not(target_os = "windows"))]
    #[test]
    fn test_skips_isp_device_without_transfer_report() {
        let bootloader = FakeBootloader::new(SPEC);
        let hid = FakeHid::new(
            &bootloader,
            vec![FakeDevice::isp().descriptor(VENDOR_DESCRIPTOR)],
            vec![],
        );
        let mut selector = DeviceSelector::with_backend(hid);

        assert!(matches!(
            selector.try_fetch_isp_device(SPEC, 1),
            Err(DeviceSelectorError::NotFound)
        ));
    }

    #[cfg(not(target_os = "windows"))]
    #[test]
    fn test_connected_devices_tree() {
        use crate::hid_tree::TreeDisplay;
        use sinowisp::to_hex_string;

        let bootloader = FakeBootloader::new(SPEC);
        let mut receiver =
            FakeDevice::new("receiver", 0x046d, 0xc52b, 2).descriptor(ISP_DESCRIPTOR);
        receiver.manufacturer = None;
        let mut bluetooth = FakeDevice::new("bluetooth", 0x0001, 0x0001, 0);
        bluetooth.usb = false;
        let mut unopenable = FakeDevice::new("unopenable", 0x05ac, 0x024f, 2);
        unopenable.descriptor = None;
        let devices = vec![
            unopenable,
            FakeDevice::new("kbd1", 0x05ac, 0x024f, 1).descriptor(VENDOR_DESCRIPTOR),
            bluetooth,
            receiver,
        ];
        let selector = DeviceSelector::with_backend(FakeHid::new(&bootloader, devices, vec![]));

        let tree = selector
            .connected_devices_tree()
            .unwrap()
            .into_iter()
            .to_tree_string(0);

        let usage = "        usage_page=0x0001 usage=0x0006".to_string();
        let expected = [
            "ID 046d:c52b manufacturer=\"None\" product=\"Gaming KB\"".to_string(),
            "    path=\"receiver\" interface_number=2".to_string(),
            format!("    report_descriptor=[{}]", to_hex_string(ISP_DESCRIPTOR)),
            "    feature_report_ids=[5, 6]".to_string(),
            usage.clone(),
            "ID 05ac:024f manufacturer=\"SINO WEALTH\" product=\"Gaming KB\"".to_string(),
            "    path=\"kbd1\" interface_number=1".to_string(),
            format!(
                "    report_descriptor=[{}]",
                to_hex_string(VENDOR_DESCRIPTOR)
            ),
            "    feature_report_ids=[5]".to_string(),
            usage.clone(),
            "    path=\"unopenable\" interface_number=2".to_string(),
            format!("    report_descriptor=error: {}", HidError::DeviceNotFound),
            "    feature_report_ids=error: Device not found".to_string(),
            usage,
        ];
        assert_eq!(tree, expected.join("\n"));
    }
}
