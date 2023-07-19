use clap::*;
use log::*;
use simple_logger::SimpleLogger;
use std::io::Read;
use std::{fs, process};

mod part;
mod isp;
mod hid;
mod util;
mod ihex;

pub use crate::part::*;
pub use crate::isp::*;
pub use crate::hid::*;
pub use crate::ihex::*;
pub use crate::util::*;

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
                .arg(
                    arg!(-p --part <PART>)
                        .value_parser(PARTS.keys().map(|&s| s).collect::<Vec<_>>())
                        .required(true),
                )
                .arg(arg!(-b --bootloader "read only booloader").conflicts_with("full"))
                .arg(
                    arg!(-f --full "read complete flash (including the bootloader)")
                        .conflicts_with("bootloader"),
                ),
        )
        .subcommand(
            Command::new("write")
                .short_flag('w')
                .about("Write file (Intel HEX) into flash.")
                .arg(arg!(input_file: <INPUT_FILE> "payload to write into flash"))
                .arg(
                    arg!(-p --part <PART>)
                        .value_parser(PARTS.keys().map(|&s| s).collect::<Vec<_>>())
                        .required(true),
                ),
        );
}

fn main() {
    SimpleLogger::new().init().unwrap();

    let matches = cli().get_matches();

    match matches.subcommand() {
        Some(("read", sub_matches)) => {
            let part_name = sub_matches
                .get_one::<String>("part")
                .map(|s| s.as_str())
                .unwrap();

            let output_file = sub_matches
                .get_one::<String>("output_file")
                .map(|s| s.as_str())
                .unwrap();

            let full = sub_matches.get_flag("full");

            let bootloader = sub_matches.get_flag("bootloader");

            let part = PARTS.get(part_name).unwrap();

            let read_type = match (full, bootloader) {
                (true, _) => ReadType::Full,
                (_, true) => ReadType::Bootloader,
                _ => ReadType::Normal,
            };

            let result = ISPDevice::new(part).read_cycle(read_type);

            let digest = md5::compute(&result);
            println!("MD5: {:x}", digest);

            let ihex = to_ihex(result).expect("Failed converting to IHEX");
            fs::write(output_file, ihex).expect("Unable to write file");
        }
        Some(("write", sub_matches)) => {
            let input_file = sub_matches
                .get_one::<String>("input_file")
                .map(|s| s.as_str())
                .unwrap();

            let part_name = sub_matches
                .get_one::<String>("part")
                .map(|s| s.as_str())
                .unwrap();

            let part = PARTS.get(part_name).unwrap();

            let mut file = fs::File::open(input_file).unwrap();
            let mut file_buf = Vec::new();
            file.read_to_end(&mut file_buf).unwrap();
            let file_str = String::from_utf8_lossy(&file_buf[..]);
            let mut firmware = from_ihex(&file_str, part.flash_size).unwrap();

            if firmware.len() < part.flash_size {
                firmware.resize(part.flash_size, 0);
            }

            match ISPDevice::new(part).write_cycle(&mut firmware) {
                Err(e) => {
                    error!("{}", e.to_message());
                    process::exit(1);
                }
                Ok(_) => {}
            };
        }
        Some(("erase", sub_matches)) => {
            let part_name = sub_matches
                .get_one::<String>("part")
                .map(|s| s.as_str())
                .unwrap();

            let part = PARTS.get(part_name).unwrap();

            ISPDevice::new(part).erase_cycle();
        }
        _ => unreachable!(),
    }
}
