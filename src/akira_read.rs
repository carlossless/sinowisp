use std::time::Duration;

use nusb::transfer::{ControlIn, ControlOut, ControlType, Recipient};
use nusb::MaybeFuture;

use crate::device_selector::{
    GAMING_KB_IFACE, GAMING_KB_PRODUCT_ID, GAMING_KB_V2_PRODUCT_ID, GAMING_KB_VENDOR_ID,
};

const TIMEOUT: Duration = Duration::from_millis(2000);
const SET_REPORT: u8 = 0x09;
const GET_REPORT: u8 = 0x01;
const REPORT_TYPE_FEATURE: u16 = 0x03;
const REPORT_ID_CMD: u8 = 0x05;
const REPORT_ID_RAW: u8 = 0x41;
const AKIRA: [u8; 6] = [REPORT_ID_CMD, b'A', b'K', b'I', b'R', b'A'];
const READ_LOCK_ADDR: u16 = 0xfe27;
const READ_LOCK_UNLOCKED: u8 = 0xa5;
const CHUNK: usize = 64;

#[derive(thiserror::Error, Debug)]
pub enum RawReadError {
    #[error("ISP device {0:04x}:{1:04x} not found")]
    NotFound(u16, u16),
    #[error(transparent)]
    Usb(#[from] nusb::Error),
    #[error("USB control transfer failed ({0})")]
    Transfer(nusb::transfer::TransferError),
}

impl From<nusb::transfer::TransferError> for RawReadError {
    fn from(e: nusb::transfer::TransferError) -> Self {
        RawReadError::Transfer(e)
    }
}

#[cfg(target_os = "windows")]
type Handle = nusb::Interface;
#[cfg(not(target_os = "windows"))]
type Handle = nusb::Device;

fn set_report(h: &Handle, report_id: u8, index: u16, data: &[u8]) -> Result<(), RawReadError> {
    h.control_out(
        ControlOut {
            control_type: ControlType::Class,
            recipient: Recipient::Interface,
            request: SET_REPORT,
            value: (REPORT_TYPE_FEATURE << 8) | report_id as u16,
            index,
            data,
        },
        TIMEOUT,
    )
    .wait()?;
    Ok(())
}

fn get_report(h: &Handle, report_id: u8, index: u16, length: u16) -> Result<Vec<u8>, RawReadError> {
    Ok(h.control_in(
        ControlIn {
            control_type: ControlType::Class,
            recipient: Recipient::Interface,
            request: GET_REPORT,
            value: (REPORT_TYPE_FEATURE << 8) | report_id as u16,
            index,
            length,
        },
        TIMEOUT,
    )
    .wait()?)
}

fn open() -> Result<Handle, RawReadError> {
    let pids = [GAMING_KB_PRODUCT_ID, GAMING_KB_V2_PRODUCT_ID];
    let info = nusb::list_devices()
        .wait()?
        .find(|d| d.vendor_id() == GAMING_KB_VENDOR_ID && pids.contains(&d.product_id()))
        .ok_or(RawReadError::NotFound(
            GAMING_KB_VENDOR_ID,
            GAMING_KB_PRODUCT_ID,
        ))?;
    let dev = info.open().wait()?;
    #[cfg(target_os = "windows")]
    return Ok(dev.claim_interface(GAMING_KB_IFACE as u8).wait()?);
    #[cfg(not(target_os = "windows"))]
    Ok(dev)
}

pub fn read(
    start_addr: usize,
    length: usize,
    progress: &dyn Fn(usize, usize),
) -> Result<Vec<u8>, RawReadError> {
    let h = open()?;
    let iface = GAMING_KB_IFACE as u16;

    set_report(&h, REPORT_ID_CMD, iface, &AKIRA)?;
    set_report(&h, REPORT_ID_RAW, READ_LOCK_ADDR, &[READ_LOCK_UNLOCKED])?;

    let mut out: Vec<u8> = Vec::with_capacity(length);
    while out.len() < length {
        let addr = start_addr + out.len();
        let n = CHUNK.min(length - out.len());
        let data = get_report(&h, REPORT_ID_RAW, addr as u16, n as u16)?;
        if data.is_empty() {
            break;
        }
        out.extend_from_slice(&data);
        progress(out.len().min(length), length);
    }
    out.truncate(length);
    Ok(out)
}
