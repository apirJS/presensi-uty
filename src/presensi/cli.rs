use base64::Engine as _;
use clap::Parser;

use crate::{
    error::ValidationError,
    presensi::types::{Account, Nim, Password, Subject, Week},
};

#[derive(Parser)]
#[command(version = "1.0.0", about = "Inject Presensi UTY")]
pub struct Args {
    #[arg(
        long,
        required_unless_present = "id_matkul",
        conflicts_with = "id_matkul"
    )]
    pub presensi_lama: Option<String>,

    #[arg(
        long,
        required_unless_present = "presensi_lama",
        conflicts_with = "presensi_lama"
    )]
    pub id_matkul: Option<String>,

    #[arg(short, long)]
    pub minggu: String,

    #[arg(short, long)]
    pub nim: String,

    #[arg(short, long)]
    pub password: String,
}

impl Args {
    pub fn subject(&self) -> Result<Subject, ValidationError> {
        if let Some(id) = &self.id_matkul {
            if id.len() != 6 || !id.chars().all(|d| d.is_ascii_digit()) {
                return Err(ValidationError::InvalidSubjectId);
            }

            Ok(Subject::SubjectId(id.clone()))
        } else if let Some(attendance_code) = &self.presensi_lama {
            let decrypted = extract_subject_id(attendance_code)?;
            Ok(Subject::OldAttendanceCode(decrypted))
        } else {
            unreachable!()
        }
    }

    pub fn weeks(&self) -> Result<Vec<Week>, ValidationError> {
        self.minggu
            .split(',')
            .map(|s| {
                let s = s.trim();
                let n: u8 = s.parse().map_err(|_| ValidationError::InvalidWeek)?;
                if n < 1 || n > 14 {
                    return Err(ValidationError::InvalidWeek);
                }
                Ok(Week(s.to_string()))
            })
            .collect()
    }

    pub fn account(&self) -> Result<Account, ValidationError> {
        if !self.nim.chars().all(|d| d.is_ascii_digit()) {
            return Err(ValidationError::InvalidNim);
        }

        Ok(Account {
            nim: Nim(self.nim.clone()),
            password: Password(self.password.clone()),
        })
    }
}

fn extract_subject_id(attendance_code: &str) -> Result<String, ValidationError> {
    let secret = b"utyjombor123";

    let encrypted = base64::engine::general_purpose::STANDARD
        .decode(attendance_code)
        .map_err(|_| ValidationError::InvalidAttendanceCode)?;

    if encrypted.len() < 16 || &encrypted[..8] != b"Salted__" {
        return Err(ValidationError::InvalidAttendanceCode);
    }

    let salt = &encrypted[8..16];
    let ciphertext = &encrypted[16..];

    let (key, iv) = evp_bytes_to_key(secret, salt);

    use aes::cipher::{BlockDecryptMut, KeyIvInit, block_padding::Pkcs7};
    type Aes256CbcDec = cbc::Decryptor<aes::Aes256>;

    let mut buf = ciphertext.to_vec();
    let decrypted = Aes256CbcDec::new(&key.into(), &iv.into())
        .decrypt_padded_mut::<Pkcs7>(&mut buf)
        .map_err(|_| ValidationError::InvalidAttendanceCode)?;

    let plaintext =
        std::str::from_utf8(decrypted).map_err(|_| ValidationError::InvalidAttendanceCode)?;

    let id = plaintext
        .split(',')
        .next()
        .ok_or(ValidationError::InvalidAttendanceCode)?;

    Ok(id.to_string())
}

/// OpenSSL EVP_BytesToKey - same KDF CryptoJS uses
fn evp_bytes_to_key(password: &[u8], salt: &[u8]) -> ([u8; 32], [u8; 16]) {
    use md5::Digest;
    let mut d: Vec<u8> = Vec::new();
    let mut prev: Vec<u8> = Vec::new();

    while d.len() < 48 {
        let mut hasher = md5::Md5::new();
        hasher.update(&prev);
        hasher.update(password);
        hasher.update(salt);
        prev = hasher.finalize().to_vec();
        d.extend_from_slice(&prev);
    }

    let mut key = [0u8; 32];
    let mut iv = [0u8; 16];
    key.copy_from_slice(&d[..32]);
    iv.copy_from_slice(&d[32..48]);
    (key, iv)
}

// -- Tests
#[cfg(test)]
mod tests {
    mod util {
        use crate::presensi::cli::extract_subject_id;

        #[test]
        fn returns_subject_id_from_valid_code() {
            let code = "U2FsdGVkX19xJlSDxwXghX/OIKvlFrU/zymK5IWts1+9zVuiarSWbf1nu4c0n8PSYCmVNpg7w4gW9BfiekIndg==";
            let result = extract_subject_id(code);
            assert_eq!(Ok("120184".to_string()), result);
        }

