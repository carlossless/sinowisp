use phf::{phf_map, Map};

#[derive(Clone, Copy, PartialEq)]
pub struct Part {
    pub firmware_size: usize,
    pub bootloader_size: usize,
    pub page_size: usize,
    pub vendor_id: u16,
    pub product_id: u16,

    /// USB interface number with the ISP report
    pub isp_iface_num: u8,
    // The following properties and values are important only for windows support because its
    // HIDAPI requires us to use a specific device for each collection
    /// HID collection `usage_page` with the ISP report
    pub isp_usage_page: u16,
    /// HID collection `usage` with the ISP report
    pub isp_usage: u16,
    /// Index of matching (usage_page && usage) collection at which the ISP report appears in.
    pub isp_index: usize,

    pub reboot: bool,
}

pub const PART_BASE_DEFAULT: Part = Part {
    firmware_size: 0,
    bootloader_size: 4096,
    page_size: 2048,

    vendor_id: 0x0000,
    product_id: 0x0000,

    isp_iface_num: 1,
    isp_usage_page: 0xff00,
    isp_usage: 0x0001,
    isp_index: 0,

    reboot: true,
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

pub const PART_LEOBOG_HI75: Part = Part {
    vendor_id: 0x258a,
    product_id: 0x010c,
    isp_index: 1,
    ..PART_BASE_SH68F90
};

pub const PART_XINMENG_K916: Part = Part {
    vendor_id: 0x258a,
    product_id: 0x00a1,
    isp_index: 1,
    ..PART_BASE_SH68F90
};

pub const PART_XINMENG_XM_RF68: Part = Part {
    vendor_id: 0x258a,
    product_id: 0x002a,
    ..PART_BASE_SH68F90
};

pub const PART_XINMENG_M71: Part = Part {
    vendor_id: 0x258a,
    product_id: 0x010c,
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

pub const PART_REDRAGON_K641_SHACO_PRO: Part = Part {
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

pub const PART_ROYALKLUDGE_RK68_ISO_RETURN: Part = Part {
    vendor_id: 0x258a,
    product_id: 0x00a9,
    ..PART_BASE_SH68F90
};

pub const PART_ROYALKLUDGE_RK68_BT_DUAL: Part = Part {
    vendor_id: 0x258a,
    product_id: 0x008b,
    ..PART_BASE_SH68F90
};

pub const PART_ROYALKLUDGE_RKG68: Part = Part {
    vendor_id: 0x258a,
    product_id: 0x0049,
    ..PART_BASE_SH68F90
};

pub const PART_ROYALKLUDGE_RK71: Part = Part {
    vendor_id: 0x258a,
    product_id: 0x00ea,
    ..PART_BASE_SH68F90
};

pub const PART_ROYALKLUDGE_RK84_ISO_RETURN: Part = Part {
    vendor_id: 0x258a,
    product_id: 0x00f4,
    ..PART_BASE_SH68F90
};

pub const PART_ROYALKLUDGE_RK100: Part = Part {
    vendor_id: 0x258a,
    product_id: 0x0056,
    ..PART_BASE_SH68F90
};

pub const PART_DIGITALALLIANCE_MECA_WARRIOR_X: Part = Part {
    vendor_id: 0x258a,
    product_id: 0x0090,
    ..PART_BASE_SH68F90
};

pub const PART_WEIKAV_SUGAR65: Part = Part {
    vendor_id: 0x05ac,
    product_id: 0x024f,
    isp_usage: 0x0002,
    ..PART_BASE_SH68F90
};

pub const PART_TRUST_GXT_960: Part = Part {
    vendor_id: 0x145f,
    product_id: 0x02b6,
    ..PART_BASE_SH68F90
};

pub const PART_GLORIOUS_MODEL_O: Part = Part {
    vendor_id: 0x258a,
    product_id: 0x0036,
    ..PART_BASE_SH68F90
};

pub const PART_MACHENIKE_K500_B61: Part = Part {
    vendor_id: 0x258a,
    product_id: 0x0049,
    isp_index: 1,
    ..PART_BASE_SH68F90
};

pub const PART_REDRAGON_K658_PRO_SE: Part = Part {
    vendor_id: 0x258a,
    product_id: 0x0049,
    isp_index: 1,
    ..PART_BASE_SH68F90
};

pub const PART_REDRAGON_K530_DRACONIC_PRO: Part = Part {
    vendor_id: 0x258a,
    product_id: 0x0049,
    ..PART_BASE_SH68F90
};

pub const PART_EYOOSO_Z11: Part = Part {
    vendor_id: 0x258a,
    product_id: 0x002a,
    ..PART_BASE_SH68F90
};

pub static PARTS: Map<&'static str, Part> = phf_map! {
    "digitalalliance-meca-warrior-x" => PART_DIGITALALLIANCE_MECA_WARRIOR_X,
    "eyooso-z11" => PART_EYOOSO_Z11,
    "genesis-thor-300-rgb" => PART_GENESIS_THOR_300_RGB,
    "genesis-thor-300" => PART_GENESIS_THOR_300,
    "glorious-model-o" => PART_GLORIOUS_MODEL_O,
    "leobog-hi75" => PART_LEOBOG_HI75,
    "machenike-k500-b61" => PART_MACHENIKE_K500_B61,
    "nuphy-air60" => PART_NUPHY_AIR60,
    "nuphy-air75" => PART_NUPHY_AIR60, // same as nuphy-air60
    "nuphy-air96" => PART_NUPHY_AIR60, // same as nuphy-air60
    "nuphy-halo65" => PART_NUPHY_AIR60, // same as nuphy-air60
    "re-k70-byk800" => PART_RE_K70_BYK800,
    "redragon-k530-draconic-pro" => PART_REDRAGON_K530_DRACONIC_PRO,
    "redragon-k614-anivia" => PART_REDRAGON_ANIVIA_K614,
    "redragon-k617-fizz" => PART_REDRAGON_FIZZ_K617,
    "redragon-k641-shaco-pro" => PART_REDRAGON_K641_SHACO_PRO,
    "redragon-k658-pro-se" => PART_REDRAGON_K658_PRO_SE,
    "royalkludge-rk100" => PART_ROYALKLUDGE_RK100,
    "royalkludge-rk61" => PART_ROYALKLUDGE_RK61,
    "royalkludge-rk68-bt-dual" => PART_ROYALKLUDGE_RK68_BT_DUAL,
    "royalkludge-rk68-iso-return" => PART_ROYALKLUDGE_RK68_ISO_RETURN,
    "royalkludge-rk71" => PART_ROYALKLUDGE_RK71,
    "royalkludge-rk84-iso-return" => PART_ROYALKLUDGE_RK84_ISO_RETURN,
    "royalkludge-rkg68" => PART_ROYALKLUDGE_RKG68,
    "terport-tr95" => PART_TERPORT_TR95,
    "trust-gxt-960" => PART_TRUST_GXT_960,
    "weikav-sugar65" => PART_WEIKAV_SUGAR65,
    "xinmeng-k916" => PART_XINMENG_K916,
    "xinmeng-m71" => PART_XINMENG_M71,
    "xinmeng-xm-rf68" => PART_XINMENG_XM_RF68,
    "yunzii-al71" => PART_XINMENG_M71, // same as xinmeng-m71
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
