use data_encoding::HEXLOWER;
use hyperintel_core::Sha256Digest;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

/// Fixed schema identifier for PQAI hash-root assurance anchors.
pub const PQAI_ANCHOR_SCHEMA_V1: &str = "hyperintel.assurance.pqai-hash-root-anchor.v1";
/// Domain separator for deterministic PQAI anchor commitment digests.
pub const PQAI_ANCHOR_DOMAIN_V1: &[u8] = b"hyperintel:pqai-anchor:v1\0";

/// Cryptographic root posture declared for an offline assurance anchor.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PqaiHashRootPostureV1 {
    /// Classical-only root (Ed25519 / ECDSA / BLS) — vulnerable to quantum attacks.
    ClassicalPreQuantum,
    /// Lattice-backed PQC root without a pure hash-based root anchor.
    LatticeOnlyPqc,
    /// Pure hash-based root (`FIPS 205 SLH-DSA-SHA2-128f` / `XMSS-WOTS+-SHA256` / `EIP-8288`).
    HashOnlyPqai,
}

/// Hash primitive family backing the external root commitment.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PqaiHashFamilyV1 {
    /// Conservative cryptographic hash (SHA-256 / BLAKE3).
    ConservativeSha256Blake3,
    /// Algebraic SNARK-friendly hash (Poseidon / Poseidon2).
    AlgebraicPoseidon2,
}

/// Fail-closed policy controlling acceptance of PQAI assurance anchors.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PqaiAnchorPolicyV1 {
    /// Require a pure hash-based PQAI root (`HashOnlyPqai`).
    pub require_hash_only_root: bool,
    /// Reject algebraic hash primitives (`Poseidon`/`Poseidon2`) at the trust root.
    pub forbid_algebraic_hash_root: bool,
}

/// Offline anchor binding a `HyperIntel` assurance envelope to a hash-based PQAI root.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PqaiAssuranceAnchorV1 {
    /// Fixed schema identifier.
    pub schema_version: String,
    /// Digest of the bound `HyperIntel` `AssuranceEnvelopeV1`.
    pub assurance_envelope_digest: Sha256Digest,
    /// Digest of the bound subject (e.g. `SignedStatementV1` or evidence set).
    pub subject_digest: Sha256Digest,
    /// Cryptographic root class.
    pub root_posture: PqaiHashRootPostureV1,
    /// Hash primitive family used by the root.
    pub hash_family: PqaiHashFamilyV1,
    /// Canonical algorithm identifier (e.g. `slh-dsa-sha2-128f`, `xmss-wots-sha256`).
    pub root_algorithm: String,
    /// Lowercase hex SHA-256 fingerprint of the external PQAI root public key.
    pub root_pubkey_sha256_hex: String,
    /// Lowercase hex SHA-256 commitment over all fields above.
    pub anchor_commitment_hex: String,
}

/// Verified offline PQAI anchor result.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerifiedPqaiAnchorV1 {
    /// Bound assurance envelope digest.
    pub assurance_envelope_digest: Sha256Digest,
    /// Bound subject digest.
    pub subject_digest: Sha256Digest,
    /// Verified cryptographic posture.
    pub root_posture: PqaiHashRootPostureV1,
    /// Verified deterministic anchor commitment.
    pub anchor_commitment_hex: String,
}

/// Closed verification errors for PQAI hash-root assurance anchors.
#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum PqaiAnchorError {
    /// Unexpected schema version.
    #[error("unsupported pqai anchor schema version")]
    UnsupportedSchema,
    /// Root algorithm label is empty or not allowed for declared posture.
    #[error("invalid pqai root algorithm for declared posture")]
    InvalidRootAlgorithm,
    /// Public key fingerprint or commitment digest is not 64 lowercase hex characters.
    #[error("malformed 32-byte lowercase hex digest")]
    MalformedDigestHex,
    /// Policy requires pure hash-based PQAI root.
    #[error("anchor posture does not satisfy require_hash_only_root policy")]
    InsufficientPosture,
    /// Policy forbids algebraic hashes at the trust root.
    #[error("algebraic hash primitive forbidden at trust root")]
    AlgebraicHashForbidden,
    /// Commitment digest does not match deterministic recomputation.
    #[error("pqai anchor commitment mismatch")]
    CommitmentMismatch,
}

