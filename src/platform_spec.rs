use phf::{phf_map, Map};

const DEFAULT_BOOTLOADER_SIZE: usize = 4096;
const DEFAULT_PAGE_SIZE: usize = 2048;

#[derive(Clone, Copy, PartialEq)]
pub struct PlatformSpec {
    pub firmware_size: usize,
    pub bootloader_size: usize,
    pub page_size: usize,
}

const PLATFORM_DEFAULT: PlatformSpec = PlatformSpec {
    firmware_size: 0,
    bootloader_size: DEFAULT_BOOTLOADER_SIZE,
    page_size: DEFAULT_PAGE_SIZE,
};

pub const PLATFORM_SH68F90: PlatformSpec = PlatformSpec {
    firmware_size: 65536 - PLATFORM_DEFAULT.bootloader_size, // 61440 until bootloader
    ..PLATFORM_DEFAULT
};

pub const PLATFORM_SH68F881: PlatformSpec = PlatformSpec {
    firmware_size: 32768 - PLATFORM_DEFAULT.bootloader_size, // 28672 until bootloader
    ..PLATFORM_DEFAULT
};

pub static PLATFORMS: Map<&'static str, PlatformSpec> = phf_map! {
    "sh68f90" => PLATFORM_SH68F90,
    "sh68f881" => PLATFORM_SH68F881,
};

impl PlatformSpec {
    pub fn available_platforms() -> Vec<&'static str> {
        let mut platforms = PLATFORMS.keys().copied().collect::<Vec<_>>();
        platforms.sort();
        platforms
    }
}
