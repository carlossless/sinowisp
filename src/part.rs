use phf::{phf_map, Map};

#[derive(Clone, Copy)]
pub struct Part {
    pub firmware_size: usize,
    pub bootloader_size: usize,
    pub page_size: usize,
    pub vendor_id: u16,
    pub product_id: u16,

    // The following properties and values are important only for windows support because its
    // HIDAPI requires us to use a specific device for each collection
    /// HID collection `usage_page` with the ISP report
    pub isp_usage_page: u16,
    /// HID collection `usage` with the ISP report
    pub isp_usage: u16,
    /// Index of matching (usage_page && usage) collection at which the ISP report appears in.
    pub isp_index: usize,
}

pub const PART_BASE_DEFAULT: Part = Part {
    firmware_size: 0,
    bootloader_size: 4096,
    page_size: 2048,

    vendor_id: 0x0000,
    product_id: 0x0000,

    isp_usage_page: 0xff00,
    isp_usage: 0x0001,
    isp_index: 0,
};

pub const PART_BASE_SH68F90: Part = Part {
    firmware_size: 61440, // 61440 until bootloader
    ..PART_BASE_DEFAULT
};

pub const PART_BASE_SH68F881: Part = Part {
    firmware_size: 28672, // 28672 until bootloader
    ..PART_BASE_DEFAULT
};

pub const PART_NUPHY_AIR60: Part = Part {
    vendor_id: 0x05ac,
    product_id: 0x024f,
    isp_index: 1,
    ..PART_BASE_SH68F90
};

pub const PART_XINMENG_K916: Part = Part {
    vendor_id: 0x258a,
    product_id: 0x00a1,
    isp_index: 1,
    ..PART_BASE_SH68F90
};

pub const PART_RE_K70_BYK800: Part = Part {
    vendor_id: 0x258a,
    product_id: 0x001a,
    ..PART_BASE_SH68F881
};

pub const PART_TERPORT_TR95: Part = Part {
    vendor_id: 0x258a,
    product_id: 0x0049,
    isp_index: 1,
    ..PART_BASE_SH68F90
};

pub const PART_REDRAGON_FIZZ_K617: Part = Part {
    vendor_id: 0x258a,
    product_id: 0x0049,
    isp_index: 1,
    ..PART_BASE_SH68F90
};

pub const PART_REDRAGON_ANIVIA_K614: Part = Part {
    vendor_id: 0x258a,
    product_id: 0x0049,
    isp_index: 1,
    ..PART_BASE_SH68F90
};

pub const PART_GENESIS_THOR_300: Part = Part {
    vendor_id: 0x258a,
    product_id: 0x001f,
    ..PART_BASE_SH68F881
};

pub const PART_GENESIS_THOR_300_RGB: Part = Part {
    vendor_id: 0x258a,
    product_id: 0x0090,
    ..PART_BASE_SH68F90
};

pub const PART_ROYALKLUDGE_RK61: Part = Part {
    vendor_id: 0x258a,
    product_id: 0x00c7,
    ..PART_BASE_SH68F90
};

pub const PART_ROYALKLUDGE_RK100: Part = Part {
    vendor_id: 0x258a,
    product_id: 0x0056,
    ..PART_BASE_SH68F90
};

pub const PART_WEIKAV_SUGAR65: Part = Part {
    vendor_id: 0x05ac,
    product_id: 0x024f,
    isp_usage: 0x0002,
    ..PART_BASE_SH68F90
};

pub static PARTS: Map<&'static str, Part> = phf_map! {
    "nuphy-air60" => PART_NUPHY_AIR60,
    "nuphy-air75" => PART_NUPHY_AIR60, // same as nuphy-air60
    "nuphy-air96" => PART_NUPHY_AIR60, // same as nuphy-air60
    "nuphy-halo65" => PART_NUPHY_AIR60, // same as nuphy-air60
    "xinmeng-k916" => PART_XINMENG_K916,
    "re-k70-byk800" => PART_RE_K70_BYK800,
    "terport-tr95" => PART_TERPORT_TR95,
    "redragon-k617-fizz" => PART_REDRAGON_FIZZ_K617,
    "redragon-k614-anivia" => PART_REDRAGON_ANIVIA_K614,
    "royalkludge-rk61" => PART_ROYALKLUDGE_RK61,
    "royalkludge-rk100" => PART_ROYALKLUDGE_RK100,
    "genesis-thor-300" => PART_GENESIS_THOR_300,
    "genesis-thor-300-rgb" => PART_GENESIS_THOR_300_RGB,
    "weikav-sugar65" => PART_WEIKAV_SUGAR65,
};

impl Part {
    pub fn available_parts() -> Vec<&'static str> {
        let mut parts = PARTS.keys().copied().collect::<Vec<_>>();
        parts.sort();
        parts
    }

    pub fn num_pages(&self) -> usize {
        self.firmware_size / self.page_size
    }
}

#[test]
fn test_num_pages() {
    assert_eq!(PART_NUPHY_AIR60.num_pages(), 30)
}
