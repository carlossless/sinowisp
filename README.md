# sinowealth-kb-tool

[![crate](https://img.shields.io/crates/v/sinowealth-kb-tool.svg)](https://crates.io/crates/sinowealth-kb-tool) [![ci](https://github.com/carlossless/sinowealth-kb-tool/actions/workflows/push.yml/badge.svg)](https://github.com/carlossless/sinowealth-kb-tool/actions/workflows/push.yml)

A utility for reading and writing flash contents on Sinowealth 8051-based devices (keyboards and mice) since they all seem to have similar ISP bootloaders.

## Disclaimer

This is an experimental tool, so use it at your own risk.

## Usage

### Reading

⚠️ A read operation will set an LJMP (0x02) opcode at address `<firmware_size-5>` if it's not already present there. When this opcode is set, the bootloader considers the main firmware enabled and jumps to it when the device is powered on. This opcode should already be set on most devices and therefore the read operation **should** not cause any issues.

⚠️ During reading the ISP bootloader will redirect values in `0x0001 - 0x0002` to `<firmware_size-4> - <firmware_size-3>`. Because of this, the produced payload will be different from how memory is actually laid out in the MCU flash.

```sh
# reads firmware excluding isp bootloader 
sinowealth-kb-tool read -p nuphy-air60 foobar.hex

# reads only isp bootloader section
sinowealth-kb-tool read -p nuphy-air60 -b bootloader.hex

# full dump including firmware and bootloader
sinowealth-kb-tool read -p nuphy-air60 --full full.hex

# custom device
sinowealth-kb-tool read \
    --vendor_id 0x05ac \
    --product_id 0x024f \
    --firmware_size 61440 \
    --bootloader_size 4096 \ # optional
    --page_size 2048 \ # optional
    --isp_usage_page 0xff00 \ # optional
    --isp_usage 0x0001 \ # optional
    --isp_index 0 \ # optional
    foobar.hex
```

### Writing

⚠️ Same as the [read](#reading) operation, the ISP bootloader will write values meant for addresses `0x0001-0x0002` to `<firmware_size-4> - <firmware_size-3>`. 

```sh
# overwrites firmware (does not touch the bootloader section)
sinowealth-kb-tool write -p nuphy-air60 foobar.hex

# custom device
sinowealth-kb-tool write \
    --vendor_id 0x05ac \
    --product_id 0x024f \
    --firmware_size 61440 \
    --bootloader_size 4096 \ # optional
    --page_size 2048 \ # optional
    --isp_usage_page 0xff00 \ # optional
    --isp_usage 0x0001 \ # optional
    --isp_index 0 \ # optional
    foobar.hex
```

## Supported Hardware

| Keyboard | ISP MD5 | MCU | MCU Label | Tested Read | Tested Write |
| -------- | ------- | --- | --------- | ----------- | ------------ |
| [Genesis Thor 300 RGB](https://genesis-zone.com/product/thor-300-rgb-brown) | 2d169670eae0d36eae8188562c1f66e8 | SH68F90 | SH68F90S | ✅ | ✅ |
| [Genesis Thor 300](https://genesis-zone.com/product/thor-300-outemu-blue) | e57490acebcaabfcff84a0ff013955d9 | SH68F881 | SH68F881W | ✅ | ✅ |
| Hykker X Range 2017 (RE-K70-BYK800) | 13df4ce2933f9654ffef80d6a3c27199 | SH68F881 | BYK801 | ✅ | ❓ |
| [NuPhy Air60](https://nuphy.com/products/air60) | 3e0ebd0c440af5236d7ff8872343f85d | SH68F90A | BYK916 | ✅ | ✅ |
| [NuPhy Air75](https://nuphy.com/products/air75) | 3e0ebd0c440af5236d7ff8872343f85d | SH68F90A | BYK916 | ✅ | ❓ |
| [NuPhy Air96](https://nuphy.com/products/air96-wireless-mechanical-keyboard) | 3e0ebd0c440af5236d7ff8872343f85d | SH68F90A | BYK916 | ✅ | ❓ |
| [NuPhy Halo65](https://nuphy.com/products/halo65) | 3e0ebd0c440af5236d7ff8872343f85d | SH68F90A | BYK916 | ✅ | ❓ |
| [Redragon K614 Anivia 60%](https://www.redragonzone.com/products/redragon-k614-anivia-60-ultra-thin-wired-mechanical-keyboard) | 2d169670eae0d36eae8188562c1f66e8 | SH68F90A | BYK916 | ✅ | ✅ |
| [Redragon K617 FIZZ 60%](https://www.redragonzone.com/collections/keyboard/products/redragon-k617-fizz-60-wired-rgb-gaming-keyboard-61-keys-compact-mechanical-keyboard) | 2d169670eae0d36eae8188562c1f66e8 | SH68F90A | BYK916 | ✅ | ❓ |
| [Royal Kludge RK100](http://en.rkgaming.com/product/14/) | cfc8661da8c9d7e351b36c0a763426aa | SH68F90? | BYK916 | ✅ | ❓ |
| [Royal Kludge RK61](http://en.rkgaming.com/product/11/) | 3e0ebd0c440af5236d7ff8872343f85d | SH68F90? | BYK916 | ✅ | ✅ |
| Terport TR95 | 2d169670eae0d36eae8188562c1f66e8 | SH68F90A | BYK916 | ✅ | ❓ |
| Weikav Sugar65 | 2d169670eae0d36eae8188562c1f66e8 | SH68F90 | SH68F90S | ✅ | ❓ |
| Xinmeng K916 | cfc8661da8c9d7e351b36c0a763426aa | SH68F90 | ❓ | ✅ | ✅ |

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

Thanks to [@gashtaan](https://github.com/gashtaan) for analyzing and explaining the inner workings of the ISP bootloaders. Without his help, this tool wouldn't be here!
