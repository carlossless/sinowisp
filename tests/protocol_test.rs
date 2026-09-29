use std::{cell::RefCell, str::FromStr};

use hidra::MaybeFuture;

use sinowisp::{DeviceSpec, ISPDevice, ISPError, IspTransform, ReadSection, DEVICE_BASE_SH68F90};
use sinowisp_testing::FakeBootloader;

const SPEC: DeviceSpec = DEVICE_BASE_SH68F90;
const PAGE: usize = SPEC.platform.page_size;

fn add_one(_offset: usize, byte: u8) -> u8 {
    byte.wrapping_add(1)
}

fn xor_offset(offset: usize, byte: u8) -> u8 {
    byte ^ offset as u8
}

const TRANSFORMED: DeviceSpec = DeviceSpec {
    isp_transform: Some(IspTransform {
        read: xor_offset,
        write: add_one,
    }),
    ..SPEC
};

fn pattern(len: usize) -> Vec<u8> {
    (0..len).map(|i| (i * 7 + i / 256) as u8).collect()
}

#[test]
fn test_commands() {
    let fake = FakeBootloader::new(SPEC);
    let device = ISPDevice::with_transport(SPEC, &fake, None);

    device.enable_firmware().wait().unwrap();
    device.init_read(0x1234).wait().unwrap();
    device.init_write(0xf000).wait().unwrap();
    device.erase().wait().unwrap();
    device.reboot().wait().unwrap();

    assert_eq!(
        fake.sent(),
        vec![
            vec![0x05, 0x55, 0x00, 0x00, 0x00, 0x00],
            vec![0x05, 0x52, 0x34, 0x12, 0x00, 0x00],
            vec![0x05, 0x57, 0x00, 0xf0, 0x00, 0x00],
            vec![0x05, 0x45, 0x45, 0x45, 0x45, 0x45],
            vec![0x05, 0x5a, 0x00, 0x00, 0x00, 0x00],
        ]
    );
}

#[test]
fn test_read() {
    let flash = pattern(SPEC.total_flash_size());
    let fake = FakeBootloader::with_flash(SPEC, flash.clone());
    let device = ISPDevice::with_transport(SPEC, &fake, None);
    let progress = RefCell::new(vec![]);

    let result = device
        .read(0x800, 3 * PAGE, &|done, total| {
            progress.borrow_mut().push((done, total))
        })
        .wait()
        .unwrap();

    assert_eq!(result, flash[0x800..0x800 + 3 * PAGE]);
    assert_eq!(*progress.borrow(), vec![(1, 3), (2, 3), (3, 3)]);
    assert_eq!(fake.sent(), vec![vec![0x05, 0x52, 0x00, 0x08, 0x00, 0x00]]);
}

#[test]
fn test_read_page_rejects_wrong_transfer_type() {
    let fake = FakeBootloader::new(SPEC);
    fake.set_read_type(0x77);
    let device = ISPDevice::with_transport(SPEC, &fake, None);

    let result = device.read_page(&mut vec![]).wait();

    assert!(matches!(result, Err(ISPError::ReadWriteMismatch)));
}

#[test]
fn test_read_page_applies_isp_transform() {
    let flash = pattern(SPEC.total_flash_size());
    let fake = FakeBootloader::with_flash(SPEC, flash.clone());
    let device = ISPDevice::with_transport(TRANSFORMED, &fake, None);

    device.init_read(0x800).wait().unwrap();
    let mut page = vec![];
    device.read_page(&mut page).wait().unwrap();

    let expected: Vec<u8> = flash[0x800..0x800 + PAGE]
        .iter()
        .enumerate()
        .map(|(i, b)| xor_offset(i, *b))
        .collect();
    assert_eq!(page, expected);
}

#[test]
fn test_write() {
    let firmware = pattern(SPEC.platform.firmware_size);
    let fake = FakeBootloader::new(SPEC);
    let device = ISPDevice::with_transport(SPEC, &fake, None);
    let progress = RefCell::new(vec![]);

    device
        .write(0, &firmware, &|done, _total| {
            progress.borrow_mut().push(done)
        })
        .wait()
        .unwrap();

    let sent = fake.sent();
    assert_eq!(sent[0], vec![0x05, 0x57, 0x00, 0x00, 0x00, 0x00]);
    assert_eq!(sent.len(), 1 + SPEC.num_pages());
    for (i, report) in sent[1..].iter().enumerate() {
        assert_eq!(report[..2], [0x06, 0x77]);
        assert_eq!(report[2..], firmware[i * PAGE..(i + 1) * PAGE]);
    }
    assert_eq!(
        *progress.borrow(),
        (1..=SPEC.num_pages()).collect::<Vec<_>>()
    );
}

#[test]
fn test_write_page_applies_isp_transform() {
    let fake = FakeBootloader::new(SPEC);
    let device = ISPDevice::with_transport(TRANSFORMED, &fake, None);

    device.write_page(&[0x00, 0x7f, 0xff]).wait().unwrap();

    assert_eq!(fake.sent(), vec![vec![0x06, 0x77, 0x01, 0x80, 0x00]]);
}

#[test]
fn test_transfers_use_xfer_handle() {
    let cmd = FakeBootloader::new(SPEC);
    let xfer = FakeBootloader::with_flash(SPEC, vec![0xaa; SPEC.total_flash_size()]);
    let device = ISPDevice::with_transport(SPEC, &cmd, Some(&xfer));

    device.init_read(0x800).wait().unwrap();
    let mut page = vec![];
    device.read_page(&mut page).wait().unwrap();
    device.write_page(&[1, 2, 3]).wait().unwrap();

    assert!(page[1..].iter().all(|b| *b == 0xaa));
    assert_eq!(cmd.sent(), vec![vec![0x05, 0x52, 0x00, 0x08, 0x00, 0x00]]);
    assert_eq!(xfer.sent(), vec![vec![0x06, 0x77, 1, 2, 3]]);
}

#[test]
fn test_read_section_round_trip() {
    for name in ReadSection::available_sections() {
        assert_eq!(ReadSection::from_str(name).unwrap().to_str(), name);
    }
}
