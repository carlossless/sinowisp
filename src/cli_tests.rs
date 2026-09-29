use std::fs;

use sinowisp::DEVICES;
use sinowisp_testing::FakeBootloader;

use super::*;
use crate::fake_hid::{FakeDevice, FakeHid};

fn nuphy_air60() -> DeviceSpec {
    *DEVICES.get("nuphy-air60").unwrap()
}

fn temp_path(name: &str) -> String {
    std::env::temp_dir()
        .join(format!("sinowisp-{}-{name}", std::process::id()))
        .display()
        .to_string()
}

fn run_args(args: &[&str], hid: FakeHid) -> Result<(), CLIError> {
    let matches = cli().get_matches_from(std::iter::once("sinowisp").chain(args.iter().copied()));
    run(&matches, || Ok(DeviceSelector::with_backend(hid)))
}

fn isp_form_firmware(len: usize) -> Vec<u8> {
    let fw = nuphy_air60().platform.firmware_size;
    let mut firmware: Vec<u8> = (0..len).map(|i| (i * 7 + i / 256) as u8).collect();
    firmware[..3].copy_from_slice(&[0x02, 0x00, 0x66]);
    if len == fw {
        firmware[fw - 5..fw - 2].fill(0);
    }
    firmware
}

#[test]
fn test_cli_definition() {
    cli().debug_assert();
}

#[test]
fn test_read() {
    let spec = nuphy_air60();
    let fw = spec.platform.firmware_size;
    let flash: Vec<u8> = (0..spec.total_flash_size())
        .map(|i| (i % 251) as u8)
        .collect();
    let bootloader = FakeBootloader::with_flash(spec, flash.clone());
    let hid = FakeHid::new(&bootloader, vec![FakeDevice::isp()], vec![]);
    let bin = temp_path("read.bin");

    run_args(&["read", "--device", "nuphy-air60", "-r", "1", &bin], hid).unwrap();

    let read = fs::read(&bin).unwrap();
    assert_eq!(read.len(), fw);
    assert_eq!(read[3..fw - 5], flash[3..fw - 5]);
}

#[test]
fn test_read_full_section_as_ihex() {
    let spec = nuphy_air60();
    let bootloader = FakeBootloader::new(spec);
    let hid = FakeHid::new(&bootloader, vec![FakeDevice::isp()], vec![]);
    let hex = temp_path("read-full.hex");

    run_args(
        &[
            "read",
            "--device",
            "nuphy-air60",
            "-r",
            "1",
            "-s",
            "full",
            &hex,
        ],
        hid,
    )
    .unwrap();

    let read = from_ihex(&fs::read_to_string(&hex).unwrap(), 0x10000).unwrap();
    assert_eq!(read.len(), spec.total_flash_size());
}

#[test]
fn test_write() {
    let spec = nuphy_air60();
    let fw = spec.platform.firmware_size;
    let firmware = isp_form_firmware(fw);
    let bin = temp_path("write.bin");
    fs::write(&bin, &firmware).unwrap();
    let bootloader = FakeBootloader::new(spec);
    let hid = FakeHid::new(&bootloader, vec![FakeDevice::isp()], vec![]);

    run_args(&["write", "--device", "nuphy-air60", "-r", "1", &bin], hid).unwrap();

    let flash = bootloader.flash();
    assert_eq!(flash[3..fw - 5], firmware[3..fw - 5]);
    assert_eq!(flash[fw - 5..fw - 2], [0x02, 0x00, 0x66]);
}

#[test]
fn test_write_pads_short_firmware_when_forced() {
    let spec = nuphy_air60();
    let fw = spec.platform.firmware_size;
    let firmware = isp_form_firmware(0x100);
    let bin = temp_path("write-short.bin");
    fs::write(&bin, &firmware).unwrap();
    let bootloader = FakeBootloader::new(spec);
    let hid = FakeHid::new(&bootloader, vec![FakeDevice::isp()], vec![]);

    run_args(
        &[
            "write",
            "--device",
            "nuphy-air60",
            "-r",
            "1",
            "--force",
            &bin,
        ],
        hid,
    )
    .unwrap();

    let flash = bootloader.flash();
    assert_eq!(flash[3..0x100], firmware[3..]);
    assert!(flash[0x100..fw - 5].iter().all(|b| *b == 0));
}

#[test]
fn test_write_reports_missing_device() {
    let bootloader = FakeBootloader::new(nuphy_air60());
    let bin = temp_path("write-missing.bin");
    fs::write(&bin, isp_form_firmware(0x100)).unwrap();

    let result = run_args(
        &[
            "write",
            "--device",
            "nuphy-air60",
            "-r",
            "1",
            "--force",
            &bin,
        ],
        FakeHid::new(&bootloader, vec![], vec![]),
    );

    assert!(matches!(
        result,
        Err(CLIError::DeviceSelectorError(DeviceSelectorError::NotFound))
    ));
}

