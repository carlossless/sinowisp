use phf::{phf_map, Map};

use crate::platform_spec::{PlatformSpec, PLATFORM_SH68F881, PLATFORM_SH68F90};

const DEFAULT_ISP_IFACE_NUM: i32 = 1;
const DEFAULT_ISP_REPORT_ID: u32 = 5;
const DEFAULT_REBOOT: bool = true;

#[derive(Clone, Copy, PartialEq)]
pub struct DeviceSpec {
    pub vendor_id: u16,
    pub product_id: u16,

    pub platform: PlatformSpec,

    /// USB interface number with the ISP report
    pub isp_iface_num: i32,
    /// HID report ID
    pub isp_report_id: u32,

    pub reboot: bool,
}

pub const DEVICE_BASE_SH68F90: DeviceSpec = DeviceSpec {
    vendor_id: 0x0000,
    product_id: 0x0000,
    platform: PLATFORM_SH68F90,
    isp_iface_num: DEFAULT_ISP_IFACE_NUM,
    isp_report_id: DEFAULT_ISP_REPORT_ID,
    reboot: DEFAULT_REBOOT,
};

pub const DEVICE_BASE_SH68F881: DeviceSpec = DeviceSpec {
    vendor_id: 0x0000,
    product_id: 0x0000,
    platform: PLATFORM_SH68F881,
    isp_iface_num: DEFAULT_ISP_IFACE_NUM,
    isp_report_id: DEFAULT_ISP_REPORT_ID,
    reboot: DEFAULT_REBOOT,
};

pub const DEVICE_NUPHY_AIR60: DeviceSpec = DeviceSpec {
    vendor_id: 0x05ac,
    product_id: 0x024f,
    ..DEVICE_BASE_SH68F90
};

pub const DEVICE_LEOBOG_HI75: DeviceSpec = DeviceSpec {
    vendor_id: 0x258a,
    product_id: 0x010c,
    ..DEVICE_BASE_SH68F90
};

pub const DEVICE_AULA_F75: DeviceSpec = DeviceSpec {
    vendor_id: 0x258a,
    product_id: 0x010c,
    ..DEVICE_BASE_SH68F90
};

pub const DEVICE_AULA_F87: DeviceSpec = DeviceSpec {
    vendor_id: 0x258a,
    product_id: 0x010c,
    ..DEVICE_BASE_SH68F90
};

pub const DEVICE_XINMENG_K916: DeviceSpec = DeviceSpec {
    vendor_id: 0x258a,
    product_id: 0x00a1,
    ..DEVICE_BASE_SH68F90
};

pub const DEVICE_XINMENG_XM_RF68: DeviceSpec = DeviceSpec {
    vendor_id: 0x258a,
    product_id: 0x002a,
    ..DEVICE_BASE_SH68F90
};

pub const DEVICE_XINMENG_M71: DeviceSpec = DeviceSpec {
    vendor_id: 0x258a,
    product_id: 0x010c,
    ..DEVICE_BASE_SH68F90
};

pub const DEVICE_RE_K70_BYK800: DeviceSpec = DeviceSpec {
    vendor_id: 0x258a,
    product_id: 0x001a,
    ..DEVICE_BASE_SH68F881
};

pub const DEVICE_TERPORT_TR95: DeviceSpec = DeviceSpec {
    vendor_id: 0x258a,
    product_id: 0x0049,
    ..DEVICE_BASE_SH68F90
};

pub const DEVICE_REDRAGON_FIZZ_K617: DeviceSpec = DeviceSpec {
    vendor_id: 0x258a,
    product_id: 0x0049,
    ..DEVICE_BASE_SH68F90
};

pub const DEVICE_REDRAGON_K618: DeviceSpec = DeviceSpec {
    vendor_id: 0x258a,
    product_id: 0x0049,
    ..DEVICE_BASE_SH68F90
};

pub const DEVICE_REDRAGON_ANIVIA_K614: DeviceSpec = DeviceSpec {
    vendor_id: 0x258a,
    product_id: 0x0049,
    ..DEVICE_BASE_SH68F90
};

pub const DEVICE_REDRAGON_K641_SHACO_PRO: DeviceSpec = DeviceSpec {
    vendor_id: 0x258a,
    product_id: 0x0049,
    ..DEVICE_BASE_SH68F90
};

pub const DEVICE_GENESIS_THOR_300: DeviceSpec = DeviceSpec {
    vendor_id: 0x258a,
    product_id: 0x001f,
    ..DEVICE_BASE_SH68F881
};

pub const DEVICE_GENESIS_THOR_300_RGB: DeviceSpec = DeviceSpec {
    vendor_id: 0x258a,
    product_id: 0x0090,
    ..DEVICE_BASE_SH68F90
};

pub const DEVICE_ROYALKLUDGE_RK61: DeviceSpec = DeviceSpec {
    vendor_id: 0x258a,
    product_id: 0x00c7,
    ..DEVICE_BASE_SH68F90
};

