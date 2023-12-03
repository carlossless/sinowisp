use ihex::{create_object_file_representation, Reader, ReaderError, Record, WriterError};
use thiserror::Error;

#[derive(Debug, Error, PartialEq)]
pub enum UnpackingError {
    #[error("Unsupported record type")]
    UnsupportedRecordType(Record),
    #[error("Error while parsing IHEX records")]
    Parsing(#[from] ReaderError),
    #[error("Address {addr:#06x} greater than binary size {size:#06x}")]
    AddressTooHigh { addr: usize, size: usize },
}

#[derive(Debug, Error, PartialEq)]
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
                        return Err(UnpackingError::AddressTooHigh {
                            addr: end_addr,
                            size: max_length,
                        });
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

#[test]
fn test_from_ihex() {
    let result = from_ihex(
        ":100000000200660227BD010A32646402CB9053DA13\n:00000001FF",
        16,
    )
    .unwrap();
    let expected = vec![
        2, 0, 102, 2, 39, 189, 1, 10, 50, 100, 100, 2, 203, 144, 83, 218,
    ];
    assert_eq!(result, expected);
}

#[test]
fn test_from_ihex_address_start_at_0x0001() {
    let result = from_ihex(
        ":100010000200660227BD010A32646402CB9053DA03\n:00000001FF",
        32,
    );
    let mut expected: Vec<u8> = Vec::new();
    expected.resize(16, 0);
    expected.extend_from_slice(&[
        2, 0, 102, 2, 39, 189, 1, 10, 50, 100, 100, 2, 203, 144, 83, 218,
    ]);
    assert_eq!(result, Ok(expected));
}

#[test]
fn test_from_ihex_err_checksum_mismatch() {
    let result = from_ihex(
        ":100000000200660227BD010A32646402CB9053DA00\n:00000001FF",
        16,
    );
    let expected = Err(ConversionError::Unpacking(UnpackingError::Parsing(
        ReaderError::ChecksumMismatch(0x13, 0x00),
    )));
    assert_eq!(result, expected);
}

#[test]
fn test_from_ihex_err_address_too_high() {
    let result = from_ihex(
        ":100010000200660227BD010A32646402CB9053DA03\n:00000001FF",
        16,
    );
    let expected = Err(ConversionError::Unpacking(UnpackingError::AddressTooHigh {
        addr: 0x20,
        size: 0x10,
    }));
    assert_eq!(result, expected);
}
