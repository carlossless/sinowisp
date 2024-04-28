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
    --isp_iface_num 1 \ # optional
    --isp_usage_page 0xff00 \ # optional
    --isp_usage 0x0001 \ # optional
    --isp_index 0 \ # optional
    --reboot false \ # optional
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
    --isp_iface_num 1 \ # optional
    --isp_usage_page 0xff00 \ # optional
    --isp_usage 0x0001 \ # optional
    --isp_index 0 \ # optional
    --reboot false \ # optional
    foobar.hex
```

## Supported Hardware

### Keyboards

| Model | ISP MD5 | MCU | MCU Label | Tested Read | Tested Write |
| ----- | ------- | --- | --------- | ----------- | ------------ |
| Digital Alliance Meca Warrior X | 2d169670eae0d36eae8188562c1f66e8 | SH68F90 | SH68F90S | ✅ | ✅ |
| E-Yooso Z11 | 3e0ebd0c440af5236d7ff8872343f85d | SH68F90? | BYK901 | ✅ | ✅ |
| [Genesis Thor 300 RGB](https://genesis-zone.com/product/thor-300-rgb-brown) | 2d169670eae0d36eae8188562c1f66e8 | SH68F90 | SH68F90S | ✅ | ✅ |
| [Genesis Thor 300](https://genesis-zone.com/product/thor-300-outemu-blue) | e57490acebcaabfcff84a0ff013955d9 | SH68F881 | SH68F881W | ✅ | ✅ |
| Hykker X Range 2017 (RE-K70-BYK800) | 13df4ce2933f9654ffef80d6a3c27199 | SH68F881 | BYK801 | ✅ | ❓ |
| [Leobog Hi75](https://leobogtech.com/products/leobog-hi75) | 3e0ebd0c440af5236d7ff8872343f85d | SH68F90A | BYK916 | ✅ | ✅ |
| [Machenike K500-B61](https://global.machenike.com/products/k500-b61) | 2d169670eae0d36eae8188562c1f66e8 | SH68F90? | BYK916 | ✅ | ✅ |
| [NuPhy Air60](https://nuphy.com/products/air60) | 3e0ebd0c440af5236d7ff8872343f85d | SH68F90A | BYK916 | ✅ | ✅ |
| [NuPhy Air75](https://nuphy.com/products/air75) | 3e0ebd0c440af5236d7ff8872343f85d | SH68F90A | BYK916 | ✅ | ✅ |
| [NuPhy Air96](https://nuphy.com/products/air96-wireless-mechanical-keyboard) | 3e0ebd0c440af5236d7ff8872343f85d | SH68F90A | BYK916 | ✅ | ✅ |
| [NuPhy Halo65](https://nuphy.com/products/halo65) | 3e0ebd0c440af5236d7ff8872343f85d | SH68F90A | BYK916 | ✅ | ✅ |
| [Redragon K530 Draconic PRO](https://www.redragonzone.com/products/draconic-k530) | cfc8661da8c9d7e351b36c0a763426aa | SH68F90A | BYK916 | ✅ | ✅ |
| [Redragon K614 Anivia 60%](https://www.redragonzone.com/products/redragon-k614-anivia-60-ultra-thin-wired-mechanical-keyboard) | 2d169670eae0d36eae8188562c1f66e8 | SH68F90A | BYK916 | ✅ | ✅ |
| [Redragon K617 FIZZ 60%](https://www.redragonzone.com/collections/keyboard/products/redragon-k617-fizz-60-wired-rgb-gaming-keyboard-61-keys-compact-mechanical-keyboard) | 2d169670eae0d36eae8188562c1f66e8 | SH68F90A | BYK916 | ✅ | ✅ |
| [Redragon K641 SHACO PRO](https://www.redragonzone.com/products/redragon-k641-shaco-pro-65-aluminum-rgb-mechanical-keyboard) | 3e0ebd0c440af5236d7ff8872343f85d | SH68F90A | BYK916 | ✅ | ✅ |
| [Redragon K658 PRO SE](https://www.redragonzone.com/products/k658-pro-se-90-wireless-rgb-gaming-keyboard) | 3e0ebd0c440af5236d7ff8872343f85d | SH68F90A | BYK916 | ✅ | ✅ |
| [Royal Kludge RK100](http://en.rkgaming.com/product/14/) | cfc8661da8c9d7e351b36c0a763426aa | SH68F90? | BYK916 | ✅ | ✅ |
| [Royal Kludge RK61](http://en.rkgaming.com/product/11/) | 3e0ebd0c440af5236d7ff8872343f85d | SH68F90? | BYK916 | ✅ | ✅ |
| Royal Kludge RK68 BT Dual | cfc8661da8c9d7e351b36c0a763426aa | SH68F90? | BYK901 | ✅ | ✅ |
| Royal Kludge RK68 ISO Return | ❓ | SH68F90? | BYK916 | ✅ | ❓ |
| [Royal Kludge RK71](http://en.rkgaming.com/product/12/) | cfc8661da8c9d7e351b36c0a763426aa | SH68F90? | ❓ | ✅ | ✅ |
| [Royal Kludge RK84](http://en.rkgaming.com/product/16/) | cfc8661da8c9d7e351b36c0a763426aa | SH68F90? | BYK916 | ✅ | ✅ |
| Royal Kludge RKG68 | cfc8661da8c9d7e351b36c0a763426aa | SH68F90A | SH68F90AS | ✅ | ✅ |
| Terport TR95 | 2d169670eae0d36eae8188562c1f66e8 | SH68F90A | BYK916 | ✅ | ✅ |
| Weikav Sugar65 | 2d169670eae0d36eae8188562c1f66e8 | SH68F90 | SH68F90S | ✅ | ✅ |
| Xinmeng K916 | cfc8661da8c9d7e351b36c0a763426aa | SH68F90 | ❓ | ✅ | ✅ |
| Xinmeng M71 | 2d169670eae0d36eae8188562c1f66e8 | SH68F90A | SH68F90AS | ✅ | ✅ |
| Xinmeng XM-RF68 | 2d169670eae0d36eae8188562c1f66e8 | SH68F90 | SH68F90U | ✅ | ✅ |
| Yunzii AL71 | 2d169670eae0d36eae8188562c1f66e8 | SH68F90A | SH68F90AS | ✅ | ✅ |

### Mice

| Model | ISP MD5 | MCU | MCU Label | Tested Read | Tested Write |
| ----- | ------- | --- | --------- | ----------- | ------------ |
| [Glorious Model O](https://web.archive.org/web/20220609205659mp_/https://www.gloriousgaming.com/products/glorious-model-o-black) | 46459c31e58194fa076b8ce8fb1f3eaa | ❓ | BY8948 | ✅ | ❓ |
| [Trust GXT 960](https://www.trust.com/en/product/23758-gxt-960-graphin-ultra-lightweight-gaming-mouse) | 620f0b67a91f7f74151bc5be745b7110 | ❓ | BY8801 | ✅ | ❓ |

## Bootloader Support

### Platforms

| ISP MD5                          | Windows  | macOS    | Linux |
| -------------------------------- | -------- | -------- | ----- |
| 13df4ce2933f9654ffef80d6a3c27199 | ?        | ?        | ok    |
| 2d169670eae0d36eae8188562c1f66e8 | ok       | ?        | ok    |
| 3e0ebd0c440af5236d7ff8872343f85d | ok       | ok       | ok    |
| 46459c31e58194fa076b8ce8fb1f3eaa | ?        | ?        | ok    |
| 620f0b67a91f7f74151bc5be745b7110 | ?        | fail[^1] | ok    |
| cfc8661da8c9d7e351b36c0a763426aa | ok       | fail[^1] | ok    |
| e57490acebcaabfcff84a0ff013955d9 | ok       | ?        | ?     |

[^1]: macOS does not recognize the composite device as an HID device

## Prerequisites

### Linux

To enable running this tool without superuser privileges add the following udev rule with `xxxx` and `yyyy` replaced with your device Vendor ID and Product ID respectively.

```udev
# /etc/udev/rules.d/plugdev.rule
SUBSYSTEMS=="usb", ATTRS{idVendor}=="xxxx", ATTRS{idProduct}=="yyyy", MODE="0660", GROUP="plugdev"
SUBSYSTEMS=="usb", ATTRS{idVendor}=="0603", ATTRS{idProduct}=="1020", MODE="0660", GROUP="plugdev"
```

Make sure your user is part of the `plugdev` group.

### macOS

If you encounter errors like:
```
hid_open_path: failed to open IOHIDDevice from mach entry...
```

Ensure that your terminal application has [access to input monitoring](https://support.apple.com/guide/mac-help/control-access-to-input-monitoring-on-mac-mchl4cedafb6/mac).

## Acknowledgments

Thanks to [@gashtaan](https://github.com/gashtaan) for analyzing and explaining the inner workings of the ISP bootloaders. Without his help, this tool wouldn't be here!
