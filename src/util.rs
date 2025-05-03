use crate::part::Part;
use hidapi::HidError;
use log::error;
use thiserror::Error;

#[cfg(test)]
use crate::part::PART_BASE_SH68F90;

#[derive(Debug, Clone, Error, PartialEq)]
pub enum VerificationError {
    #[error("Firmware Mismatch @ {addr:#06x} --- {expected:#04x} != {actual:#04x}")]
    ByteMismatch {
        addr: usize,
        expected: u8,
        actual: u8,
    },
    #[error("Length Mismatch {expected} {actual}")]
    LengthMismatch { expected: usize, actual: usize },
}

pub fn verify(expected: &[u8], actual: &[u8]) -> Result<(), VerificationError> {
    if expected.len() != actual.len() {
        return Err(VerificationError::LengthMismatch {
            expected: expected.len(),
            actual: actual.len(),
        });
    }

    for i in 0..expected.len() {
        if expected[i] != actual[i] {
            return Err(VerificationError::ByteMismatch {
                addr: i,
                expected: expected[i],
                actual: actual[i],
            });
        }
    }

    Ok(())
}

#[derive(Debug, Error)]
pub enum PayloadConversionError {
    #[error("Expected LJMP not found at {addr:#06x}")]
    LJMPNotFoundError { addr: u16 },
    #[error("Unexpected addr at {source_addr:#06x} pointing to {target_addr:#06x}")]
    UnexpectedAddressError { source_addr: u16, target_addr: u16 },
}

pub fn convert_to_jtag_payload(input: &mut [u8], part: Part) -> Result<(), PayloadConversionError> {
    if input[0] != 0x02 {
        return Err(PayloadConversionError::LJMPNotFoundError { addr: 0x0000 });
    }

    let main_fw_address = u16::from_be_bytes(input[1..3].try_into().unwrap());
    if main_fw_address > 0xefff {
        return Err(PayloadConversionError::UnexpectedAddressError {
            source_addr: 0x0001,
            target_addr: main_fw_address,
        });
    }

    let bootloader_ljmp_addr = (part.firmware_size as u16).to_be_bytes();
    let ljmp_addr = part.firmware_size - 5;

    input[1..3].copy_from_slice(&bootloader_ljmp_addr);
    input[ljmp_addr] = 0x02;
    input[ljmp_addr + 1..ljmp_addr + 3].copy_from_slice(&main_fw_address.to_be_bytes());

    Ok(())
}

pub fn convert_to_isp_payload(input: &mut [u8], part: Part) -> Result<(), PayloadConversionError> {
    if input[0] != 0x02 {
        return Err(PayloadConversionError::LJMPNotFoundError { addr: 0 });
    }

    let ljmp_addr = part.firmware_size - 5;
    if input[ljmp_addr] != 0x02 {
        return Err(PayloadConversionError::LJMPNotFoundError { addr: 0x0000 });
    }

    let main_fw_address =
        u16::from_be_bytes(input[ljmp_addr + 1..ljmp_addr + 3].try_into().unwrap());
    if main_fw_address > 0xefff {
        return Err(PayloadConversionError::UnexpectedAddressError {
            source_addr: (ljmp_addr + 1) as u16,
            target_addr: main_fw_address,
        });
    }

    input[1..3].copy_from_slice(&main_fw_address.to_be_bytes());
    input[ljmp_addr..ljmp_addr + 3].fill(0x00);

    Ok(())
}

pub fn to_hex_string(bytes: &[u8]) -> String {
    let strs: Vec<String> = bytes.iter().map(|b| format!("{:02X}", b)).collect();
    strs.join(" ")
}

pub fn is_expected_error(err: &HidError) -> bool {
    match err {
        #[cfg(target_os = "macos")]
        HidError::HidApiError { ref message } if message == "IOHIDDeviceSetReport failed: (0xE0005000) unknown error code" => true,
        #[cfg(target_os = "linux")]
        HidError::HidApiError { ref message } if message == "hid_error is not implemented yet" => true,
        #[cfg(target_os = "windows")]
        HidError::HidApiError { ref message } if message == "HidD_SetFeature: (0x0000001F) A device attached to the system is not functioning." => true,
        _ => false,
    }
}

#[test]
fn test_verify_success() {
    assert!(verify(&vec![1, 2, 3, 4], &vec![1, 2, 3, 4]).is_ok());
}

#[test]
fn test_verify_error_length_mismatch() {
    assert_eq!(
        verify(&vec![1, 2, 3, 4], &vec![1, 2, 3]),
        Err(VerificationError::LengthMismatch {
            expected: 4,
            actual: 3
        })
    );
}

#[test]
fn test_verify_error_byte_mismatch() {
    assert_eq!(
        verify(&vec![1, 2, 3, 4], &vec![1, 2, 4, 3]),
        Err(VerificationError::ByteMismatch {
            addr: 2,
            expected: 3,
            actual: 4
        })
    );
}

#[test]
fn test_convert_to_jtag_payload() {
    let part = PART_BASE_SH68F90;
    let mut firmware: [u8; 65536] = [0; 65536];
    firmware[0] = 0x02;
    firmware[1] = 0x00;
    firmware[2] = 0x66;

    convert_to_jtag_payload(&mut firmware, part).unwrap();

    assert_eq!(firmware[0..3], [0x02, 0xf0, 0x00]);
    assert_eq!(firmware[0xeffb..0xeffe], [0x02, 0x00, 0x66]);
}

#[test]
fn test_convert_to_isp_payload() {
    let part = PART_BASE_SH68F90;
    let mut firmware: [u8; 65536] = [0; 65536];
    firmware[0] = 0x02;
    firmware[1] = 0xf0;
    firmware[2] = 0x00;
    firmware[0xeffb] = 0x02;
    firmware[0xeffc] = 0x00;
    firmware[0xeffd] = 0x66;

    convert_to_isp_payload(&mut firmware, part).unwrap();

    assert_eq!(firmware[0..3], [0x02, 0x00, 0x66]);
    assert_eq!(firmware[0xeffb..0xeffe], [0x00, 0x00, 0x00]);
}
