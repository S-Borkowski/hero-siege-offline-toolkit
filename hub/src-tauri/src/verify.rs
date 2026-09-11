//! Integrity: SHA-256 over artifacts, minisign over the catalog.
//!
//! None of the ten tools is code-signed, so SmartScreen will warn about all of
//! them and Authenticode has nothing to say. The catalog is what stands in for
//! that: a hash pinned at build time inside a file signed by a key the hub
//! embeds. Swapping a release asset after the fact fails here rather than
//! installing.

use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;

use sha2::{Digest, Sha256};

/// The key `catalog.json.minisig` is verified against, compiled in.
///
/// Embedded rather than fetched, on purpose: a trust root you download is not a
/// trust root. Rotating it is a hub release.
pub const CATALOG_PUBLIC_KEY: &str = include_str!("../../../catalog/catalog-signing.pub");

#[derive(Debug)]
pub enum VerifyError {
    Io(std::io::Error),
    /// The bytes are not what the catalog said they would be. Carries both
    /// values so the log says which, rather than just "verification failed".
    HashMismatch { expected: String, actual: String },
    Signature(String),
}

impl std::fmt::Display for VerifyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VerifyError::Io(e) => write!(f, "{e}"),
            VerifyError::HashMismatch { expected, actual } => write!(
                f,
                "SHA-256 mismatch: the catalog pins {expected} but the download hashes to {actual}"
            ),
            VerifyError::Signature(e) => write!(f, "catalog signature: {e}"),
        }
    }
}

impl std::error::Error for VerifyError {}

impl From<std::io::Error> for VerifyError {
    fn from(value: std::io::Error) -> Self {
        VerifyError::Io(value)
    }
}

pub fn sha256_bytes(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hex(&hasher.finalize())
}

/// Hash a file without reading it all into memory -- the largest artifact in the
/// catalog is 50 MB today, but nothing here should assume that stays true.
pub fn sha256_file(path: &Path) -> Result<String, VerifyError> {
    let mut reader = BufReader::new(File::open(path)?);
    let mut hasher = Sha256::new();
    let mut buffer = vec![0u8; 64 * 1024];
    loop {
        let read = reader.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hex(&hasher.finalize()))
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Compare case-insensitively; catalogs write lowercase but several of the
/// upstream `SHA256SUMS` files are uppercase and a hand-edited catalog might be.
pub fn expect_sha256(actual: &str, expected: &str) -> Result<(), VerifyError> {
    if actual.eq_ignore_ascii_case(expected) {
        Ok(())
    } else {
        Err(VerifyError::HashMismatch {
            expected: expected.to_ascii_lowercase(),
            actual: actual.to_ascii_lowercase(),
        })
    }
}

/// The base64 payload out of a minisign `.pub` file.
fn public_key_payload(text: &str) -> Result<&str, VerifyError> {
    text.lines()
        .map(str::trim)
        .find(|line| !line.is_empty() && !line.starts_with("untrusted comment:"))
        .ok_or_else(|| VerifyError::Signature("the public key file has no payload line".into()))
}

/// Check a detached minisign signature over `payload`.
///
/// `minisign_verify::PublicKey::verify` checks the global signature over
/// `signature || trusted comment` as well as the one over the content, so the
/// trusted comment this returns has actually been signed.
pub fn verify_catalog(payload: &[u8], signature_text: &str) -> Result<String, VerifyError> {
    let key_text = public_key_payload(CATALOG_PUBLIC_KEY)?;
    let public_key = minisign_verify::PublicKey::from_base64(key_text)
        .map_err(|e| VerifyError::Signature(format!("embedded public key is unusable: {e}")))?;
    let signature = minisign_verify::Signature::decode(signature_text)
        .map_err(|e| VerifyError::Signature(format!("malformed signature file: {e}")))?;
    let trusted = signature.trusted_comment().to_string();
    public_key
        .verify(payload, &signature, false)
        .map_err(|e| VerifyError::Signature(e.to_string()))?;
    Ok(trusted)
}

#[cfg(test)]
mod tests {
    use super::*;

    const EMPTY_SHA256: &str =
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

    #[test]
    fn hashes_match_the_known_vector() {
        assert_eq!(sha256_bytes(b""), EMPTY_SHA256);
        assert_eq!(
            sha256_bytes(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn file_hashing_agrees_with_byte_hashing_across_the_buffer_boundary() {
        let dir = std::env::temp_dir().join(format!("hub-verify-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("big.bin");
        // Larger than the 64 KiB read buffer, so a bug in the loop shows up.
        let data: Vec<u8> = (0..200_000u32).map(|i| (i % 251) as u8).collect();
        std::fs::write(&path, &data).unwrap();
        assert_eq!(sha256_file(&path).unwrap(), sha256_bytes(&data));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn a_mismatch_names_both_values() {
        let err = expect_sha256("aa", "bb").unwrap_err();
        let text = err.to_string();
        assert!(text.contains("aa") && text.contains("bb"), "{text}");
    }

    #[test]
    fn case_does_not_decide_whether_an_artifact_is_trusted() {
        assert!(expect_sha256(EMPTY_SHA256, &EMPTY_SHA256.to_uppercase()).is_ok());
    }

    #[test]
    fn the_embedded_public_key_parses() {
        // If this fails the hub cannot verify anything, and it should fail here
        // rather than the first time a user presses Check for updates.
        let payload = public_key_payload(CATALOG_PUBLIC_KEY).unwrap();
        assert!(minisign_verify::PublicKey::from_base64(payload).is_ok());
    }

    #[test]
    fn the_checked_in_catalog_verifies_against_the_checked_in_key() {
        let payload = include_bytes!("../../../catalog/catalog.json");
        let signature = include_str!("../../../catalog/catalog.json.minisig");
        let trusted = verify_catalog(payload, signature).expect("catalog signature");
        assert!(trusted.starts_with("catalog "), "trusted comment: {trusted}");
    }

    #[test]
    fn a_catalog_with_one_byte_changed_does_not_verify() {
        let mut payload = include_bytes!("../../../catalog/catalog.json").to_vec();
        let signature = include_str!("../../../catalog/catalog.json.minisig");
        let last = payload.len() - 1;
        payload[last] ^= 0x01;
        assert!(verify_catalog(&payload, signature).is_err());
    }
}
