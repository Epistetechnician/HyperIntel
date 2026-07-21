//! Offline verifier for a portable `HyperIntel` market evidence bundle.

use std::{collections::BTreeMap, env, fs, path::PathBuf, process::ExitCode};

use hyperintel_assurance::{RegisteredVerificationKeyV1, VerificationKeyRegistry};
use hyperintel_core::Sha256Digest;
use hyperintel_market::{
    MarketEvidenceBundleV1, SignedMarketEvidenceBundleV2, verify_market_evidence_bundle_v1,
    verify_signed_market_evidence_bundle_v2,
};
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct VerificationInputV1 {
    bundle: MarketEvidenceBundleV1,
    assurance_blobs: BTreeMap<Sha256Digest, Vec<u8>>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct VerificationInputV2 {
    bundle: SignedMarketEvidenceBundleV2,
    assurance_blobs: BTreeMap<Sha256Digest, Vec<u8>>,
}

enum VerificationInput {
    SignedV2(VerificationInputV2),
    LegacyV1(VerificationInputV1),
}

#[derive(Deserialize)]
struct VerificationInputProbe {
    bundle: BundleSchemaProbe,
}

#[derive(Deserialize)]
struct BundleSchemaProbe {
    schema_version: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TrustInputV1 {
    trusted_verification_keys: Vec<RegisteredVerificationKeyV1>,
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("hyperintel verification failed: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let mut arguments = env::args_os();
    let program = arguments.next().unwrap_or_else(|| "hyperintel-cli".into());
    let evidence_path = arguments.next().map(PathBuf::from).ok_or_else(|| {
        format!(
            "usage: {} <evidence-input.json> <trusted-keys.json>",
            PathBuf::from(program).display()
        )
    })?;
    let trust_path = arguments
        .next()
        .map(PathBuf::from)
        .ok_or("missing caller-controlled trusted-key file")?;
    if arguments.next().is_some() {
        return Err("expected exactly two input paths".into());
    }

    let input_bytes = fs::read(evidence_path)?;
    let input = decode_verification_input(&input_bytes)?;
    let trust_bytes = fs::read(trust_path)?;
    let trust: TrustInputV1 = serde_json::from_slice(&trust_bytes)?;
    let mut trusted_registry = VerificationKeyRegistry::default();
    for key in trust.trusted_verification_keys {
        trusted_registry.register(key)?;
    }
    let bundle_digest = match input {
        VerificationInput::SignedV2(input) => {
            verify_signed_market_evidence_bundle_v2(
                &input.bundle,
                &input.assurance_blobs,
                &trusted_registry,
            )?;
            input.bundle.bundle_digest
        }
        VerificationInput::LegacyV1(input) => {
            verify_market_evidence_bundle_v1(
                &input.bundle,
                &input.assurance_blobs,
                &trusted_registry,
            )?;
            input.bundle.bundle_digest
        }
    };
    println!("verified market evidence bundle {bundle_digest}");
    Ok(())
}

fn decode_verification_input(
    input_bytes: &[u8],
) -> Result<VerificationInput, Box<dyn std::error::Error>> {
    let probe: VerificationInputProbe = serde_json::from_slice(input_bytes)?;
    match probe.bundle.schema_version.as_str() {
        "hyperintel.market-evidence-bundle.v2" => Ok(VerificationInput::SignedV2(
            serde_json::from_slice(input_bytes)?,
        )),
        "hyperintel.market-evidence-bundle.v1" => Ok(VerificationInput::LegacyV1(
            serde_json::from_slice(input_bytes)?,
        )),
        _ => Err("unsupported market evidence bundle schema".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contract_cli_dispatches_only_explicit_v1_or_v2_bundle_schemas() {
        let unknown = br#"{"bundle":{"schema_version":"hyperintel.market-evidence-bundle.v3"}}"#;
        assert!(
            decode_verification_input(unknown).is_err_and(
                |error| error.to_string() == "unsupported market evidence bundle schema"
            )
        );

        let incomplete_v1 =
            br#"{"bundle":{"schema_version":"hyperintel.market-evidence-bundle.v1"}}"#;
        let incomplete_v2 =
            br#"{"bundle":{"schema_version":"hyperintel.market-evidence-bundle.v2"}}"#;
        assert!(
            decode_verification_input(incomplete_v1).is_err_and(
                |error| error.to_string() != "unsupported market evidence bundle schema"
            )
        );
        assert!(
            decode_verification_input(incomplete_v2).is_err_and(
                |error| error.to_string() != "unsupported market evidence bundle schema"
            )
        );
    }
}
