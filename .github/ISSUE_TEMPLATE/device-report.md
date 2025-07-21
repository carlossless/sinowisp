---
name: Device Report
about: For reporting operation with a particular device
title: "[device-report] Manufacturer Model"
labels: device-report
assignees: ''

---

## Device Info

- Sinowealth Device: _example `SH68F90A`_
- IC Label: _example `BYK916`_
- Product Page: _example https://nuphy.com/products/air60_

## Part Info

```
plarform: sh68f90
vendor_id: 0xdead
product_id: 0xcafe
firmware_size: 61440 # necessary if not default, otherwise remove this line
bootloader_size: 4096 # necessary if not default, otherwise remove this line
page_size: 2048 # necessary if not default, otherwise remove this line
isp_iface_num: 1 # necessary if not default, otherwise remove this line
isp_report_id: 5 # necessary if not default, otherwise remove this line
reboot: false # necessary if not default, otherwise remove this line
```

## Operations Tested

- [ ] Read
- [ ] Write

## Platforms Tested

- [ ] linux
- [ ] macos
- [ ] windows

## Checksums

- Stock Firmware MD5: `deadbeefdeadbeefdeadbeefdeadbeef`
- Bootloader MD5: `beefcafebeefcafebeefcafebeefcafe` _(shown when running `sinowealth-kb-tool read -s bootloader ...`)_

## Device Info (HID Reports)

Output when running `sinowealth-kb-tool list --vendor_id=<PID> --product_id=<PID>`

<details>
<summary>Output</summary>

```
sinowealth-kb-tool list --vendor_id=0x05ac --product_id=0x024f
ID 05ac:024f manufacturer="contact@carlossless.io" product="SMK Keyboard"
    path="DevSrvsID:4294974930" interface_number=0
    report_descriptor=[05 01 09 06 A1 01 05 07 19 E0 29 E7 15 00 25 01 75 01 95 08 81 02 75 08 95 01 81 01 05 07 19 00 29 FF 15 00 26 FF 00 75 08 95 06 81 00 05 08 19 01 29 05 15 00 25 01 75 01 95 05 91 02 75 03 95 01 91 01 C0]
    feature_report_ids=[]
        usage_page=0x0001 usage=0x0006
    path="DevSrvsID:4294974929" interface_number=1
    report_descriptor=[05 01 09 80 A1 01 85 01 19 81 29 83 15 00 25 01 75 01 95 03 81 02 95 05 81 01 C0 05 0C 09 01 A1 01 85 02 19 00 2A 3C 02 15 00 26 3C 02 75 10 95 01 81 00 C0 06 00 FF 09 01 A1 01 85 05 19 01 29 02 15 00 26 FF 00 75 08 95 05 B1 02 C0 05 01 09 06 A1 01 85 06 05 07 19 E0 29 E7 15 00 25 01 75 01 95 08 81 02 05 07 19 00 29 9F 15 00 25 01 75 01 95 A0 81 02 C0]
    feature_report_ids=[5]
        usage_page=0x0001 usage=0x0006
        usage_page=0x0001 usage=0x0080
        usage_page=0x000c usage=0x0001
        usage_page=0xff00 usage=0x0001
```

</details>

## PCB Photos

_If possible, include photos of your device PCB clearly showing MCU and wireless IC labels_
