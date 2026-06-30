use std::{fs::File, io::Read, path::Path};

use base64::{Engine as _, engine::general_purpose::STANDARD};
use ed25519_dalek::{Signature, Signer, Verifier, VerifyingKey};
use sha2::{Digest, Sha256};

use crate::UpdateError;

/// Verifies a detached Ed25519 signature for update manifest bytes.
///
/// # Errors
///
/// Returns an error if no public key is configured, the signature is malformed,
/// a configured public key is malformed, or no configured key verifies the
/// signature.
pub fn verify_manifest_signature(
    manifest_bytes: &[u8],
    signature_base64: &str,
    public_keys_base64: &[String],
) -> Result<(), UpdateError> {
    if public_keys_base64.is_empty() {
        return Err(UpdateError::Disabled(
            "no update public key is configured".to_string(),
        ));
    }

    let signature = decode_signature(signature_base64)?;
    let accepted = public_keys_base64.iter().any(|public_key| {
        decode_public_key(public_key)
            .and_then(|key| key.verify(manifest_bytes, &signature).map_err(|_| ()))
            .is_ok()
    });

    if accepted {
        Ok(())
    } else {
        Err(UpdateError::SignatureVerificationFailed)
    }
}

/// Signs update manifest bytes with a base64-encoded Ed25519 seed key.
///
/// # Errors
///
/// Returns an error if the signing key is not valid base64 or does not decode
/// to a 32-byte Ed25519 seed.
pub fn sign_manifest_bytes(
    manifest_bytes: &[u8],
    signing_key_base64: &str,
) -> Result<String, UpdateError> {
    let key_bytes = STANDARD
        .decode(signing_key_base64.trim())
        .map_err(|error| UpdateError::InvalidManifest(format!("invalid signing key: {error}")))?;
    let key_bytes: [u8; 32] = key_bytes.as_slice().try_into().map_err(|_| {
        UpdateError::InvalidManifest("signing key must be 32 raw Ed25519 seed bytes".to_string())
    })?;
    let signing_key = ed25519_dalek::SigningKey::from_bytes(&key_bytes);
    let signature = signing_key.sign(manifest_bytes);

    Ok(STANDARD.encode(signature.to_bytes()))
}

/// Computes a lowercase hex SHA-256 digest for a file.
///
/// # Errors
///
/// Returns an error if the file cannot be opened or read.
pub fn file_sha256_hex(path: &Path) -> Result<String, UpdateError> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0_u8; 64 * 1024].into_boxed_slice();

    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }

    Ok(hex::encode(hasher.finalize()))
}

fn decode_public_key(value: &str) -> Result<VerifyingKey, ()> {
    let bytes = STANDARD.decode(value.trim()).map_err(|_| ())?;
    let bytes: [u8; 32] = bytes.as_slice().try_into().map_err(|_| ())?;
    VerifyingKey::from_bytes(&bytes).map_err(|_| ())
}

fn decode_signature(value: &str) -> Result<Signature, UpdateError> {
    let bytes = STANDARD
        .decode(value.trim())
        .map_err(|_| UpdateError::SignatureVerificationFailed)?;
    let bytes: [u8; 64] = bytes
        .as_slice()
        .try_into()
        .map_err(|_| UpdateError::SignatureVerificationFailed)?;
    Ok(Signature::from_bytes(&bytes))
}

#[cfg(test)]
mod tests {
    use base64::{Engine as _, engine::general_purpose::STANDARD};
    use ed25519_dalek::{Signer, SigningKey};
    use rand_core::OsRng;

    use super::*;

    #[test]
    fn verify_manifest_signature_accepts_matching_key() {
        let signing_key = SigningKey::generate(&mut OsRng);
        let manifest = br#"{"schemaVersion":1}"#;
        let signature = signing_key.sign(manifest);
        let public_key = STANDARD.encode(signing_key.verifying_key().to_bytes());

        let result = verify_manifest_signature(
            manifest,
            &STANDARD.encode(signature.to_bytes()),
            &[public_key],
        );

        assert!(result.is_ok(), "signature should verify: {result:?}");
    }

    #[test]
    fn verify_manifest_signature_rejects_tampered_bytes() {
        let signing_key = SigningKey::generate(&mut OsRng);
        let signature = signing_key.sign(br#"{"schemaVersion":1}"#);
        let public_key = STANDARD.encode(signing_key.verifying_key().to_bytes());

        let result = verify_manifest_signature(
            br#"{"schemaVersion":2}"#,
            &STANDARD.encode(signature.to_bytes()),
            &[public_key],
        );

        assert!(matches!(
            result,
            Err(UpdateError::SignatureVerificationFailed)
        ));
    }
}