#[test]
fn test_convert_round_trip() {
    let bootloader = FakeBootloader::new(nuphy_air60());
    let fw = nuphy_air60().platform.firmware_size;
    let isp = isp_form_firmware(fw);
    let isp_path = temp_path("convert-isp.bin");
    let jtag_path = temp_path("convert-jtag.hex");
    let back_path = temp_path("convert-back.bin");
    fs::write(&isp_path, &isp).unwrap();

    run_args(
        &[
            "convert",
            "--platform",
            "sh68f90",
            "--direction",
            "to_jtag",
            &isp_path,
            &jtag_path,
        ],
        FakeHid::new(&bootloader, vec![], vec![]),
    )
    .unwrap();
    run_args(
        &[
            "convert",
            "--platform",
            "sh68f90",
            "--direction",
            "to_isp",
            &jtag_path,
            &back_path,
        ],
        FakeHid::new(&bootloader, vec![], vec![]),
    )
    .unwrap();

    let jtag = from_ihex(&fs::read_to_string(&jtag_path).unwrap(), 0x10000).unwrap();
    assert_eq!(jtag[..3], [0x02, 0xf0, 0x00]);
    assert_eq!(fs::read(&back_path).unwrap(), isp);
}

#[test]
fn test_convert_honours_format_flags_over_extensions() {
    let bootloader = FakeBootloader::new(nuphy_air60());
    let input = temp_path("flags-input.hex");
    let output = temp_path("flags-output.bin");
    fs::write(&input, isp_form_firmware(0x100)).unwrap();

    run_args(
        &[
            "convert",
            "--device",
            "nuphy-air60",
            "--direction",
            "to_jtag",
            "--input_format",
            "bin",
            "--output_format",
            "ihex",
            &input,
            &output,
        ],
        FakeHid::new(&bootloader, vec![], vec![]),
    )
    .unwrap();

    assert!(fs::read_to_string(&output)
        .unwrap()
        .ends_with(":00000001FF\n"));
}

#[test]
fn test_convert_reports_missing_input() {
    let bootloader = FakeBootloader::new(nuphy_air60());

    let result = run_args(
        &[
            "convert",
            "--platform",
            "sh68f90",
            "--direction",
            "to_isp",
            &temp_path("does-not-exist.bin"),
            &temp_path("unused.bin"),
        ],
        FakeHid::new(&bootloader, vec![], vec![]),
    );

    assert!(matches!(result, Err(CLIError::IOError(_))));
}

#[test]
fn test_convert_full_jtag_image_to_isp() {
    let bootloader = FakeBootloader::new(nuphy_air60());
    let mut jtag = vec![0u8; 0x10000];
    jtag[..3].copy_from_slice(&[0x02, 0xf0, 0x00]);
    jtag[0xeffb..0xeffe].copy_from_slice(&[0x02, 0x00, 0x66]);
    let input = temp_path("full-jtag.bin");
    let output = temp_path("full-isp.bin");
    fs::write(&input, &jtag).unwrap();

    run_args(
        &[
            "convert",
            "--platform",
            "sh68f90",
            "--direction",
            "to_isp",
            &input,
            &output,
        ],
        FakeHid::new(&bootloader, vec![], vec![]),
    )
    .unwrap();

    let isp = fs::read(&output).unwrap();
    assert_eq!(isp.len(), 0x10000);
    assert_eq!(isp[..3], [0x02, 0x00, 0x66]);
    assert_eq!(isp[0xeffb..0xeffe], [0x00, 0x00, 0x00]);
}

#[test]
fn test_convert_rejects_payload_without_reset_vector() {
    let bootloader = FakeBootloader::new(nuphy_air60());
    let input = temp_path("no-vector.bin");
    fs::write(&input, [0u8; 0x100]).unwrap();

    let result = run_args(
        &[
            "convert",
            "--platform",
            "sh68f90",
            "--direction",
            "to_jtag",
            &input,
            &temp_path("no-vector-out.bin"),
        ],
        FakeHid::new(&bootloader, vec![], vec![]),
    );

    assert!(matches!(result, Err(CLIError::PayloadConversionError(_))));
}

#[test]
fn test_list() {
    let bootloader = FakeBootloader::new(nuphy_air60());
    let devices = vec![
        FakeDevice::new("kbd", 0x05ac, 0x024f, 0),
        FakeDevice::new("other-kbd", 0x05ac, 0x0250, 0),
        FakeDevice::new("mouse", 0x046d, 0xc52b, 0),
    ];

    run_args(
        &["list", "--vendor_id", "0x05ac", "--product_id", "0x024f"],
        FakeHid::new(&bootloader, devices, vec![]),
    )
    .unwrap();
}

#[test]
fn test_device_spec_overrides() {
    let matches = cli().get_matches_from([
        "sinowisp",
        "read",
        "--platform",
        "sh68f90",
        "--vendor_id",
        "0x1234",
        "--product_id",
        "0x5678",
        "--firmware_size",
        "0x8000",
        "--bootloader_size",
        "0x800",
        "--page_size",
        "0x400",
        "--isp_iface_num",
        "2",
        "--isp_report_id",
        "6",
        "--reboot",
        "false",
        "out.bin",
    ]);
    let (_, sub_matches) = matches.subcommand().unwrap();

    let spec = get_device_spec_from_matches(sub_matches);

    assert_eq!((spec.vendor_id, spec.product_id), (0x1234, 0x5678));
    assert_eq!(spec.platform.firmware_size, 0x8000);
    assert_eq!(spec.platform.bootloader_size, 0x800);
    assert_eq!(spec.platform.page_size, 0x400);
    assert_eq!((spec.isp_iface_num, spec.isp_report_id), (2, 6));
    assert!(!spec.reboot);
}
