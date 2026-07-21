//! Offline verifier for a portable `HyperIntel` market evidence bundle.

use std::{collections::BTreeMap, env, fs, path::PathBuf, process::ExitCode};

use hyperintel_assurance::{RegisteredVerificationKeyV1, VerificationKeyRegistry};
use hyperintel_core::Sha256Digest;
use hyperintel_market::{MarketEvidenceBundleV1, verify_market_evidence_bundle_v1};
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct VerificationInputV1 {
    bundle: MarketEvidenceBundleV1,
    assurance_blobs: BTreeMap<Sha256Digest, Vec<u8>>,
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
    let input: VerificationInputV1 = serde_json::from_slice(&input_bytes)?;
    let trust_bytes = fs::read(trust_path)?;
    let trust: TrustInputV1 = serde_json::from_slice(&trust_bytes)?;
    let mut trusted_registry = VerificationKeyRegistry::default();
    for key in trust.trusted_verification_keys {
        trusted_registry.register(key)?;
    }
    verify_market_evidence_bundle_v1(&input.bundle, &input.assurance_blobs, &trusted_registry)?;
    println!(
        "verified market evidence bundle {}",
        input.bundle.bundle_digest
    );
    Ok(())
}
