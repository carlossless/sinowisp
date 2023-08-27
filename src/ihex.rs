use ihex::*;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum UnpackingError {
    #[error("Unsupported record type")]
    UnsupportedRecordType(Record),
    #[error("Error while parsing IHEX records")]
    Parsing(#[from] ReaderError),
    #[error("Address ({0}) greater than binary size ({1})")]
    AddressTooHigh(usize, usize),
}

#[derive(Debug, Error)]
pub enum ConversionError {
    #[error("Error while unpacking IHEX into array")]
    Unpacking(#[from] UnpackingError),
    #[error("Errow while writing IHEX to string")]
    Serializing(#[from] WriterError),
}

pub fn to_ihex(byte_array: Vec<u8>) -> Result<String, ConversionError> {
    let mut result: Vec<Record> = vec![];
    for (i, chunk) in byte_array.chunks(16).enumerate() {
        result.push(Record::Data {
            offset: (i as u16) * 16,
            value: chunk.to_vec(),
        });
    }
    result.push(Record::EndOfFile);
    create_object_file_representation(&result).map_err(ConversionError::from)
}

pub fn from_ihex(ihex_string: &str, max_length: usize) -> Result<Vec<u8>, ConversionError> {
    let mut reader = Reader::new(ihex_string);
    unpack_records(&mut reader, max_length).map_err(ConversionError::from)
}

fn unpack_records(
    records: &mut impl Iterator<Item = Result<Record, ReaderError>>,
    max_length: usize,
) -> Result<Vec<u8>, UnpackingError> {
    let mut result: Vec<u8> = vec![];
    for rec in records {
        match rec {
            Ok(rec) => match rec {
                Record::Data { offset, value } => {
                    let end_addr = offset as usize + value.len();
                    if end_addr > max_length {
                        return Err(UnpackingError::AddressTooHigh(end_addr, max_length));
                    }
                    if end_addr > result.len() {
                        result.resize(end_addr, 0);
                    }

                    for (n, b) in value.iter().enumerate() {
                        result[offset as usize + n] = *b;
                    }
                }
                Record::ExtendedSegmentAddress(_base) => {
                    return Err(UnpackingError::UnsupportedRecordType(rec))
                }
                Record::ExtendedLinearAddress(_base) => {
                    return Err(UnpackingError::UnsupportedRecordType(rec))
                }
                Record::EndOfFile => break,
                Record::StartLinearAddress(_) | Record::StartSegmentAddress { .. } => {}
            },
            Err(err) => return Err(UnpackingError::Parsing(err)),
        }
    }
    Ok(result)
}
