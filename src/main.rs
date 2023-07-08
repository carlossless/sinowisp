use clap::*;
use ihex::*;
use ihex_ext::*;
use log::*;
use simple_logger::SimpleLogger;
use std::{fs, process};

mod part;
pub use part::*;

mod isp;
pub use isp::*;

mod util;
pub use util::*;

mod hid;
pub use hid::*;

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

            let ihex = result.to_ihex();

            let obj = create_object_file_representation(&ihex).unwrap();

            fs::write(output_file, obj).expect("Unable to write file");
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

            let (mut firmware, _) = load_file_vec(input_file, part.flash_size, 0).unwrap();

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
