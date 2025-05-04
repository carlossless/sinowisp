use assert_cmd::Command;
use serial_test::serial;

#[cfg(target_os = "macos")]
const NUPHY_AIR60_SMK_ENTRY: &str = "\
ID 05ac:024f manufacturer=\"contact@carlossless.io\" product=\"SMK Keyboard\"
    path=\".+\" interface_number=0
    report_descriptor=\\[05 01 09 06 A1 01 05 07 19 E0 29 E7 15 00 25 01 75 01 95 08 81 02 75 08 95 01 81 01 05 07 19 00 29 FF 15 00 26 FF 00 75 08 95 06 81 00 05 08 19 01 29 05 15 00 25 01 75 01 95 05 91 02 75 03 95 01 91 01 C0\\]
    feature_report_ids=\\[\\]
        usage_page=0x0001 usage=0x0006
    path=\".+\" interface_number=1
    report_descriptor=\\[05 01 09 80 A1 01 85 01 19 81 29 83 15 00 25 01 75 01 95 03 81 02 95 05 81 01 C0 05 0C 09 01 A1 01 85 02 19 00 2A 3C 02 15 00 26 3C 02 75 10 95 01 81 00 C0 06 00 FF 09 01 A1 01 85 05 19 01 29 02 15 00 26 FF 00 75 08 95 05 B1 02 C0 05 01 09 06 A1 01 85 06 05 07 19 E0 29 E7 15 00 25 01 75 01 95 08 81 02 05 07 19 00 29 9F 15 00 25 01 75 01 95 A0 81 02 C0\\]
    feature_report_ids=\\[5\\]
        usage_page=0x0001 usage=0x0006
        usage_page=0x0001 usage=0x0080
        usage_page=0x000c usage=0x0001
        usage_page=0xff00 usage=0x0001
";

#[cfg(target_os = "linux")]
const NUPHY_AIR60_SMK_ENTRY: &str = "\
ID 05ac:024f manufacturer=\"contact@carlossless.io\" product=\"SMK Keyboard\"
    path=\".+\" interface_number=0
    report_descriptor=\\[05 01 09 06 A1 01 05 07 19 E0 29 E7 15 00 25 01 75 01 95 08 81 02 75 08 95 01 81 01 05 07 19 00 29 FF 15 00 26 FF 00 75 08 95 06 81 00 05 08 19 01 29 05 15 00 25 01 75 01 95 05 91 02 75 03 95 01 91 01 C0\\]
    feature_report_ids=\\[\\]
    path=\".+\" interface_number=1
    report_descriptor=\\[05 01 09 80 A1 01 85 01 19 81 29 83 15 00 25 01 75 01 95 03 81 02 95 05 81 01 C0 05 0C 09 01 A1 01 85 02 19 00 2A 3C 02 15 00 26 3C 02 75 10 95 01 81 00 C0 06 00 FF 09 01 A1 01 85 05 19 01 29 02 15 00 26 FF 00 75 08 95 05 B1 02 C0 05 01 09 06 A1 01 85 06 05 07 19 E0 29 E7 15 00 25 01 75 01 95 08 81 02 05 07 19 00 29 9F 15 00 25 01 75 01 95 A0 81 02 C0\\]
    feature_report_ids=\\[5\\]
";

#[cfg(target_os = "windows")]
const NUPHY_AIR60_SMK_ENTRY: &str = "\
ID 05ac:024f manufacturer=\"contact@carlossless.io\" product=\"SMK Keyboard\"
    interface_number=0
        path=\".+\" usage_page=0x0001 usage=0x0006
        report_descriptor=\\[05 01 09 06 A1 01 05 07 19 E0 29 E7 15 00 25 01 75 01 95 08 81 02 75 08 95 01 81 03 19 00 29 FF 15 00 26 FF 00 75 08 95 06 81 00 05 08 19 01 29 05 15 00 25 01 75 01 95 05 91 02 75 03 95 01 91 03 C0\\]
        feature_report_ids=\\[\\]
    interface_number=1
        path=\".+\" usage_page=0x0001 usage=0x0080
        report_descriptor=\\[05 01 09 80 A1 01 85 01 19 81 29 83 15 00 25 01 75 01 95 03 81 02 75 05 95 01 81 03 C0\\]
        feature_report_ids=\\[\\]
    interface_number=1
        path=\".+\" usage_page=0x000c usage=0x0001
        report_descriptor=\\[05 0C 09 01 A1 01 85 02 19 00 2A 3C 02 15 00 26 3C 02 75 10 95 01 81 00 C0\\]
        feature_report_ids=\\[\\]
    interface_number=1
        path=\".+\" usage_page=0xff00 usage=0x0001
        report_descriptor=\\[06 00 FF 09 01 A1 01 85 05 19 01 29 02 15 00 26 FF 00 75 08 95 05 B1 02 C0\\]
        feature_report_ids=\\[5\\]
    interface_number=1
        path=\".+\" usage_page=0x0001 usage=0x0006
        report_descriptor=\\[05 01 09 06 A1 01 85 06 05 07 19 E0 29 E7 15 00 25 01 75 01 95 08 81 02 19 00 29 9F 15 00 25 01 75 01 95 A0 81 02 C0\\]
        feature_report_ids=\\[\\]
";

#[test]
#[serial]
fn test_list_devices() {
    let mut cmd = Command::cargo_bin("sinowealth-kb-tool").unwrap();
    let assert = cmd.arg("list").assert();
    assert
        .success()
        .stdout(predicates::str::is_match(NUPHY_AIR60_SMK_ENTRY).unwrap());
}

#[test]
#[serial]
fn test_list_with_vid_filter() {
    let mut cmd = Command::cargo_bin("sinowealth-kb-tool").unwrap();
    let assert = cmd.arg("list").args(&["--vendor_id", "0x05ac"]).assert();
    assert
        .success()
        .stdout(predicates::str::is_match(NUPHY_AIR60_SMK_ENTRY).unwrap());
}

#[test]
#[serial]
fn test_list_with_pid_filter() {
    let mut cmd = Command::cargo_bin("sinowealth-kb-tool").unwrap();
    let assert = cmd.arg("list").args(&["--product_id", "0x024f"]).assert();
    assert
        .success()
        .stdout(predicates::str::is_match(NUPHY_AIR60_SMK_ENTRY).unwrap());
}

#[test]
#[serial]
fn test_list_with_vid_and_pid_filter() {
    let mut cmd = Command::cargo_bin("sinowealth-kb-tool").unwrap();
    let assert = cmd
        .arg("list")
        .args(&["--vendor_id", "0x05ac"])
        .args(&["--product_id", "0x024f"])
        .assert();
    assert
        .success()
        .stdout(predicates::str::is_match(NUPHY_AIR60_SMK_ENTRY).unwrap());
}
