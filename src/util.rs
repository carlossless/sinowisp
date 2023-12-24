use thiserror::Error;

#[derive(Debug, Clone, Error, PartialEq)]
pub enum VerificationError {
    #[error("Firmware Mismatch @ {addr:#06x} --- {expected:#04x} != {actual:#04x}")]
    ByteMismatch {
        addr: usize,
        expected: u8,
        actual: u8,
    },
    #[error("Length Mismatch {expected} {actual}")]
    LengthMismatch { expected: usize, actual: usize },
}

pub fn verify(expected: &Vec<u8>, actual: &Vec<u8>) -> Result<(), VerificationError> {
    if expected.len() != actual.len() {
        return Err(VerificationError::LengthMismatch {
            expected: expected.len(),
            actual: actual.len(),
        });
    }

    for i in 0..expected.len() {
        if expected[i] != actual[i] {
            return Err(VerificationError::ByteMismatch {
                addr: i,
                expected: expected[i],
                actual: actual[i],
            });
        }
    }

    Ok(())
}

#[test]
fn test_verify_success() {
    assert!(verify(&vec![1, 2, 3, 4], &vec![1, 2, 3, 4]).is_ok());
}

#[test]
fn test_verify_error_length_mismatch() {
    assert_eq!(
        verify(&vec![1, 2, 3, 4], &vec![1, 2, 3]),
        Err(VerificationError::LengthMismatch {
            expected: 4,
            actual: 3
        })
    );
}

#[test]
fn test_verify_error_byte_mismatch() {
    assert_eq!(
        verify(&vec![1, 2, 3, 4], &vec![1, 2, 4, 3]),
        Err(VerificationError::ByteMismatch {
            addr: 2,
            expected: 3,
            actual: 4
        })
    );
}
