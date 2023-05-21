use rusb::*;
use std::time;

pub struct HidDevice {
    handle: DeviceHandle<GlobalContext>,
    interface: u8,
}

impl HidDevice {
    pub fn open(vendor_id: u16, product_id: u16) -> Option<HidDevice> {
        let Some(mut handle) = open_device_with_vid_pid(vendor_id, product_id) else {
            return None;
        };

        let interface = handle
            .device()
            .config_descriptor(0)
            .unwrap()
            .interfaces()
            .map(|i| i.descriptors())
            .flatten()
            .filter(|i| i.class_code() == constants::LIBUSB_CLASS_HID)
            .map(|i| i.interface_number())
            .last()
            .unwrap();

        if supports_detach_kernel_driver() {
            handle.set_auto_detach_kernel_driver(true).unwrap();
        }

        #[cfg(not(any(target_os = "macos")))]
        handle.claim_interface(interface).unwrap();

        return Some(HidDevice {
            handle: handle,
            interface: interface,
        });
    }

    pub fn get_feature_report(self: &Self, buf: &mut [u8]) -> Result<usize> {
        let report_number = buf[0] as u16;

        return self.handle.read_control(
            constants::LIBUSB_REQUEST_TYPE_CLASS
                | constants::LIBUSB_RECIPIENT_INTERFACE
                | constants::LIBUSB_ENDPOINT_IN,
            0x01,
            (3/*HID feature*/ << 8) | report_number,
            self.interface as u16,
            buf,
            time::Duration::from_millis(1000),
        );
    }

    pub fn send_feature_report(self: &Self, buf: &[u8]) -> Result<usize> {
        let report_number = buf[0] as u16;

        return self.handle.write_control(
            constants::LIBUSB_REQUEST_TYPE_CLASS
                | constants::LIBUSB_RECIPIENT_INTERFACE
                | constants::LIBUSB_ENDPOINT_OUT,
            0x09,
            (3/*HID feature*/ << 8) | report_number,
            self.interface as u16,
            buf,
            time::Duration::from_millis(1000),
        );
    }
}
