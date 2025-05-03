use std::{
    env, fs,
    io::{self, Read},
    process::ExitCode,
};

use clap::{arg, value_parser, ArgMatches, Command};
use clap_num::maybe_hex;
use device_selector::{DeviceSelector, DeviceSelectorError};
use hid_tree::TreeDisplay;
use log::{error, info};
use simple_logger::SimpleLogger;
use thiserror::Error;

mod device_selector;
mod hid_tree;
mod ihex;
mod isp_device;
mod part;
mod util;

pub use crate::{ihex::*, isp_device::*, part::*, util::*};

#[derive(Debug, Error)]
pub enum CLIError {
    #[error(transparent)]
    IOError(#[from] io::Error),
    #[error(transparent)]
    ISPError(#[from] ISPError),
    #[error(transparent)]
    IHEXError(#[from] ConversionError),
    #[error(transparent)]
    PayloadConversionError(#[from] PayloadConversionError),
    #[error(transparent)]
    DeviceSelectorError(#[from] DeviceSelectorError),
}

fn main() -> ExitCode {
    match err_main() {
        Ok(_) => ExitCode::SUCCESS,
        Err(err) => {
            error!("{:}", err);
            ExitCode::FAILURE
        }
    }
}

fn cli() -> Command {
    Command::new("sinowealth-kb-tool")
        .about("A programming tool for Sinowealth Gaming KB devices")
        .version(env!("CARGO_PKG_VERSION"))
        .subcommand_required(true)
        .arg_required_else_help(true)
        .author("Karolis Stasaitis")
        .subcommand(
            Command::new("list")
                .short_flag('l')
                .about("List all connected devices and their identifiers. This is useful to find the manufacturer and product id for your device.")
                .arg(arg!(--vendor_id <VID>).value_parser(maybe_hex::<u16>))
                .arg(arg!(--product_id <PID>).value_parser(maybe_hex::<u16>))
        )
        .subcommand(
            Command::new("convert")
                .short_flag('c')
                .about("Convert payload from bootloader to JTAG and vice versa.")
                .arg(arg!(-d --direction <DIRECTION> "direction of conversion").value_parser(["to_jtag", "to_isp"]).required(true))
                .arg(arg!(input_file: <INPUT_FILE> "file to convert"))
                .arg(arg!(output_file: <OUTPUT_FILE> "file to write results to"))
                .part_args()
        )
        .subcommand(
            Command::new("read")
                .short_flag('r')
                .about("Read flash contents. (Intel HEX)")
                .arg(arg!(output_file: <OUTPUT_FILE> "file to write flash contents to"))
                .arg(arg!(-f --fragment <fragment> "firmware fragment to read").value_parser(["firmware", "bootloader", "full"]).default_value("firmware"))
                .arg(arg!(-r --retry <NUM> "number of retries trying to find device").value_parser(value_parser!(usize)).default_value("5"))
                .part_args()
        )
        .subcommand(
            Command::new("write")
                .short_flag('w')
                .about("Write file (Intel HEX) into flash.")
                .arg(arg!(input_file: <INPUT_FILE> "payload to write into flash"))
                .arg(arg!(-r --retry <NUM> "number of retries trying to find device").value_parser(value_parser!(usize)).default_value("5"))
                .part_args(),
        )
}

fn get_log_level() -> log::LevelFilter {
    if let Ok(debug) = env::var("DEBUG") {
        if debug == "1" {
            log::LevelFilter::Debug
        } else {
            log::LevelFilter::Info
        }
    } else {
        #[cfg(debug_assertions)]
        return log::LevelFilter::Debug;
        #[cfg(not(debug_assertions))]
        log::LevelFilter::Info
    }
}

fn err_main() -> Result<(), CLIError> {
    SimpleLogger::new()
        .with_level(get_log_level())
        .init()
        .unwrap();

    let matches = cli().get_matches();

    match matches.subcommand() {
        Some(("read", sub_matches)) => {
            let output_file = sub_matches
                .get_one::<String>("output_file")
                .map(|s| s.as_str())
                .unwrap();

            let retry_count = sub_matches
                .get_one::<usize>("retry")
                .map(|s| s.to_owned())
                .unwrap();

            let fragment = sub_matches
                .get_one::<String>("fragment")
                .map(|s| s.as_str())
                .unwrap();

            let part = get_part_from_matches(sub_matches);

            let fragment: ReadFragment = match fragment {
                "firmware" => ReadFragment::Firmware,
                "bootloader" => ReadFragment::Bootloader,
                "full" => ReadFragment::Full,
                _ => panic!("Invalid read fragment"),
            };

            let mut ds = DeviceSelector::new().map_err(CLIError::DeviceSelectorError)?;
            let device = ds
                .find_isp_device(part, retry_count)
                .map_err(CLIError::from)?;
            let result = device.read_cycle(fragment).map_err(CLIError::from)?;

            let digest = md5::compute(&result);
            info!("MD5: {:x}", digest);

            let ihex = to_ihex(result).map_err(CLIError::from)?;
            fs::write(output_file, ihex).map_err(CLIError::from)?;
        }
        Some(("write", sub_matches)) => {
            let input_file = sub_matches
                .get_one::<String>("input_file")
                .map(|s| s.as_str())
                .unwrap();

            let retry_count = sub_matches
                .get_one::<usize>("retry")
                .map(|s| s.to_owned())
                .unwrap();

            let part = get_part_from_matches(sub_matches);

            let mut file = fs::File::open(input_file).map_err(CLIError::from)?;
            let mut file_buf = Vec::new();
            file.read_to_end(&mut file_buf).map_err(CLIError::from)?;
            let file_str = String::from_utf8_lossy(&file_buf[..]);
            let mut firmware = from_ihex(&file_str, part.firmware_size).map_err(CLIError::from)?;

            if firmware.len() < part.firmware_size {
                firmware.resize(part.firmware_size, 0);
            }

            let mut ds = DeviceSelector::new().map_err(CLIError::DeviceSelectorError)?;
            let device = ds
                .find_isp_device(part, retry_count)
                .map_err(CLIError::from)?;
            device.write_cycle(&mut firmware).map_err(CLIError::from)?;
        }
        Some(("list", sub_matches)) => {
            let vendor_id = sub_matches.get_one::<u16>("vendor_id");
            let product_id = sub_matches.get_one::<u16>("product_id");

            let ds = DeviceSelector::new().map_err(CLIError::DeviceSelectorError)?;
            let devices = ds
                .connected_devices_tree()
                .map_err(CLIError::DeviceSelectorError)?;
            let tree = devices
                .into_iter()
                .filter(|device| {
                    if let Some(vendor_id) = vendor_id {
                        if device.vendor_id != *vendor_id {
                            return false;
                        }
                    }
                    if let Some(product_id) = product_id {
                        if device.product_id != *product_id {
                            return false;
                        }
                    }
                    true
                })
                .to_tree_string(0);

            println!("{}", tree);
        }
        Some(("convert", sub_matches)) => {
            let input_file = sub_matches
                .get_one::<String>("input_file")
                .map(|s| s.as_str())
                .unwrap();

            let output_file = sub_matches
                .get_one::<String>("output_file")
                .map(|s| s.as_str())
                .unwrap();

            let direction = sub_matches
                .get_one::<String>("direction")
                .map(|s| s.as_str())
                .unwrap();

            let part = get_part_from_matches(sub_matches);

            let mut file = fs::File::open(input_file).map_err(CLIError::from)?;
            let mut file_buf = Vec::new();
            file.read_to_end(&mut file_buf).map_err(CLIError::from)?;
            let file_str = String::from_utf8_lossy(&file_buf[..]);
            let mut firmware = from_ihex(&file_str, part.firmware_size + part.bootloader_size)
                .map_err(CLIError::from)?;

            if firmware.len() < part.firmware_size {
                log::warn!(
                    "Firmware size is more than expected ({}). Increasing to {}",
                    firmware.len(),
                    part.firmware_size
                );
                firmware.resize(part.firmware_size, 0);
            }

            match direction {
                "to_jtag" => {
                    convert_to_jtag_payload(&mut firmware, part).map_err(CLIError::from)?;
                    if firmware.len() < part.total_flash_size() {
                        log::warn!(
                            "Firmware is smaller ({} bytes) than expected ({} bytes). This payload might not be suitable for JTAG flashing.",
                            firmware.len(),
                            part.total_flash_size()
                        );
                    }
                }
                "to_isp" => {
                    convert_to_isp_payload(&mut firmware, part).map_err(CLIError::from)?;
                    if firmware.len() > part.firmware_size {
                        log::warn!(
                            "Firmware size is larger ({} bytes) than expected ({} bytes). This payload might not be suitable for ISP flashing.",
                            firmware.len(),
                            part.firmware_size
                        );
                    }
                }
                _ => unreachable!(),
            }

            let ihex = to_ihex(firmware).map_err(CLIError::from)?;
            fs::write(output_file, ihex).map_err(CLIError::from)?;
        }
        _ => unreachable!(),
    }
    Ok(())
}

trait PartCommand {
    fn part_args(self) -> Command;
}

impl PartCommand for Command {
    fn part_args(self) -> Command {
        self.arg(
            arg!(-p --part <PART>)
                .value_parser(Part::available_parts())
                .required_unless_present_all(["firmware_size", "vendor_id", "product_id"]),
        )
        .arg(
            arg!(--firmware_size <SIZE>)
                .required_unless_present("part")
                .value_parser(maybe_hex::<usize>),
        )
        .arg(
            arg!(--vendor_id <VID>)
                .required_unless_present("part")
                .value_parser(maybe_hex::<u16>),
        )
        .arg(
            arg!(--product_id <PID>)
                .required_unless_present("part")
                .value_parser(maybe_hex::<u16>),
        )
        .arg(arg!(--bootloader_size <SIZE>).value_parser(maybe_hex::<usize>))
        .arg(arg!(--page_size <SIZE>).value_parser(maybe_hex::<usize>))
        .arg(arg!(--isp_iface_num <NUM>).value_parser(clap::value_parser!(i32)))
        .arg(arg!(--isp_report_id <USAGE>).value_parser(maybe_hex::<u32>))
        .arg(arg!(--reboot <BOOL>).value_parser(value_parser!(bool)))
    }
}

fn get_part_from_matches(sub_matches: &ArgMatches) -> Part {
    let part_name = sub_matches.get_one::<String>("part").map(|s| s.as_str());

    let mut part = match part_name {
        Some(part_name) => *PARTS.get(part_name).unwrap(),
        _ => PART_BASE_DEFAULT,
    };

    let firmware_size = sub_matches.get_one::<usize>("firmware_size");
    let bootloader_size = sub_matches.get_one::<usize>("bootloader_size");
    let page_size = sub_matches.get_one::<usize>("page_size");
    let vendor_id = sub_matches.get_one::<u16>("vendor_id");
    let product_id = sub_matches.get_one::<u16>("product_id");
    let isp_iface_num = sub_matches.get_one::<i32>("isp_iface_num");
    let isp_report_id = sub_matches.get_one::<u32>("isp_report_id");
    let reboot = sub_matches.get_one::<bool>("reboot");

    if let Some(firmware_size) = firmware_size {
        part.firmware_size = *firmware_size;
    }
    if let Some(vendor_id) = vendor_id {
        part.vendor_id = *vendor_id;
    }
    if let Some(product_id) = product_id {
        part.product_id = *product_id;
    }
    if let Some(bootloader_size) = bootloader_size {
        part.bootloader_size = *bootloader_size;
    }
    if let Some(page_size) = page_size {
        part.page_size = *page_size;
    }
    if let Some(isp_iface_num) = isp_iface_num {
        part.isp_iface_num = *isp_iface_num;
    }
    if let Some(isp_report_id) = isp_report_id {
        part.isp_report_id = *isp_report_id;
    }
    if let Some(reboot) = reboot {
        part.reboot = *reboot;
    }
    part
}
