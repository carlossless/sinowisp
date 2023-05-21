# sinowealth-kb-tool

A utility for reading and writing flash contents on Sinowealth 8051-based devices (keyboards and mice) since they all seem to have similar ISP bootloaders.

## Disclaimer

This is an experimental tool, so use it at your own risk.

## Supported Hardware

| Keyboard | ISP MD5 | MCU | Supported |
| -------- | ------- | --- | --------- |
| [NuPhy Air60](https://nuphy.com/products/air60) | 3e0ebd0c440af5236d7ff8872343f85d | SH68F90A (labeled as BYK916) | ✅ |
| Xinmeng K916 | unknown | SH68F90 | ❓ |
| Hykker X Range 2017 (RE-K70-BYK800) | 13df4ce2933f9654ffef80d6a3c27199 | SH68F881 (labeled as BYK801) | ✅ |

## Acknowledgments

* https://github.com/gashtaan/sinowealth-8051-dumper
* https://github.com/ayufan-rock64/pinebook-pro-keyboard-updater
