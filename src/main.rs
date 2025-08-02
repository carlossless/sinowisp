use std::{
    env, fs,
    io::{self, Read},
    path::Path,
    process::ExitCode,
    str::FromStr,
};

use clap::{arg, value_parser, ArgMatches, Command};
use clap_num::maybe_hex;
use device_selector::{DeviceSelector, DeviceSelectorError};
use dialoguer::Confirm;
use hid_tree::TreeDisplay;
use log::error;
use platform_spec::PlatformSpec;
use simple_logger::SimpleLogger;
use thiserror::Error;

mod device_selector;
mod device_spec;
mod hid_tree;
mod ihex;
mod isp_device;
mod platform_spec;
mod util;

pub use crate::{device_spec::*, ihex::*, isp_device::*, util::*};

const DEFAULT_RETRY_COUNT: &str = "5";

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

#[derive(Clone, Copy)]
enum Format {
    IntelHex,
    Binary,
}

impl Format {
    pub fn to_str(self) -> &'static str {
        match self {
            Format::IntelHex => "ihex",
            Format::Binary => "bin",
        }
    }

    pub fn available_formats() -> Vec<&'static str> {
        vec![Format::IntelHex.to_str(), Format::Binary.to_str()]
    }
}

impl FromStr for Format {
    type Err = ();
    fn from_str(format: &str) -> Result<Self, Self::Err> {
        Ok(match format {
            "ihex" => Format::IntelHex,
            "bin" => Format::Binary,
            _ => panic!("Invalid format: {}", format),
        })
    }
}

fn main() -> ExitCode {
    match err_main() {
        Ok(_) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("{:}", err);
            ExitCode::FAILURE
        }
    }
}

fn cli() -> Command {
    Command::new("sinowealth-kb-tool")
        .about("A tool to read and write flash for SinoWealth ISP devices")
        .version(env!("CARGO_PKG_VERSION"))
        .subcommand_required(true)
        .arg_required_else_help(true)
        .author("Karolis Stasaitis")
        .subcommand(
            Command::new("list")
                .about("List all connected usb hid devices and info about them.")
                .arg(arg!(--vendor_id <VID>).value_parser(maybe_hex::<u16>))
                .arg(arg!(--product_id <PID>).value_parser(maybe_hex::<u16>)),
        )
        .subcommand(
            Command::new("read")
                .about("Read flash into a file.")
                .arg(arg!(output_file: <OUTPUT_FILE> "file to write flash contents to"))
                .arg(arg!(--format <FORMAT>).value_parser(Format::available_formats()))
                .arg(
                    arg!(-s --section <SECTION> "firmware section to read")
                        .value_parser(ReadSection::available_sections())
                        .default_value(ReadSection::Firmware.to_str()),
                )
                .arg(
                    arg!(-r --retry <NUM> "number of attempts trying to find device")
                        .value_parser(value_parser!(usize))
                        .default_value(DEFAULT_RETRY_COUNT),
                )
                .device_args(),
        )
        .subcommand(
            Command::new("write")
                .about("Write a file into flash.")
                .arg(arg!(input_file: <INPUT_FILE> "payload to write into flash"))
                .arg(arg!(-f --force "ignore firmware size check"))
                .arg(arg!(--format <FORMAT>).value_parser(Format::available_formats()))
                .arg(
                    arg!(-r --retry <NUM> "number of attempts trying to find device")
                        .value_parser(value_parser!(usize))
                        .default_value(DEFAULT_RETRY_COUNT),
                )
                .device_args(),
        )
        .subcommand(
            Command::new("convert")
                .about("Convert payload from ISP to JTAG and vice versa.")
                .arg(
                    arg!(--direction <DIRECTION> "direction of conversion")
                        .value_parser(["to_jtag", "to_isp"])
                        .required(true),
                )
                .arg(arg!(--input_format <FORMAT>).value_parser(Format::available_formats()))
                .arg(arg!(--output_format <FORMAT>).value_parser(Format::available_formats()))
                .arg(arg!(input_file: <INPUT_FILE> "file to convert"))
                .arg(arg!(output_file: <OUTPUT_FILE> "file to write results to"))
                .device_args(), // TODO: not all of these args are needed and should be removed
        )
}