pub const DEVICE_ROYALKLUDGE_RK68_ISO_RETURN: DeviceSpec = DeviceSpec {
    vendor_id: 0x258a,
    product_id: 0x00a9,
    ..DEVICE_BASE_SH68F90
};

pub const DEVICE_ROYALKLUDGE_RK68_BT_DUAL: DeviceSpec = DeviceSpec {
    vendor_id: 0x258a,
    product_id: 0x008b,
    ..DEVICE_BASE_SH68F90
};

pub const DEVICE_ROYALKLUDGE_RKG68: DeviceSpec = DeviceSpec {
    vendor_id: 0x258a,
    product_id: 0x0049,
    ..DEVICE_BASE_SH68F90
};

pub const DEVICE_ROYALKLUDGE_RK71: DeviceSpec = DeviceSpec {
    vendor_id: 0x258a,
    product_id: 0x00ea,
    ..DEVICE_BASE_SH68F90
};

pub const DEVICE_ROYALKLUDGE_RK84_ISO_RETURN: DeviceSpec = DeviceSpec {
    vendor_id: 0x258a,
    product_id: 0x00f4,
    ..DEVICE_BASE_SH68F90
};

pub const DEVICE_ROYALKLUDGE_RK100: DeviceSpec = DeviceSpec {
    vendor_id: 0x258a,
    product_id: 0x0056,
    ..DEVICE_BASE_SH68F90
};

pub const DEVICE_DELTACO_WK95R: DeviceSpec = DeviceSpec {
    vendor_id: 0x258a,
    product_id: 0x0049,
    ..DEVICE_BASE_SH68F90
};

pub const DEVICE_DIGITALALLIANCE_MECA_WARRIOR_X: DeviceSpec = DeviceSpec {
    vendor_id: 0x258a,
    product_id: 0x0090,
    ..DEVICE_BASE_SH68F90
};

pub const DEVICE_KZZI_K68PRO: DeviceSpec = DeviceSpec {
    vendor_id: 0x258a,
    product_id: 0x0186,
    ..DEVICE_BASE_SH68F90
};

pub const DEVICE_WEIKAV_SUGAR65: DeviceSpec = DeviceSpec {
    vendor_id: 0x05ac,
    product_id: 0x024f,
    ..DEVICE_BASE_SH68F90
};

pub const DEVICE_TRUST_GXT_960: DeviceSpec = DeviceSpec {
    vendor_id: 0x145f,
    product_id: 0x02b6,
    ..DEVICE_BASE_SH68F90
};

pub const DEVICE_GLORIOUS_MODEL_O: DeviceSpec = DeviceSpec {
    vendor_id: 0x258a,
    product_id: 0x0036,
    ..DEVICE_BASE_SH68F90
};

pub const DEVICE_MACHENIKE_K500_B61: DeviceSpec = DeviceSpec {
    vendor_id: 0x258a,
    product_id: 0x0049,
    ..DEVICE_BASE_SH68F90
};

pub const DEVICE_MAGEGEE_MKSTAR61: DeviceSpec = DeviceSpec {
    vendor_id: 0x258a,
    product_id: 0x010c,
    ..DEVICE_BASE_SH68F90
};

pub const DEVICE_REDRAGON_K658_PRO_SE: DeviceSpec = DeviceSpec {
    vendor_id: 0x258a,
    product_id: 0x0049,
    ..DEVICE_BASE_SH68F90
};

pub const DEVICE_REDRAGON_K530_DRACONIC_PRO: DeviceSpec = DeviceSpec {
    vendor_id: 0x258a,
    product_id: 0x0049,
    ..DEVICE_BASE_SH68F90
};

pub const DEVICE_REDRAGON_K630_NO_RGB: DeviceSpec = DeviceSpec {
    vendor_id: 0x258a,
    product_id: 0x002a,
    ..DEVICE_BASE_SH68F90
};

pub const DEVICE_EYOOSO_Z11: DeviceSpec = DeviceSpec {
    vendor_id: 0x258a,
    product_id: 0x002a,
    ..DEVICE_BASE_SH68F90
};

pub const DEVICE_EYOOSO_Z82: DeviceSpec = DeviceSpec {
    vendor_id: 0x258a,
    product_id: 0x010c,
    ..DEVICE_BASE_SH68F90
};

pub const DEVICE_PORTRONICS_HYDRA10: DeviceSpec = DeviceSpec {
    vendor_id: 0x258a,
    product_id: 0x0049,
    ..DEVICE_BASE_SH68F90
};

pub const DEVICE_REDRAGON_K633_RYZE: DeviceSpec = DeviceSpec {
    vendor_id: 0x258a,
    product_id: 0x0049,
    ..DEVICE_BASE_SH68F90
};

