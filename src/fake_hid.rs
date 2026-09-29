use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

use hidra::HidError;
use sinowisp::{testing::FakeBootloader, Transport};

use crate::device_selector::{DeviceSelectorError, HidBackend, HidInfo};

pub const ISP_PATH: &str = "isp";

pub const KEYBOARD_DESCRIPTOR: &[u8] = &[
    0x05, 0x01, 0x09, 0x06, 0xA1, 0x01, 0x05, 0x07, 0x19, 0xE0, 0x29, 0xE7, 0x15, 0x00, 0x25, 0x01,
    0x75, 0x01, 0x95, 0x08, 0x81, 0x02, 0x75, 0x08, 0x95, 0x01, 0x81, 0x01, 0x05, 0x07, 0x19, 0x00,
    0x29, 0xFF, 0x15, 0x00, 0x26, 0xFF, 0x00, 0x75, 0x08, 0x95, 0x06, 0x81, 0x00, 0x05, 0x08, 0x19,
    0x01, 0x29, 0x05, 0x15, 0x00, 0x25, 0x01, 0x75, 0x01, 0x95, 0x05, 0x91, 0x02, 0x75, 0x03, 0x95,
    0x01, 0x91, 0x01, 0xC0,
];

pub const VENDOR_DESCRIPTOR: &[u8] = &[
    0x05, 0x01, 0x09, 0x80, 0xA1, 0x01, 0x85, 0x01, 0x19, 0x81, 0x29, 0x83, 0x15, 0x00, 0x25, 0x01,
    0x75, 0x01, 0x95, 0x03, 0x81, 0x02, 0x95, 0x05, 0x81, 0x01, 0xC0, 0x05, 0x0C, 0x09, 0x01, 0xA1,
    0x01, 0x85, 0x02, 0x19, 0x00, 0x2A, 0x3C, 0x02, 0x15, 0x00, 0x26, 0x3C, 0x02, 0x75, 0x10, 0x95,
    0x01, 0x81, 0x00, 0xC0, 0x06, 0x00, 0xFF, 0x09, 0x01, 0xA1, 0x01, 0x85, 0x05, 0x19, 0x01, 0x29,
    0x02, 0x15, 0x00, 0x26, 0xFF, 0x00, 0x75, 0x08, 0x95, 0x05, 0xB1, 0x02, 0xC0, 0x05, 0x01, 0x09,
    0x06, 0xA1, 0x01, 0x85, 0x06, 0x05, 0x07, 0x19, 0xE0, 0x29, 0xE7, 0x15, 0x00, 0x25, 0x01, 0x75,
    0x01, 0x95, 0x08, 0x81, 0x02, 0x05, 0x07, 0x19, 0x00, 0x29, 0x9F, 0x15, 0x00, 0x25, 0x01, 0x75,
    0x01, 0x95, 0xA0, 0x81, 0x02, 0xC0,
];

pub const ISP_DESCRIPTOR: &[u8] = &[
    0x06, 0x00, 0xFF, 0x09, 0x01, 0xA1, 0x01, 0x85, 0x05, 0x15, 0x00, 0x25, 0xFF, 0x75, 0x08, 0x95,
    0x05, 0xB1, 0x02, 0x85, 0x06, 0x96, 0x08, 0x01, 0xB1, 0x02, 0xC0,
];

#[derive(Clone)]
pub struct FakeDevice {
    pub path: String,
    pub vendor_id: u16,
    pub product_id: u16,
    pub interface_number: i32,
    pub usage_page: u16,
    pub usage: u16,
    pub manufacturer: Option<String>,
    pub product: Option<String>,
    pub usb: bool,
    pub descriptor: Option<&'static [u8]>,
}

impl FakeDevice {
    pub fn new(path: &str, vendor_id: u16, product_id: u16, interface_number: i32) -> Self {
        Self {
            path: path.to_string(),
            vendor_id,
            product_id,
            interface_number,
            usage_page: 0x0001,
            usage: 0x0006,
            manufacturer: Some("SINO WEALTH".to_string()),
            product: Some("Gaming KB".to_string()),
            usb: true,
            descriptor: Some(KEYBOARD_DESCRIPTOR),
        }
    }

    pub fn descriptor(mut self, descriptor: &'static [u8]) -> Self {
        self.descriptor = Some(descriptor);
        self
    }

    pub fn isp() -> Self {
        Self::new(ISP_PATH, 0x0603, 0x1020, 0).descriptor(ISP_DESCRIPTOR)
    }
}

