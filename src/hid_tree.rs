use crate::{device_selector::DeviceSelectorError, to_hex_string};

pub struct DeviceNode {
    pub product_id: u16,
    pub vendor_id: u16,
    pub product_string: String,
    pub manufacturer_string: String,
    pub children: Vec<InterfaceNode>,
}

pub struct InterfaceNode {
    pub interface_number: i32,
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    pub path: String,
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    pub descriptor: Result<Vec<u8>, DeviceSelectorError>,
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    pub feature_report_ids: Result<Vec<u32>, DeviceSelectorError>,
    #[cfg(any(target_os = "macos", target_os = "windows"))]
    pub children: Vec<ItemNode>,
}

pub struct ItemNode {
    #[cfg(target_os = "windows")]
    pub path: String,
    #[cfg(any(target_os = "macos", target_os = "windows"))]
    pub usage_page: u16,
    #[cfg(any(target_os = "macos", target_os = "windows"))]
    pub usage: u16,
    #[cfg(target_os = "windows")]
    pub descriptor: Result<Vec<u8>, DeviceSelectorError>,
    #[cfg(target_os = "windows")]
    pub feature_report_ids: Result<Vec<u32>, DeviceSelectorError>,
}

const INDENT_SIZE: usize = 4;

pub trait TreeDisplay {
    fn to_tree_string(self, level: usize) -> String;
}

impl<T, I> TreeDisplay for T
where
    T: Iterator<Item = I>,
    I: TreeDisplay,
{
    fn to_tree_string(self, level: usize) -> String {
        let indent = " ".repeat(INDENT_SIZE).repeat(level);
        let mut s: Vec<String> = vec![];
        for item in self {
            s.push(format!("{}{}", indent, item.to_tree_string(level)));
        }
        s.join("\n")
    }
}

impl TreeDisplay for DeviceNode {
    fn to_tree_string(self, level: usize) -> String {
        let indent = " ".repeat(INDENT_SIZE).repeat(level);
        let mut s: Vec<String> = vec![];
        s.push(format!(
            "{indent}ID {:04x}:{:04x} manufacturer=\"{:}\" product=\"{:}\"",
            self.vendor_id, self.product_id, self.manufacturer_string, self.product_string
        ));
        for child in self.children {
            s.push(child.to_tree_string(level + 1));
        }
        s.join("\n")
    }
}

impl TreeDisplay for InterfaceNode {
    fn to_tree_string(self, level: usize) -> String {
        let indent = " ".repeat(INDENT_SIZE).repeat(level);
        let mut s: Vec<String> = vec![];
        #[cfg(any(target_os = "macos", target_os = "linux"))]
        s.push(format!(
            "{indent}path=\"{}\" interface_number={}",
            self.path, self.interface_number
        ));
        #[cfg(target_os = "windows")]
        s.push(format!(
            "{indent}interface_number={}",
            self.interface_number
        ));
        #[cfg(any(target_os = "macos", target_os = "linux"))]
        {
            let descriptor = self.descriptor.as_ref();
            match descriptor {
                Ok(descriptor) => {
                    s.push(format!(
                        "{indent}report_descriptor=[{}]",
                        to_hex_string(descriptor)
                    ));
                }
                Err(e) => {
                    s.push(format!("{indent}report_descriptor=error: {}", e));
                }
            }
            let feature_report_ids = self.feature_report_ids.as_ref();
            match feature_report_ids {
                Ok(feature_report_ids) => {
                    s.push(format!(
                        "{indent}feature_report_ids=[{}]",
                        feature_report_ids
                            .iter()
                            .map(|rid| format!("{:#04x}", rid))
                            .collect::<Vec<String>>()
                            .join(", ")
                    ));
                }
                Err(e) => {
                    s.push(format!("{indent}feature_report_ids=error: {}", e));
                }
            }
        }
        #[cfg(any(target_os = "macos", target_os = "windows"))]
        {
            for child in self.children {
                s.push(child.to_tree_string(level + 1));
            }
        }
        s.join("\n")
    }
}

impl TreeDisplay for ItemNode {
    fn to_tree_string(self, level: usize) -> String {
        let indent = " ".repeat(INDENT_SIZE).repeat(level);
        let mut s: Vec<String> = vec![];
        #[cfg(target_os = "macos")]
        s.push(format!(
            "{indent}usage_page={:#06x} usage={:#06x}",
            self.usage_page, self.usage
        ));
        #[cfg(target_os = "windows")]
        {
            s.push(format!(
                "{indent}path=\"{}\" usage_page={:#06x} usage={:#06x}",
                self.path, self.usage_page, self.usage
            ));
            let descriptor = self.descriptor.as_ref();
            match descriptor {
                Ok(descriptor) => {
                    s.push(format!(
                        "{indent}report_descriptor=[{}]",
                        to_hex_string(descriptor)
                    ));
                }
                Err(e) => {
                    s.push(format!("{indent}report_descriptor=error: {}", e));
                }
            }
            let feature_report_ids = self.feature_report_ids.as_ref();
            match feature_report_ids {
                Ok(feature_report_ids) => {
                    s.push(format!(
                        "{indent}feature_report_ids=[{}]",
                        feature_report_ids
                            .iter()
                            .map(|rid| format!("{:#04x}", rid))
                            .collect::<Vec<String>>()
                            .join(", ")
                    ));
                }
                Err(e) => {
                    s.push(format!("{indent}feature_report_ids=error: {}", e));
                }
            }
        }
        s.join("\n")
    }
}
