use std::fs;

use assert_cmd::Command;
use serial_test::serial;

#[macro_use]
pub mod common;

use common::get_fixture_path;

#[test]
#[serial]
fn test_convert_to_jtag() {
    let input_file = get_fixture_path("nuphy-air60_smk.hex");
    let output_file = test_filename!("hex");
    let mut cmd = Command::cargo_bin("sinowealth-kb-tool").unwrap();
    let assert = cmd
        .arg("convert")
        .args(&["--device", "nuphy-air60"])
        .args(&["--direction", "to_jtag"])
        .arg(&input_file)
        .arg(&output_file)
        .assert();

    assert.success();

    let computed_md5 = md5::compute(fs::read(&output_file).unwrap());
    assert_eq!(
        format!("{:x}", computed_md5),
        "3bbd99f81678fc11fdf1ba9eaaac2bd1"
    );
}

#[test]
#[serial]
fn test_convert_to_isp() {
    let input_file = get_fixture_path("nuphy-air60_smk_jtag.hex");
    let output_file = test_filename!("hex");
    let mut cmd = Command::cargo_bin("sinowealth-kb-tool").unwrap();
    let assert = cmd
        .arg("convert")
        .args(&["--device", "nuphy-air60"])
        .args(&["--direction", "to_isp"])
        .arg(&input_file)
        .arg(&output_file)
        .assert();

    assert.success();

    let computed_md5 = md5::compute(fs::read(&output_file).unwrap());
    assert_eq!(
        format!("{:x}", computed_md5),
        "6594e5a1ab671deb40f36483a84ad61f"
    );
}

#[test]
#[serial]
fn test_convert_to_jtag_bin() {
    let input_file = get_fixture_path("nuphy-air60_smk.hex");
    let output_file = test_filename!("bin");
    let mut cmd = Command::cargo_bin("sinowealth-kb-tool").unwrap();
    let assert = cmd
        .arg("convert")
        .args(&["--device", "nuphy-air60"])
        .args(&["--direction", "to_jtag"])
        .arg(&input_file)
        .arg(&output_file)
        .assert();

    assert.success();

    let computed_md5 = md5::compute(fs::read(&output_file).unwrap());
    assert_eq!(
        format!("{:x}", computed_md5),
        "df1ff7b247ae12dda37aa69730f090af"
    );
}
