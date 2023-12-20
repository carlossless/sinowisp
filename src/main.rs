use std::{
    fs,
    io::{self, Read},
    process::ExitCode,
};

use clap::{arg, ArgMatches, Command};
use clap_num::maybe_hex;
use log::{error, info};
use simple_logger::SimpleLogger;
use thiserror::Error;

mod isp;
mod part;
// mod hid;
mod ihex;
mod util;

pub use crate::{ihex::*, isp::*, part::*, util::*};

#[derive(Debug, Error)]
pub enum CLIError {
    #[error(transparent)]
    IOError(#[from] io::Error),
    #[error(transparent)]
    ISPError(#[from] ISPError),
    #[error(transparent)]
    IHEXError(#[from] ConversionError),
}

fn main() -> ExitCode {
    match err_main() {
        Ok(_) => ExitCode::SUCCESS,
        Err(e) => {
            error!("{}", e.to_string());
            ExitCode::FAILURE
        }
    }
}

fn cli() -> Command {
    return Command::new("sinowealth-kb-tool")
        .about("A programming tool for Sinowealth Gaming KB devices")
        .version(env!("CARGO_PKG_VERSION"))
        .subcommand_required(true)
        .arg_required_else_help(true)
        .author("Karolis Stasaitis")
        .subcommand(
            Command::new("read")
                .short_flag('r')
                .about("Read flash contents. (Intel HEX)")
                .arg(arg!(output_file: <OUTPUT_FILE> "file to write flash contents to"))
                .part_args()
                .arg(arg!(-b --bootloader --isp "read only booloader").conflicts_with("full"))
                .arg(
                    arg!(--full "read complete flash (including the bootloader)")
                        .conflicts_with("bootloader"),
                ),
        )
        .subcommand(
            Command::new("write")
                .short_flag('w')
                .about("Write file (Intel HEX) into flash.")
                .arg(arg!(input_file: <INPUT_FILE> "payload to write into flash"))
                .part_args(),
        );
}

fn err_main() -> Result<(), CLIError> {
    SimpleLogger::new().init().unwrap();

    let matches = cli().get_matches();

    match matches.subcommand() {
        Some(("read", sub_matches)) => {
            let output_file = sub_matches
                .get_one::<String>("output_file")
                .map(|s| s.as_str())
                .unwrap();

            let full = sub_matches.get_flag("full");

            let bootloader = sub_matches.get_flag("bootloader");

            let part = get_part_from_matches(sub_matches);

            let read_type = match (full, bootloader) {
                (true, _) => ReadType::Full,
                (_, true) => ReadType::Bootloader,
                _ => ReadType::Normal,
            };

            let isp = ISPDevice::new(part).map_err(CLIError::from)?;
            let result = isp.read_cycle(read_type).map_err(CLIError::from)?;

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

            let part = get_part_from_matches(sub_matches);

            let mut file = fs::File::open(input_file).map_err(CLIError::from)?;
            let mut file_buf = Vec::new();
            file.read_to_end(&mut file_buf).map_err(CLIError::from)?;
            let file_str = String::from_utf8_lossy(&file_buf[..]);
            let mut firmware = from_ihex(&file_str, part.firmware_size).map_err(CLIError::from)?;

            if firmware.len() < part.firmware_size {
                firmware.resize(part.firmware_size, 0);
            }

            let isp = ISPDevice::new(part).map_err(CLIError::from)?;
            isp.write_cycle(&mut firmware).map_err(CLIError::from)?;
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
                .required_unless_present_all([
                    "firmware_size",
                    "bootloader_size",
                    "page_size",
                    "vendor_id",
                    "product_id",
                    "isp_index"
                ]),
        )
        .arg(
            arg!(--firmware_size <SIZE>)
                .required_unless_present("part")
                .value_parser(clap::value_parser!(usize)),
        )
        .arg(
            arg!(--bootloader_size <SIZE>)
                .required_unless_present("part")
                .value_parser(clap::value_parser!(usize)),
        )
        .arg(
            arg!(--page_size <SIZE>)
                .required_unless_present("part")
                .value_parser(clap::value_parser!(usize)),
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
        .arg(
            arg!(--isp_index <PID>)
                .required_unless_present("part")
                .value_parser(clap::value_parser!(usize)),
        )
    }
}

fn get_part_from_matches(sub_matches: &ArgMatches) -> Part {
    let part_name = sub_matches.get_one::<String>("part").map(|s| s.as_str());

    let mut part = match part_name {
        Some(part_name) => *PARTS.get(part_name).unwrap(),
        _ => Part::default(),
    };

    let firmware_size = sub_matches.get_one::<usize>("firmware_size");
    let bootloader_size = sub_matches.get_one::<usize>("bootloader_size");
    let page_size = sub_matches.get_one::<usize>("page_size");
    let vendor_id = sub_matches.get_one::<u16>("vendor_id");
    let product_id = sub_matches.get_one::<u16>("product_id");
    let isp_index = sub_matches.get_one::<usize>("isp_index");

    if let Some(firmware_size) = firmware_size {
        part.firmware_size = *firmware_size;
    }
    if let Some(bootloader_size) = bootloader_size {
        part.bootloader_size = *bootloader_size;
    }
    if let Some(page_size) = page_size {
        part.page_size = *page_size;
    }
    if let Some(vendor_id) = vendor_id {
        part.vendor_id = *vendor_id;
    }
    if let Some(product_id) = product_id {
        part.product_id = *product_id;
    }
    if let Some(isp_index) = isp_index {
        part.isp_index = *isp_index;
    }
    return part;
}
