use phf::{phf_map, Map};

#[derive(Default, Clone, Copy)]
pub struct Part {
    pub firmware_size: usize,
    pub bootloader_size: usize,
    pub page_size: usize,
    pub vendor_id: u16,
    pub product_id: u16,
}

pub const PART_NUPHY_AIR60: Part = Part {
    firmware_size: 61440, // 61440 until bootloader
    bootloader_size: 4096,
    page_size: 2048,
    vendor_id: 0x05ac,
    product_id: 0x024f,
};

pub const PART_XINMENG_K916: Part = Part {
    firmware_size: 61440, // 61440 until bootloader
    bootloader_size: 4096,
    page_size: 2048,
    vendor_id: 0x258a,
    product_id: 0x00a1,
};

pub const PART_RE_K70_BYK800: Part = Part {
    firmware_size: 28672, // 28672 until bootloader
    bootloader_size: 4096,
    page_size: 2048,
    vendor_id: 0x258a,
    product_id: 0x001a,
};

pub const PART_TERPORT_TR95: Part = Part {
    firmware_size: 61440, // 61440 until bootloader
    bootloader_size: 4096,
    page_size: 2048,
    vendor_id: 0x258a,
    product_id: 0x0049,
};

pub const PART_REDRAGON_FIZZ_K617: Part = Part {
    firmware_size: 61440, // 61440 until bootloader
    bootloader_size: 4096,
    page_size: 2048,
    vendor_id: 0x258a,
    product_id: 0x0049,
};

pub static PARTS: Map<&'static str, Part> = phf_map! {
    "nuphy-air60" => PART_NUPHY_AIR60,
    "nuphy-air75" => PART_NUPHY_AIR60, // same as nuphy-air60
    "nuphy-air96" => PART_NUPHY_AIR60, // same as nuphy-air60
    "nuphy-halo65" => PART_NUPHY_AIR60, // same as nuphy-air60
    "xinmeng-k916" => PART_XINMENG_K916,
    "re-k70-byk800" => PART_RE_K70_BYK800,
    "terport-tr95" => PART_TERPORT_TR95,
    "redragon-k6170-fizz" => PART_REDRAGON_FIZZ_K617
};

impl Part {
    pub fn available_parts() -> Vec<&'static str> {
        PARTS.keys().copied().collect::<Vec<_>>()
    }

    pub fn num_pages(&self) -> usize {
        self.firmware_size / self.page_size
    }
}

#[test]
fn test_num_pages() {
    assert_eq!(PART_NUPHY_AIR60.num_pages(), 30)
}
