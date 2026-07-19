use std::{env, fs, fs::OpenOptions, io::Write as _, path::PathBuf, process::ExitCode};

use base64::{Engine as _, engine::general_purpose::STANDARD};
use ed25519_dalek::{Signer as _, SigningKey};
use zeroize::Zeroizing;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("manifest signing failed: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<()> {
    let (manifest_path, output_path) = parse_args()?;
    let manifest = fs::read(manifest_path)?;
    let signing_key = Zeroizing::new(
        env::var("FRAME_UPDATE_SIGNING_KEY")
            .map_err(|_| "FRAME_UPDATE_SIGNING_KEY must contain a base64-encoded Ed25519 seed")?,
    );
    let expected_public_keys = env::var("FRAME_UPDATE_PUBLIC_KEY")
        .map_err(|_| "FRAME_UPDATE_PUBLIC_KEY must contain the embedded public key")?;
    let signature = sign(&manifest, &signing_key, &expected_public_keys)?;

    let mut output = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output_path)?;
    output.write_all(signature.as_bytes())?;
    output.write_all(b"\n")?;
    output.sync_all()?;
    Ok(())
}

fn parse_args() -> Result<(PathBuf, PathBuf)> {
    let mut args = env::args().skip(1);
    let mut manifest = None;
    let mut output = None;

    while let Some(argument) = args.next() {
        match argument.as_str() {
            "--manifest" => manifest = args.next().map(PathBuf::from),
            "--out" => output = args.next().map(PathBuf::from),
            _ => return Err(format!("unknown argument `{argument}`").into()),
        }
    }

    let manifest = manifest.ok_or("missing --manifest <path>")?;
    let output = output.ok_or("missing --out <path>")?;
    Ok((manifest, output))
}

fn sign(manifest: &[u8], signing_key_base64: &str, expected_public_keys: &str) -> Result<String> {
    let key_bytes = Zeroizing::new(STANDARD.decode(signing_key_base64.trim())?);
    let seed = Zeroizing::new(
        key_bytes
            .as_slice()
            .try_into()
            .map_err(|_| "signing key must be exactly 32 raw Ed25519 seed bytes")?,
    );
    let signing_key = SigningKey::from_bytes(&seed);
    let public_key = STANDARD.encode(signing_key.verifying_key().to_bytes());
    if !expected_public_keys
        .split(',')
        .map(str::trim)
        .any(|candidate| candidate == public_key)
    {
        return Err("signing key does not match any configured update public key".into());
    }
    Ok(STANDARD.encode(signing_key.sign(manifest).to_bytes()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signature, Verifier as _};

    #[test]
    fn detached_signature_verifies_for_the_matching_manifest() {
        let seed = [7_u8; 32];
        let encoded_seed = STANDARD.encode(seed);
        let encoded_public_key =
            STANDARD.encode(SigningKey::from_bytes(&seed).verifying_key().to_bytes());
        let manifest = br#"{"schemaVersion":1}"#;

        let encoded_signature = sign(manifest, &encoded_seed, &encoded_public_key).unwrap();
        let signature_bytes: [u8; 64] = STANDARD
            .decode(encoded_signature)
            .unwrap()
            .try_into()
            .unwrap();
        let signature = Signature::from_bytes(&signature_bytes);
        let verifying_key = SigningKey::from_bytes(&seed).verifying_key();

        assert!(verifying_key.verify(manifest, &signature).is_ok());
        assert!(
            verifying_key
                .verify(br#"{"schemaVersion":2}"#, &signature)
                .is_err()
        );
    }

    #[test]
    fn signing_rejects_a_key_that_is_not_embedded_in_the_application() {
        let error = sign(b"manifest", &STANDARD.encode([7_u8; 32]), "different-key")
            .unwrap_err()
            .to_string();

        assert!(error.contains("does not match"));
    }
}
