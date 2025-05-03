use std::fs;

use assert_cmd::Command;
use serial_test::serial;

#[macro_use]
pub mod common;

#[test]
#[serial]
fn test_read() {
    let file = test_filename!("hex");
    let mut cmd = Command::cargo_bin("sinowealth-kb-tool").unwrap();
    let assert = cmd
        .arg("read")
        .args(&["--part", "nuphy-air60"])
        .arg(&file)
        .assert();
    assert.success().stderr(predicates::str::contains(
        "MD5: 662c8707c4be0e0712e30336b0e7cfd1",
    ));

    let computed_md5 = md5::compute(fs::read(&file).unwrap());
    assert_eq!(
        format!("{:x}", computed_md5),
        "6594e5a1ab671deb40f36483a84ad61f"
    );
}

#[test]
#[serial]
fn test_read_bin() {
    let file = test_filename!("bin");
    let mut cmd = Command::cargo_bin("sinowealth-kb-tool").unwrap();
    let assert = cmd
        .arg("read")
        .args(&["--part", "nuphy-air60"])
        .arg(&file)
        .assert();
    assert.success().stderr(predicates::str::contains(
        "MD5: 662c8707c4be0e0712e30336b0e7cfd1",
    ));

    let computed_md5 = md5::compute(fs::read(&file).unwrap());
    assert_eq!(
        format!("{:x}", computed_md5),
        "662c8707c4be0e0712e30336b0e7cfd1"
    );
}

#[test]
#[serial]
fn test_read_bootloader() {
    let file = test_filename!("hex");
    let mut cmd = Command::cargo_bin("sinowealth-kb-tool").unwrap();
    let assert = cmd
        .arg("read")
        .args(&["--part", "nuphy-air60"])
        .args(&["--section", "bootloader"])
        .arg(&file)
        .assert();
    assert.success().stderr(predicates::str::contains(
        "MD5: 3e0ebd0c440af5236d7ff8872343f85d",
    ));

    let computed_md5 = md5::compute(fs::read(&file).unwrap());
    assert_eq!(
        format!("{:x}", computed_md5),
        "65956adbe2e77369d3581ebabb1592f7"
    );
}

#[test]
#[serial]
fn test_read_full() {
    let file = test_filename!("hex");
    let mut cmd = Command::cargo_bin("sinowealth-kb-tool").unwrap();
    let assert = cmd
        .arg("read")
        .args(&["--part", "nuphy-air60"])
        .args(&["--section", "full"])
        .arg(&file)
        .assert();
    assert.success().stderr(predicates::str::contains(
        "MD5: 3f87adb2125bcc5beee424a3af5272e9",
    ));

    let computed_md5 = md5::compute(fs::read(&file).unwrap());
    assert_eq!(
        format!("{:x}", computed_md5),
        "e6f497f108dbe82f8d340562eb88fe44"
    );
}

#[test]
#[serial]
fn test_read_custom_part() {
    let file = test_filename!("hex");
    let mut cmd = Command::cargo_bin("sinowealth-kb-tool").unwrap();
    let assert = cmd
        .arg("read")
        .args(&["--vendor_id", "0x05ac"])
        .args(&["--product_id", "0x024f"])
        .args(&["--isp_iface_num", "1"])
        .args(&["--isp_report_id", "5"])
        .args(&["--firmware_size", "61440"])
        .arg(&file)
        .assert();
    assert.success().stderr(predicates::str::contains(
        "MD5: 662c8707c4be0e0712e30336b0e7cfd1",
    ));

    let computed_md5 = md5::compute(fs::read(&file).unwrap());
    assert_eq!(
        format!("{:x}", computed_md5),
        "6594e5a1ab671deb40f36483a84ad61f"
    );
}

#[test]
#[serial]
fn test_read_forced_format_bin() {
    let file = test_filename!("hex");
    let mut cmd = Command::cargo_bin("sinowealth-kb-tool").unwrap();
    let assert = cmd
        .arg("read")
        .args(&["--part", "nuphy-air60"])
        .args(&["--format", "bin"])
        .arg(&file)
        .assert();
    assert
        .success()
        .stderr(predicates::str::contains(
            "Warning: binary file has hex extension. This might be unintended.",
        ))
        .stderr(predicates::str::contains(
            "MD5: 662c8707c4be0e0712e30336b0e7cfd1",
        ));

    let computed_md5 = md5::compute(fs::read(&file).unwrap());
    assert_eq!(
        format!("{:x}", computed_md5),
        "662c8707c4be0e0712e30336b0e7cfd1"
    );
}
