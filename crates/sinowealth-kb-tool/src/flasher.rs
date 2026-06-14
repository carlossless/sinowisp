//! Full read/write cycles, composed from the `sinowealth-isp` protocol
//! primitives. This is where the orchestration lives: page loops, the
//! post-erase/reboot settle delays, terminal progress bars, and verification.

use std::{thread, time::Duration};

use hidra::MaybeFuture;
use indicatif::ProgressBar;
use log::{debug, error};
use sinowealth_isp::{is_expected_error, verify, ISPDevice, ISPError, ReadSection};

/// Time the device needs to settle after an erase or reboot before it will
/// accept (or has finished acting on) further commands.
const SETTLE_DELAY: Duration = Duration::from_millis(2000);

pub fn read_cycle(device: &ISPDevice, section: ReadSection) -> Result<Vec<u8>, ISPError> {
    let spec = *device.device_spec();

    eprintln!("Enabling firmware...");
    device.enable_firmware().wait()?;

    let (start_addr, length) = match section {
        ReadSection::Firmware => (0, spec.platform.firmware_size),
        ReadSection::Bootloader => (spec.platform.firmware_size, spec.platform.bootloader_size),
        ReadSection::Full => (
            0,
            spec.platform.firmware_size + spec.platform.bootloader_size,
        ),
    };

    let firmware = read(device, start_addr, length)?;

    if spec.reboot {
        reboot(device);
    }

    Ok(firmware)
}

pub fn write_cycle(device: &ISPDevice, firmware: &mut [u8]) -> Result<(), ISPError> {
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

fn read(device: &ISPDevice, start_addr: usize, length: usize) -> Result<Vec<u8>, ISPError> {
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

fn write(device: &ISPDevice, start_addr: usize, buffer: &[u8]) -> Result<(), ISPError> {
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

fn erase(device: &ISPDevice) -> Result<(), ISPError> {
    eprintln!("Erasing...");
    device.erase().wait()?;
    thread::sleep(SETTLE_DELAY);
    Ok(())
}

fn reboot(device: &ISPDevice) {
    eprintln!("Rebooting...");
    if let Err(err) = device.reboot().wait() {
        debug!("Error: {:}", err);
        let expected = matches!(&err, ISPError::HidError(hid) if is_expected_error(hid));
        if !expected {
            error!("Unexpected error: {:}", err);
        }
    }
    thread::sleep(SETTLE_DELAY);
}