pub const DEVICE_YINREN_R108: DeviceSpec = DeviceSpec {
    vendor_id: 0x258a,
    product_id: 0x0049,
    ..DEVICE_BASE_SH68F90
};

pub static DEVICES: Map<&'static str, DeviceSpec> = phf_map! {
    "aula-f75" => DEVICE_AULA_F75,
    "aula-f87" => DEVICE_AULA_F87,
    "deltaco-wk95r" => DEVICE_DELTACO_WK95R,
    "digitalalliance-meca-warrior-x" => DEVICE_DIGITALALLIANCE_MECA_WARRIOR_X,
    "eyooso-z11" => DEVICE_EYOOSO_Z11,
    "eyooso-z82" => DEVICE_EYOOSO_Z82,
    "genesis-thor-300-rgb" => DEVICE_GENESIS_THOR_300_RGB,
    "genesis-thor-300" => DEVICE_GENESIS_THOR_300,
    "glorious-model-o" => DEVICE_GLORIOUS_MODEL_O,
    "kzzi-k68pro" => DEVICE_KZZI_K68PRO,
    "leobog-hi75" => DEVICE_LEOBOG_HI75,
    "machenike-k500-b61" => DEVICE_MACHENIKE_K500_B61,
    "magegee-mkstar61" => DEVICE_MAGEGEE_MKSTAR61,
    "nuphy-air60" => DEVICE_NUPHY_AIR60,
    "nuphy-air75" => DEVICE_NUPHY_AIR60, // same as nuphy-air60
    "nuphy-air96" => DEVICE_NUPHY_AIR60, // same as nuphy-air60
    "nuphy-halo65" => DEVICE_NUPHY_AIR60, // same as nuphy-air60
    "portronics-hydra10" => DEVICE_PORTRONICS_HYDRA10,
    "re-k70-byk800" => DEVICE_RE_K70_BYK800,
    "redragon-k530-draconic-pro" => DEVICE_REDRAGON_K530_DRACONIC_PRO,
    "redragon-k630-norgb" => DEVICE_REDRAGON_K630_NO_RGB,
    "redragon-k614-anivia" => DEVICE_REDRAGON_ANIVIA_K614,
    "redragon-k617-fizz" => DEVICE_REDRAGON_FIZZ_K617,
    "redragon-k618" => DEVICE_REDRAGON_K618,
    "redragon-k633-ryze" => DEVICE_REDRAGON_K633_RYZE,
    "redragon-k641-shaco-pro" => DEVICE_REDRAGON_K641_SHACO_PRO,
    "redragon-k658-pro-se" => DEVICE_REDRAGON_K658_PRO_SE,
    "royalkludge-rk100" => DEVICE_ROYALKLUDGE_RK100,
    "royalkludge-rk61" => DEVICE_ROYALKLUDGE_RK61,
    "royalkludge-rk68-bt-dual" => DEVICE_ROYALKLUDGE_RK68_BT_DUAL,
    "royalkludge-rk68-iso-return" => DEVICE_ROYALKLUDGE_RK68_ISO_RETURN,
    "royalkludge-rk71" => DEVICE_ROYALKLUDGE_RK71,
    "royalkludge-rk84-iso-return" => DEVICE_ROYALKLUDGE_RK84_ISO_RETURN,
    "royalkludge-rkg68" => DEVICE_ROYALKLUDGE_RKG68,
    "terport-tr95" => DEVICE_TERPORT_TR95,
    "trust-gxt-960" => DEVICE_TRUST_GXT_960,
    "weikav-sugar65" => DEVICE_WEIKAV_SUGAR65,
    "xinmeng-k916" => DEVICE_XINMENG_K916,
    "xinmeng-m66" => DEVICE_XINMENG_M71,
    "xinmeng-m71" => DEVICE_XINMENG_M71,
    "xinmeng-xm-rf68" => DEVICE_XINMENG_XM_RF68,
    "yinren-r108" => DEVICE_YINREN_R108,
    "yunzii-al66" => DEVICE_XINMENG_M71, // same as xinmeng-m71
    "yunzii-al71" => DEVICE_XINMENG_M71, // same as xinmeng-m71
};

impl DeviceSpec {
    pub fn available_devices() -> Vec<&'static str> {
        let mut device_names = DEVICES.keys().copied().collect::<Vec<_>>();
        device_names.sort();
        device_names
    }

    pub fn num_pages(&self) -> usize {
        self.platform.firmware_size / self.platform.page_size
    }

    pub fn total_flash_size(&self) -> usize {
        self.platform.firmware_size + self.platform.bootloader_size
    }
}

#[test]
fn test_device_num_pages() {
    assert_eq!(DEVICE_NUPHY_AIR60.num_pages(), 30)
}

#[test]
fn test_device_total_flash_size() {
    assert_eq!(DEVICE_NUPHY_AIR60.total_flash_size(), 65536)
}