fn err_main() -> Result<(), CLIError> {
    SimpleLogger::new()
        .with_utc_timestamps()
        .with_level(log::LevelFilter::Off)
        .env()
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

            let section = sub_matches
                .get_one::<String>("section")
                .map(|s| s.as_str())
                .map(|s| ReadSection::from_str(s).unwrap())
                .unwrap();

            let format = get_format_from_matches(sub_matches, output_file, "format");

            let device_spec = get_device_spec_from_matches(sub_matches);

            let mut ds = DeviceSelector::new().map_err(CLIError::DeviceSelectorError)?;
            let device = ds
                .try_fetch_isp_device(device_spec, retry_count)
                .map_err(CLIError::from)?;
            let firmware = device.read_cycle(section).map_err(CLIError::from)?;

            let digest = md5::compute(&firmware);
            eprintln!("MD5: {:x}", digest);

            write_with_format(output_file, &firmware, format)?;

            eprintln!(
                "Successfully read {} bytes - {}",
                firmware.len(),
                output_file
            );
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

            let force = sub_matches.get_flag("force");

            let format = get_format_from_matches(sub_matches, input_file, "format");

            let device_spec = get_device_spec_from_matches(sub_matches);

            let mut firmware = read_with_format(input_file, format)?;

            if firmware.len() < device_spec.platform.firmware_size {
                eprintln!(
                    "Warning: firmware size is less than expected ({}). It will be resized to {} and filled with 0",
                    firmware.len(),
                    device_spec.platform.firmware_size
                );
                if !force {
                    eprintln!("Use --force skip confirmation");
                    let confirmation = Confirm::new()
                        .with_prompt("Are you sure you want to continue?")
                        .default(false)
                        .interact()
                        .unwrap();

                    if !confirmation {
                        return Ok(());
                    }
                }
                firmware.resize(device_spec.platform.firmware_size, 0);
            }

            let mut ds = DeviceSelector::new().map_err(CLIError::DeviceSelectorError)?;
            let device = ds
                .try_fetch_isp_device(device_spec, retry_count)
                .map_err(CLIError::from)?;
            device.write_cycle(&mut firmware).map_err(CLIError::from)?;

            eprintln!("Successfully wrote {} bytes", firmware.len());
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

            let input_format = get_format_from_matches(sub_matches, input_file, "input_format");
            let output_format = get_format_from_matches(sub_matches, output_file, "output_format");

            let device_spec = get_device_spec_from_matches(sub_matches);

            let mut firmware = read_with_format(input_file, input_format)?;

            if firmware.len() < device_spec.platform.firmware_size {
                log::warn!(
                    "Firmware size is less than expected ({}). Increasing to {}",
                    firmware.len(),
                    device_spec.platform.firmware_size
                );
                firmware.resize(device_spec.platform.firmware_size, 0);
            }

            match direction {
                "to_jtag" => {
                    convert_to_jtag_payload(&mut firmware, device_spec).map_err(CLIError::from)?;
                    if firmware.len() < device_spec.total_flash_size() {
                        eprintln!(
                            "Firmware is smaller ({} bytes) than expected ({} bytes). This payload might not be suitable for JTAG flashing.",
                            firmware.len(),
                            device_spec.total_flash_size()
                        );
                    }
                }
                "to_isp" => {
                    convert_to_isp_payload(&mut firmware, device_spec).map_err(CLIError::from)?;
                    if firmware.len() > device_spec.platform.firmware_size {
                        eprintln!(
                            "Firmware size is larger ({} bytes) than expected ({} bytes). This payload might not be suitable for ISP flashing.",
                            firmware.len(),
                            device_spec.platform.firmware_size
                        );
                    }
                }
                _ => unreachable!(),
            }

            write_with_format(output_file, &firmware, output_format)?;
        }
        _ => unreachable!(),
    }
    Ok(())
}

trait DeviceCommand {
    fn device_args(self) -> Command;
}

impl DeviceCommand for Command {
    fn device_args(self) -> Command {
        self.arg(
            arg!(-d --device <DEVICE>)
                .required_unless_present_all(["platform", "vendor_id", "product_id"])
                .value_parser(DeviceSpec::available_devices()),
        )
        .arg(
            arg!(-p --platform <PLATFORM>)
                .value_parser(PlatformSpec::available_platforms())
                .required_unless_present("device"),
        )
        .arg(
            arg!(--vendor_id <VID>)
                .required_unless_present("device")
                .value_parser(maybe_hex::<u16>),
        )
        .arg(
            arg!(--product_id <PID>)
                .required_unless_present("device")
                .value_parser(maybe_hex::<u16>),
        )
        .arg(
            arg!(--firmware_size <SIZE>)
                .required_unless_present_any(["device", "platform"])
                .value_parser(maybe_hex::<usize>),
        )
        .arg(arg!(--bootloader_size <SIZE>).value_parser(maybe_hex::<usize>))
        .arg(arg!(--page_size <SIZE>).value_parser(maybe_hex::<usize>))
        .arg(arg!(--isp_iface_num <NUM>).value_parser(clap::value_parser!(i32)))
        .arg(arg!(--isp_report_id <USAGE>).value_parser(maybe_hex::<u32>))
        .arg(arg!(--reboot <BOOL>).value_parser(value_parser!(bool)))
    }
}