fn is_lower_hex_32(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

fn is_valid_hash_only_alg(alg: &str) -> bool {
    matches!(
        alg,
        "slh-dsa-sha2-128f" | "slh-dsa-sha2-128s" | "xmss-wots-sha256" | "eip-8288-hash-agg"
    )
}

/// Deterministically compute the SHA-256 commitment for a PQAI assurance anchor.
#[must_use]
pub fn compute_pqai_anchor_commitment_hex(anchor: &PqaiAssuranceAnchorV1) -> String {
    let mut hasher = Sha256::new();
    hasher.update(PQAI_ANCHOR_DOMAIN_V1);
    hasher.update(anchor.schema_version.as_bytes());
    hasher.update(b"\0");
    hasher.update(anchor.assurance_envelope_digest.as_bytes());
    hasher.update(anchor.subject_digest.as_bytes());
    hasher.update(format!("{:?}|{:?}", anchor.root_posture, anchor.hash_family).as_bytes());
    hasher.update(b"\0");
    hasher.update(anchor.root_algorithm.as_bytes());
    hasher.update(b"\0");
    hasher.update(anchor.root_pubkey_sha256_hex.as_bytes());
    HEXLOWER.encode(&hasher.finalize())
}

/// Verify an offline PQAI assurance anchor against a fail-closed policy.
///
/// # Errors
///
/// Returns a closed [`PqaiAnchorError`] on schema mismatch, invalid algorithm,
/// malformed hex digest, policy violation, or commitment mismatch.
pub fn verify_pqai_assurance_anchor_v1(
    anchor: &PqaiAssuranceAnchorV1,
    policy: PqaiAnchorPolicyV1,
) -> Result<VerifiedPqaiAnchorV1, PqaiAnchorError> {
    if anchor.schema_version != PQAI_ANCHOR_SCHEMA_V1 {
        return Err(PqaiAnchorError::UnsupportedSchema);
    }
    if anchor.root_algorithm.is_empty() {
        return Err(PqaiAnchorError::InvalidRootAlgorithm);
    }
    if !is_lower_hex_32(&anchor.root_pubkey_sha256_hex)
        || !is_lower_hex_32(&anchor.anchor_commitment_hex)
    {
        return Err(PqaiAnchorError::MalformedDigestHex);
    }
    if anchor.root_posture == PqaiHashRootPostureV1::HashOnlyPqai
        && !is_valid_hash_only_alg(&anchor.root_algorithm)
    {
        return Err(PqaiAnchorError::InvalidRootAlgorithm);
    }
    if policy.require_hash_only_root && anchor.root_posture != PqaiHashRootPostureV1::HashOnlyPqai {
        return Err(PqaiAnchorError::InsufficientPosture);
    }
    if policy.forbid_algebraic_hash_root
        && anchor.hash_family == PqaiHashFamilyV1::AlgebraicPoseidon2
    {
        return Err(PqaiAnchorError::AlgebraicHashForbidden);
    }
    let expected = compute_pqai_anchor_commitment_hex(anchor);
    if expected != anchor.anchor_commitment_hex {
        return Err(PqaiAnchorError::CommitmentMismatch);
    }

    Ok(VerifiedPqaiAnchorV1 {
        assurance_envelope_digest: anchor.assurance_envelope_digest,
        subject_digest: anchor.subject_digest,
        root_posture: anchor.root_posture,
        anchor_commitment_hex: expected,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_anchor() -> PqaiAssuranceAnchorV1 {
        let mut anchor = PqaiAssuranceAnchorV1 {
            schema_version: PQAI_ANCHOR_SCHEMA_V1.to_string(),
            assurance_envelope_digest: Sha256Digest::from_bytes([0x11; 32]),
            subject_digest: Sha256Digest::from_bytes([0x22; 32]),
            root_posture: PqaiHashRootPostureV1::HashOnlyPqai,
            hash_family: PqaiHashFamilyV1::ConservativeSha256Blake3,
            root_algorithm: "slh-dsa-sha2-128f".to_string(),
            root_pubkey_sha256_hex: "b".repeat(64),
            anchor_commitment_hex: String::new(),
        };
        anchor.anchor_commitment_hex = compute_pqai_anchor_commitment_hex(&anchor);
        anchor
    }

    fn strict_policy() -> PqaiAnchorPolicyV1 {
        PqaiAnchorPolicyV1 {
            require_hash_only_root: true,
            forbid_algebraic_hash_root: true,
        }
    }

    #[test]
    fn valid_slh_dsa_anchor_verifies() {
        let anchor = valid_anchor();
        let expected = VerifiedPqaiAnchorV1 {
            assurance_envelope_digest: anchor.assurance_envelope_digest,
            subject_digest: anchor.subject_digest,
            root_posture: PqaiHashRootPostureV1::HashOnlyPqai,
            anchor_commitment_hex: anchor.anchor_commitment_hex.clone(),
        };
        assert_eq!(
            verify_pqai_assurance_anchor_v1(&anchor, strict_policy()),
            Ok(expected)
        );
    }

    #[test]
    fn rejects_lattice_only_and_classical_under_strict_policy() {
        let mut anchor = valid_anchor();
        anchor.root_posture = PqaiHashRootPostureV1::LatticeOnlyPqc;
        anchor.root_algorithm = "ml-dsa-65".to_string();
        anchor.anchor_commitment_hex = compute_pqai_anchor_commitment_hex(&anchor);
        assert_eq!(
            verify_pqai_assurance_anchor_v1(&anchor, strict_policy()),
            Err(PqaiAnchorError::InsufficientPosture)
        );
    }

    #[test]
    fn rejects_algebraic_hash_and_tampered_commitment() {
        let mut anchor = valid_anchor();
        anchor.hash_family = PqaiHashFamilyV1::AlgebraicPoseidon2;
        anchor.anchor_commitment_hex = compute_pqai_anchor_commitment_hex(&anchor);
        assert_eq!(
            verify_pqai_assurance_anchor_v1(&anchor, strict_policy()),
            Err(PqaiAnchorError::AlgebraicHashForbidden)
        );

        let mut tampered = valid_anchor();
        tampered.subject_digest = Sha256Digest::from_bytes([0x33; 32]);
        assert_eq!(
            verify_pqai_assurance_anchor_v1(&tampered, strict_policy()),
            Err(PqaiAnchorError::CommitmentMismatch)
        );
    }

    #[test]
    fn rejects_spoofed_algorithm_claiming_hash_only() {
        let mut anchor = valid_anchor();
        anchor.root_algorithm = "secp256k1".to_string();
        anchor.anchor_commitment_hex = compute_pqai_anchor_commitment_hex(&anchor);
        assert_eq!(
            verify_pqai_assurance_anchor_v1(&anchor, strict_policy()),
            Err(PqaiAnchorError::InvalidRootAlgorithm)
        );
    }
}