impl HidInfo for FakeDevice {
    fn path(&self) -> &str {
        &self.path
    }
    fn vendor_id(&self) -> u16 {
        self.vendor_id
    }
    fn product_id(&self) -> u16 {
        self.product_id
    }
    fn interface_number(&self) -> i32 {
        self.interface_number
    }
    fn usage_page(&self) -> u16 {
        self.usage_page
    }
    fn usage(&self) -> u16 {
        self.usage
    }
    fn manufacturer_string(&self) -> Option<&str> {
        self.manufacturer.as_deref()
    }
    fn product_string(&self) -> Option<&str> {
        self.product.as_deref()
    }
    fn is_usb(&self) -> bool {
        self.usb
    }
}

pub struct FakeHid<'a> {
    devices: Vec<FakeDevice>,
    after_isp_switch: Vec<FakeDevice>,
    state: Rc<State<'a>>,
}

pub struct State<'a> {
    bootloader: &'a FakeBootloader,
    isp_switch_error: Option<fn() -> HidError>,
    isp_requested: Cell<bool>,
    sent: RefCell<Vec<(String, Vec<u8>)>>,
    backend_switches: Cell<usize>,
}

impl<'a> FakeHid<'a> {
    pub fn new(
        bootloader: &'a FakeBootloader,
        devices: Vec<FakeDevice>,
        after_isp_switch: Vec<FakeDevice>,
    ) -> Self {
        Self {
            devices,
            after_isp_switch,
            state: Rc::new(State {
                bootloader,
                isp_switch_error: None,
                isp_requested: Cell::new(false),
                sent: RefCell::new(vec![]),
                backend_switches: Cell::new(0),
            }),
        }
    }

    pub fn failing_isp_switch(mut self, error: fn() -> HidError) -> Self {
        Rc::get_mut(&mut self.state).unwrap().isp_switch_error = Some(error);
        self
    }

    pub fn state(&self) -> Rc<State<'a>> {
        self.state.clone()
    }
}

impl State<'_> {
    pub fn sent(&self) -> Vec<(String, Vec<u8>)> {
        self.sent.borrow().clone()
    }

    pub fn backend_switches(&self) -> usize {
        self.backend_switches.get()
    }
}

pub struct FakeHandle<'a> {
    path: String,
    state: Rc<State<'a>>,
}

impl Transport for FakeHandle<'_> {
    async fn send_feature_report(&self, data: &[u8]) -> Result<(), HidError> {
        self.state
            .sent
            .borrow_mut()
            .push((self.path.clone(), data.to_vec()));
        if self.path == ISP_PATH {
            return self.state.bootloader.send_feature_report(data).await;
        }
        if data[1] == 0x75 {
            self.state.isp_requested.set(true);
            if let Some(error) = self.state.isp_switch_error {
                return Err(error());
            }
        }
        Ok(())
    }

    async fn get_feature_report(&self, buf: &mut [u8]) -> Result<usize, HidError> {
        self.state.bootloader.get_feature_report(buf).await
    }
}

impl<'a> HidBackend for FakeHid<'a> {
    type Info = FakeDevice;
    type Handle = FakeHandle<'a>;

    fn devices(&self) -> Vec<&FakeDevice> {
        self.devices.iter().collect()
    }

    fn open(&self, path: &str) -> Result<FakeHandle<'a>, DeviceSelectorError> {
        let device = self.devices.iter().find(|d| d.path == path);
        match device {
            Some(d) if d.descriptor.is_some() => Ok(FakeHandle {
                path: path.to_string(),
                state: self.state.clone(),
            }),
            _ => Err(HidError::DeviceNotFound.into()),
        }
    }

    fn report_descriptor(
        &self,
        handle: &FakeHandle<'a>,
        buf: &mut [u8],
    ) -> Result<usize, HidError> {
        let device = self.devices.iter().find(|d| d.path == handle.path).unwrap();
        let descriptor = device.descriptor.unwrap();
        buf[..descriptor.len()].copy_from_slice(descriptor);
        Ok(descriptor.len())
    }

    fn refresh(&mut self) -> Result<(), DeviceSelectorError> {
        if self.state.isp_requested.get() {
            self.devices = self.after_isp_switch.clone();
        }
        Ok(())
    }

    fn next_backend(&mut self) -> Result<(), DeviceSelectorError> {
        self.state
            .backend_switches
            .set(self.state.backend_switches.get() + 1);
        Ok(())
    }
}
