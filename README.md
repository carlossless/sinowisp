# sinowealth-kb-tool

[![crate](https://img.shields.io/crates/v/sinowealth-kb-tool.svg)](https://crates.io/crates/sinowealth-kb-tool) [![ci](https://github.com/carlossless/sinowealth-kb-tool/actions/workflows/push.yml/badge.svg)](https://github.com/carlossless/sinowealth-kb-tool/actions/workflows/push.yml)

A utility for reading and writing flash contents on Sinowealth 8051-based devices (keyboards and mice) since they all seem to have similar ISP bootloaders.

## Disclaimer

This is an experimental tool, so use it at your own risk.

## Supported Hardware

| Keyboard | ISP MD5 | MCU | MCU Label | Tested Read | Tested Write |
| -------- | ------- | --- | --------- | ----------- | ------------ |
| [NuPhy Air60](https://nuphy.com/products/air60) | 3e0ebd0c440af5236d7ff8872343f85d | SH68F90A | BYK916 | ✅ | ✅ |
| [NuPhy Air75](https://nuphy.com/products/air75) | 3e0ebd0c440af5236d7ff8872343f85d | SH68F90A | BYK916 | ✅ | ❓ |
| [NuPhy Air96](https://nuphy.com/products/air96-wireless-mechanical-keyboard) | 3e0ebd0c440af5236d7ff8872343f85d | SH68F90A | BYK916 | ✅ | ❓ |
| [NuPhy Halo65](https://nuphy.com/products/halo65) | 3e0ebd0c440af5236d7ff8872343f85d | SH68F90A | BYK916 | ✅ | ❓ |
| Terport TR95 | 2d169670eae0d36eae8188562c1f66e8 | SH68F90A | BYK916 | ✅ | ❓ |
| Xinmeng K916 | cfc8661da8c9d7e351b36c0a763426aa | SH68F90 | ❓ | ✅ | ✅ |
| Hykker X Range 2017 (RE-K70-BYK800) | 13df4ce2933f9654ffef80d6a3c27199 | SH68F881 | BYK801 | ✅ | ❓ |

## Prerequisites

### Linux

To enable running this tool without superuser privileges add the following udev rule with `xxxx` and `yyyy` replaced with your device Vendor ID and Product ID respectively.

```udev
# /etc/udev/rules.d/plugdev.rule
SUBSYSTEMS=="usb", ATTRS{idVendor}=="xxxx", ATTRS{idProduct}=="yyyy", MODE="0660", GROUP="plugdev"
SUBSYSTEMS=="usb", ATTRS{idVendor}=="0603", ATTRS{idProduct}=="1020", MODE="0660", GROUP="plugdev"
```

Make sure your user is part of the `plugdev` group.

## Acknowledgments

* https://github.com/gashtaan/sinowealth-8051-dumper
* https://github.com/ayufan-rock64/pinebook-pro-keyboard-updater