fn get_format_from_matches(
    sub_matches: &ArgMatches,
    file_path: &str,
    format_option: &str,
) -> Format {
    let input_ext = Path::new(file_path).extension();

    let assumed_format = input_ext
        .map(|ext| {
            if ext == "ihex" || ext == "ihx" || ext == "hex" {
                Format::IntelHex
            } else {
                Format::Binary
            }
        })
        .unwrap_or(Format::Binary);

    let format = sub_matches
        .get_one::<String>(format_option)
        .map(|s| s.as_str())
        .map(|f| Format::from_str(f).unwrap())
        .unwrap_or(assumed_format);

    match (assumed_format, format) {
        (Format::IntelHex, Format::Binary) => {
            eprintln!(
                "Warning: binary file has {} extension. This might be unintended.",
                input_ext.unwrap().to_string_lossy()
            );
        }
        (Format::Binary, Format::IntelHex) => {
            eprintln!("Warning: ihex file does not have .ihex or .ihx or .hex extension. This might be unintended.");
        }
        _ => {}
    }

    format
}

fn get_device_spec_from_matches(sub_matches: &ArgMatches) -> DeviceSpec {
    let device_name = sub_matches.get_one::<String>("device").map(|s| s.as_str());
    let platform_name = sub_matches
        .get_one::<String>("platform")
        .map(|s| s.as_str());

    let vendor_id = sub_matches.get_one::<u16>("vendor_id");
    let product_id = sub_matches.get_one::<u16>("product_id");

    let firmware_size = sub_matches.get_one::<usize>("firmware_size");
    let bootloader_size = sub_matches.get_one::<usize>("bootloader_size");
    let page_size = sub_matches.get_one::<usize>("page_size");

    let isp_iface_num = sub_matches.get_one::<i32>("isp_iface_num");
    let isp_report_id = sub_matches.get_one::<u32>("isp_report_id");
    let reboot = sub_matches.get_one::<bool>("reboot");

    let mut device_spec = None;
    if let Some(device_name) = device_name {
        device_spec = Some(*DEVICES.get(device_name).unwrap());
    }

    if let Some(platform_name) = platform_name {
        device_spec = match platform_name {
            "sh68f90" => Some(DEVICE_BASE_SH68F90),
            "sh68f881" => Some(DEVICE_BASE_SH68F881),
            _ => panic!("Invalid platform"),
        }
    }

    let mut device_spec = device_spec.unwrap();

    if let Some(vendor_id) = vendor_id {
        device_spec.vendor_id = *vendor_id;
    }
    if let Some(product_id) = product_id {
        device_spec.product_id = *product_id;
    }

    if let Some(firmware_size) = firmware_size {
        device_spec.platform.firmware_size = *firmware_size;
    }
    if let Some(bootloader_size) = bootloader_size {
        device_spec.platform.bootloader_size = *bootloader_size;
    }
    if let Some(page_size) = page_size {
        device_spec.platform.page_size = *page_size;
    }

    if let Some(isp_iface_num) = isp_iface_num {
        device_spec.isp_iface_num = *isp_iface_num;
    }
    if let Some(isp_report_id) = isp_report_id {
        device_spec.isp_report_id = *isp_report_id;
    }
    if let Some(reboot) = reboot {
        device_spec.reboot = *reboot;
    }

    device_spec
}

fn read_with_format(file: &str, format: Format) -> Result<Vec<u8>, CLIError> {
    let mut file = fs::File::open(file).map_err(CLIError::from)?;
    let mut file_buf = Vec::new();
    file.read_to_end(&mut file_buf).map_err(CLIError::from)?;

    match format {
        Format::IntelHex => {
            let file_str = String::from_utf8_lossy(&file_buf[..]);
            from_ihex(&file_str, 0xFFFF).map_err(CLIError::from) // TODO reasonable length
        }
        Format::Binary => Ok(file_buf),
    }
}

fn write_with_format(file: &str, data: &[u8], format: Format) -> Result<(), CLIError> {
    match format {
        Format::IntelHex => {
            let ihex = to_ihex(data).map_err(CLIError::from)?;
            fs::write(file, ihex).map_err(CLIError::from)
        }
        Format::Binary => fs::write(file, data).map_err(CLIError::from),
    }
}