        #[test]
        fn rejects_invalid_base64() {
            let result = extract_subject_id("not!valid!base64!!!");
            assert!(result.is_err());
        }

        #[test]
        fn rejects_missing_salted_prefix() {
            // valid base64 but no "Salted__" header
            let result = extract_subject_id("aGVsbG8gd29ybGQ="); // "hello world"
            assert!(result.is_err());
        }
    }

    mod args {
        use crate::presensi::cli::{Args, default_args};

        mod subject {
            use super::*;

            #[test]
            fn accepts_valid_six_digit_id_matkul() {
                let args = default_args();
                assert!(args.subject().is_ok());
            }

            #[test]
            fn rejects_id_matkul_with_less_than_6_digits() {
                let args = Args {
                    id_matkul: Some("1234".to_string()),
                    ..default_args()
                };
                assert!(args.subject().is_err());
            }

            #[test]
            fn rejects_id_matkul_with_more_than_6_digits() {
                let args = Args {
                    id_matkul: Some("1234567".to_string()),
                    ..default_args()
                };
                assert!(args.subject().is_err());
            }

            #[test]
            fn rejects_id_matkul_with_non_digits() {
                let args = Args {
                    id_matkul: Some("12AB56".to_string()),
                    ..default_args()
                };
                assert!(args.subject().is_err());
            }

            #[test]
            fn rejects_invalid_attendance_code() {
                let args = Args {
                    presensi_lama: Some("WRONG!".to_string()),
                    id_matkul: None,
                    ..default_args()
                };
                assert!(args.subject().is_err());
            }

            #[test]
            fn accepts_valid_attendance_code() {
                let args = Args {
                    presensi_lama: Some("U2FsdGVkX19xJlSDxwXghX/OIKvlFrU/zymK5IWts1+9zVuiarSWbf1nu4c0n8PSYCmVNpg7w4gW9BfiekIndg==".to_string()),
                    id_matkul: None,
                    ..default_args()
                };
                assert!(args.subject().is_ok());
            }
        }

        mod weeks {
            use super::*;

            #[test]
            fn accepts_single_valid_week() {
                let args = Args {
                    minggu: "5".to_string(),
                    ..default_args()
                };
                assert!(args.weeks().is_ok());
            }

            #[test]
            fn accepts_multiple_valid_weeks() {
                let args = Args {
                    minggu: "1,2,3,14".to_string(),
                    ..default_args()
                };
                let weeks = args.weeks().unwrap();
                assert_eq!(weeks.len(), 4);
            }

            #[test]
            fn accepts_boundary_week_1() {
                let args = Args {
                    minggu: "1".to_string(),
                    ..default_args()
                };
                assert!(args.weeks().is_ok());
            }

            #[test]
            fn accepts_boundary_week_14() {
                let args = Args {
                    minggu: "14".to_string(),
                    ..default_args()
                };
                assert!(args.weeks().is_ok());
            }

            #[test]
            fn rejects_week_zero() {
                let args = Args {
                    minggu: "0".to_string(),
                    ..default_args()
                };
                assert!(args.weeks().is_err());
            }

            #[test]
            fn rejects_week_above_14() {
                let args = Args {
                    minggu: "15".to_string(),
                    ..default_args()
                };
                assert!(args.weeks().is_err());
            }

            #[test]
            fn rejects_non_numeric_week() {
                let args = Args {
                    minggu: "abc".to_string(),
                    ..default_args()
                };
                assert!(args.weeks().is_err());
            }

            #[test]
            fn rejects_if_any_week_in_list_is_invalid() {
                let args = Args {
                    minggu: "1,2,15".to_string(),
                    ..default_args()
                };
                assert!(args.weeks().is_err());
            }

            #[test]
            fn trims_whitespace_around_weeks() {
                let args = Args {
                    minggu: "1, 2, 3".to_string(),
                    ..default_args()
                };
                assert!(args.weeks().is_ok());
            }
        }

        mod account {
            use super::*;

            #[test]
            fn accepts_valid_numeric_nim() {
                let args = Args {
                    nim: "20190001".to_string(),
                    ..default_args()
                };
                assert!(args.account().is_ok());
            }

            #[test]
            fn rejects_nim_with_letters() {
                let args = Args {
                    nim: "2019ABC1".to_string(),
                    ..default_args()
                };
                assert!(args.account().is_err());
            }

            #[test]
            fn rejects_nim_with_symbols() {
                let args = Args {
                    nim: "2019-001".to_string(),
                    ..default_args()
                };
                assert!(args.account().is_err());
            }
        }
    }
}

#[cfg(test)]
fn default_args() -> Args {
    Args {
        presensi_lama: None,
        id_matkul: Some("120184".to_string()),
        minggu: "1".to_string(),
        nim: "123456".to_string(),
        password: "pass".to_string(),
    }
}
