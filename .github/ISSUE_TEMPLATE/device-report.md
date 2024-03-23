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
firmware_size: 61440
vendor_id: 0xdead
product_id: 0xcafe
bootloader_size: 4096 # necessary if not default, otherwise remove this line
page_size: 2048 # necessary if not default, otherwise remove this line
isp_usage_page: 0xff00 # necessary if not default, otherwise remove this line
isp_usage: 0x0001 # necessary if not default, otherwise remove this line
isp_index: 0 # necessary if not default, otherwise remove this line
```

## Operations Tested

- [ ] Read
- [ ] Write

## Platforms Tested

- [ ] linux
- [ ] macos
- [ ] windows

## Checksums

- Bootloader MD5: `beefcafebeefcafebeefcafebeefcafe`
- Stock Firmware MD5: `deadbeefdeadbeefdeadbeefdeadbeef`

## HID Dump

A dump from [usbhid-dump](https://github.com/DIGImend/usbhid-dump), [win-hid-dump](https://github.com/todbot/win-hid-dump) or [mac-hid-dump](https://github.com/todbot/mac-hid-dump)

<details>
<summary>HID Tool Output</summary>

```
# NuPhy Air60 using win-hid-dump
...
05AC:024F: BY Tech - Air60
PATH:\\?\hid#vid_05ac&pid_024f&mi_01&col05#7&2af01ac7&0&0004#{4d1e55b2-f16f-11cf-88cb-001111000030}
DESCRIPTOR:
  06  00  FF  09  01  A1  01  85  05  15  00  25  01  35  00  45
  01  65  00  55  00  75  01  95  28  B1  03  C1  00
  (29 bytes)
05AC:024F: BY Tech - Air60
PATH:\\?\hid#vid_05ac&pid_024f&mi_00#7&132c8e82&0&0000#{4d1e55b2-f16f-11cf-88cb-001111000030}\kbd
DESCRIPTOR:
  05  01  09  06  A1  01  05  07  19  E0  29  E7  15  00  25  01
  35  00  45  01  65  00  55  00  75  01  95  08  81  02  95  30
  81  03  05  FF  09  03  25  FF  45  00  75  08  95  01  81  02
  05  08  19  01  29  05  25  01  45  01  75  01  95  05  91  02
  95  03  91  03  05  C0  09  00  25  7F  45  00  75  08  95  40
  B1  02  C1  00
  (84 bytes)
...
```

</details>
