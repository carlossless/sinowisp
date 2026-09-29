//! Full read/write cycles, composed from the `sinowisp` protocol
//! primitives. This is where the orchestration lives: page loops, the
//! post-erase/reboot settle delays, terminal progress bars, and verification.

use std::{thread, time::Duration};

use hidra::MaybeFuture;
use indicatif::ProgressBar;
use log::{debug, error, warn};
use sinowisp::{
    check_bootloader, is_expected_error, verify, ISPDevice, ISPError, ReadMode, ReadSection,
    Transport,
};

use crate::{akira_read, CLIError};

/// Time the device needs to settle after an erase or reboot before it will
/// accept (or has finished acting on) further commands.
const SETTLE_DELAY: Duration = if cfg!(test) {
    Duration::ZERO
} else {
    Duration::from_millis(2000)
};

pub fn read_cycle<T: Transport>(
    device: &ISPDevice<T>,
    section: ReadSection,
) -> Result<Vec<u8>, CLIError> {
    let spec = *device.device_spec();

    let (start_addr, length) = match section {
        ReadSection::Firmware => (0, spec.platform.firmware_size),
        ReadSection::Bootloader => (spec.platform.firmware_size, spec.platform.bootloader_size),
        ReadSection::Full => (
            0,
            spec.platform.firmware_size + spec.platform.bootloader_size,
        ),
    };

    let firmware = match spec.read_mode {
        ReadMode::Standard => {
            eprintln!("Enabling firmware...");
            device.enable_firmware().wait()?;
            read(device, start_addr, length)?
        }
        ReadMode::Akira => {
            if !cfg!(any(target_os = "macos", target_os = "windows")) {
                return Err(ISPError::Unsupported(
                    "reading this device requires the raw AKIRA unlock, which only works on macOS and Windows",
                )
                .into());
            }
            read_akira(start_addr, length)?
        }
    };

    let bootloader = match section {
        ReadSection::Firmware => None,
        ReadSection::Bootloader => Some(&firmware[..]),
        ReadSection::Full => firmware.get(spec.platform.firmware_size..),
    };
    if let Some(Err(err)) = bootloader
        .filter(|_| spec.check_bootloader)
        .map(check_bootloader)
    {
        warn!("{err}");
    }

    if spec.reboot {
        reboot(device);
    }

    Ok(firmware)
}

pub fn write_cycle<T: Transport>(
    device: &ISPDevice<T>,
    firmware: &mut [u8],
) -> Result<(), ISPError> {
    let spec = *device.device_spec();

    // ensure that the address at <firmware_size-4> is the same as the reset vector
    firmware.copy_within(1..3, spec.platform.firmware_size - 4);

    erase(device)?;
    write(device, 0, firmware)?;

    // cleanup the address at <firmware_size-4>
    firmware[spec.platform.firmware_size - 4..spec.platform.firmware_size - 2].fill(0);

    let read_back = read(device, 0, spec.platform.firmware_size)?;

    eprintln!("Verifying...");
    verify(firmware, &read_back).map_err(ISPError::from)?;

    eprintln!("Enabling firmware...");
    device.enable_firmware().wait()?;

    if spec.reboot {
        reboot(device);
    }

    Ok(())
}

fn read<T: Transport>(
    device: &ISPDevice<T>,
    start_addr: usize,
    length: usize,
) -> Result<Vec<u8>, ISPError> {
    let page_size = device.device_spec().platform.page_size;
    let num_page = length / page_size;

    eprintln!("Reading...");
    let bar = ProgressBar::new(num_page as u64);

    let result = device
        .read(start_addr, length, &|done, _total| {
            debug!(
                "Reading page {} @ offset {:#06x}",
                done - 1,
                start_addr + (done - 1) * page_size
            );
            bar.set_position(done as u64);
        })
        .wait()?;

    bar.finish();
    Ok(result)
}

fn read_akira(start_addr: usize, length: usize) -> Result<Vec<u8>, CLIError> {
    eprintln!("Reading...");
    let bar = ProgressBar::new(length as u64);

    let result = akira_read::read(start_addr, length, &|done, _total| {
        bar.set_position(done as u64);
    })?;

    bar.finish();
    Ok(result)
}

fn write<T: Transport>(
    device: &ISPDevice<T>,
    start_addr: usize,
    buffer: &[u8],
) -> Result<(), ISPError> {
    let page_size = device.device_spec().platform.page_size;

    eprintln!("Writing...");
    let bar = ProgressBar::new(device.device_spec().num_pages() as u64);

    device
        .write(start_addr, buffer, &|done, _total| {
            debug!(
                "Writing page {} @ offset {:#06x}",
                done - 1,
                (done - 1) * page_size
            );
            bar.set_position(done as u64);
        })
        .wait()?;

    bar.finish();
    Ok(())
}

fn erase<T: Transport>(device: &ISPDevice<T>) -> Result<(), ISPError> {
    eprintln!("Erasing...");
    device.erase().wait()?;
    thread::sleep(SETTLE_DELAY);
    Ok(())
}

