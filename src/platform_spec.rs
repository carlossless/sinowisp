use phf::{phf_map, Map};

const DEFAULT_BOOTLOADER_SIZE: usize = 4096;
const DEFAULT_PAGE_SIZE: usize = 2048;

#[derive(Clone, Copy, PartialEq)]
pub enum InitOperand {
    Address,
    Length,
}

#[derive(Clone, Copy, PartialEq)]
pub struct PlatformSpec {
    pub firmware_size: usize,
    pub bootloader_size: usize,
    pub page_size: usize,
    pub init_operand: InitOperand,
}

const PLATFORM_DEFAULT: PlatformSpec = PlatformSpec {
    firmware_size: 0,
    bootloader_size: DEFAULT_BOOTLOADER_SIZE,
    page_size: DEFAULT_PAGE_SIZE,
    init_operand: InitOperand::Address,
};

pub const PLATFORM_SH68F90: PlatformSpec = PlatformSpec {
    firmware_size: 65536 - PLATFORM_DEFAULT.bootloader_size, // 61440 until bootloader
    ..PLATFORM_DEFAULT
};

pub const PLATFORM_SH68F89: PlatformSpec = PlatformSpec {
    firmware_size: 65536 - PLATFORM_DEFAULT.bootloader_size, // 61440 until bootloader
    ..PLATFORM_DEFAULT
};

pub const PLATFORM_SH68F881: PlatformSpec = PlatformSpec {
    firmware_size: 32768 - PLATFORM_DEFAULT.bootloader_size, // 28672 until bootloader
    ..PLATFORM_DEFAULT
};

pub const PLATFORM_SH68F902: PlatformSpec = PlatformSpec {
    firmware_size: 16384 - 3072, // 13312 until bootloader
    bootloader_size: 3072,
    page_size: 1024,
    ..PLATFORM_DEFAULT
};

pub const PLATFORM_SH68F83: PlatformSpec = PlatformSpec {
    firmware_size: 16384 - 2048,
    bootloader_size: 2048,
    init_operand: InitOperand::Length,
    ..PLATFORM_DEFAULT
};

pub static PLATFORMS: Map<&'static str, PlatformSpec> = phf_map! {
    "sh68f83" => PLATFORM_SH68F83,
    "sh68f89" => PLATFORM_SH68F89,
    "sh68f881" => PLATFORM_SH68F881,
    "sh68f90" => PLATFORM_SH68F90,
    "sh68f902" => PLATFORM_SH68F902,
};

impl PlatformSpec {
    pub fn available_platforms() -> Vec<&'static str> {
        let mut platforms = PLATFORMS.keys().copied().collect::<Vec<_>>();
        platforms.sort();
        platforms
    }
}