fn reboot<T: Transport>(device: &ISPDevice<T>) {
    eprintln!("Rebooting...");
    if let Err(err) = device.reboot().wait() {
        debug!("Error: {err:}");
        let expected = matches!(&err, ISPError::HidError(hid) if is_expected_error(hid));
        if !expected {
            error!("Unexpected error: {err:}");
        }
    }
    thread::sleep(SETTLE_DELAY);
}

#[cfg(test)]
mod tests {
    use sinowisp::{DeviceSpec, IspTransform, VerificationError, DEVICE_BASE_SH68F90};
    use sinowisp_testing::FakeBootloader;

    use super::*;

    const SPEC: DeviceSpec = DEVICE_BASE_SH68F90;
    const FW: usize = SPEC.platform.firmware_size;

    fn isp_form_firmware() -> Vec<u8> {
        let mut firmware: Vec<u8> = (0..FW).map(|i| (i * 7 + i / 256) as u8).collect();
        firmware[..3].copy_from_slice(&[0x02, 0x00, 0x66]);
        firmware[FW - 5..FW - 2].fill(0);
        firmware
    }

    fn commands(fake: &FakeBootloader) -> Vec<[u8; 4]> {
        fake.sent()
            .iter()
            .filter(|report| report[0] == 0x05)
            .map(|report| report[..4].try_into().unwrap())
            .collect()
    }

    #[test]
    fn test_write_cycle() {
        let fake = FakeBootloader::new(SPEC);
        let device = ISPDevice::with_transport(SPEC, &fake, None);
        let mut firmware = isp_form_firmware();

        write_cycle(&device, &mut firmware).unwrap();

        let flash = fake.flash();
        assert_eq!(
            flash[..3],
            [0x02, 0xf0, 0x00],
            "bootloader keeps the reset vector"
        );
        assert_eq!(
            flash[FW - 5..FW - 2],
            [0x02, 0x00, 0x66],
            "firmware enabled through the relocated vector"
        );
        assert_eq!(
            commands(&fake),
            vec![
                [0x05, 0x45, 0x45, 0x45],
                [0x05, 0x57, 0x00, 0x00],
                [0x05, 0x52, 0x00, 0x00],
                [0x05, 0x55, 0x00, 0x00],
                [0x05, 0x5a, 0x00, 0x00],
            ]
        );
    }

    #[test]
    fn test_write_cycle_skips_reboot() {
        let spec = DeviceSpec {
            reboot: false,
            ..SPEC
        };
        let fake = FakeBootloader::new(spec);
        let device = ISPDevice::with_transport(spec, &fake, None);

        write_cycle(&device, &mut isp_form_firmware()).unwrap();

        assert_eq!(commands(&fake).last().unwrap()[1], 0x55);
    }

    #[test]
    fn test_write_cycle_fails_verification_on_bad_read_back() {
        fn identity(_offset: usize, byte: u8) -> u8 {
            byte
        }
        fn add_one(_offset: usize, byte: u8) -> u8 {
            byte.wrapping_add(1)
        }
        let spec = DeviceSpec {
            isp_transform: Some(IspTransform {
                read: identity,
                write: add_one,
            }),
            ..SPEC
        };
        let fake = FakeBootloader::new(spec);
        let device = ISPDevice::with_transport(spec, &fake, None);

        let result = write_cycle(&device, &mut isp_form_firmware());

        assert!(matches!(
            result,
            Err(ISPError::VerificationError(
                VerificationError::ByteMismatch { .. }
            ))
        ));
        assert!(
            !commands(&fake).iter().any(|cmd| cmd[1] == 0x55),
            "firmware must not be enabled after a failed verify"
        );
    }

    #[test]
    fn test_read_cycle_sections() {
        let flash: Vec<u8> = (0..SPEC.total_flash_size()).map(|i| i as u8).collect();
        for (section, addr, len) in [
            (ReadSection::Firmware, 0, FW),
            (ReadSection::Bootloader, FW, SPEC.platform.bootloader_size),
            (ReadSection::Full, 0, SPEC.total_flash_size()),
        ] {
            let fake = FakeBootloader::with_flash(SPEC, flash.clone());
            let device = ISPDevice::with_transport(SPEC, &fake, None);

            let result = read_cycle(&device, section.clone()).unwrap();

            let mut expected = flash[addr..addr + len].to_vec();
            if addr == 0 {
                expected[0] = 0x02;
                expected[1..3].copy_from_slice(&flash[FW - 4..FW - 2]);
                expected[FW - 5..FW - 2].fill(0);
            }
            assert_eq!(result, expected, "{section:?}");
            assert_eq!(
                commands(&fake),
                vec![
                    [0x05, 0x55, 0x00, 0x00],
                    [0x05, 0x52, addr as u8, (addr >> 8) as u8],
                    [0x05, 0x5a, 0x00, 0x00],
                ],
                "{section:?}"
            );
        }
    }
}
