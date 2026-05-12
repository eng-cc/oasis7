//! Tests for the runtime module.

use ed25519_dalek::{Signer, SigningKey};

use crate::runtime::{util, ModuleArtifactIdentity};

pub(super) fn pos(x: i64, y: i64) -> crate::geometry::GeoPos {
    crate::geometry::GeoPos {
        x_cm: x,
        y_cm: y,
        z_cm: 0,
    }
}

const TEST_MODULE_ARTIFACT_SIGNER_NODE_ID: &str = "test.module.release.signer";

pub(super) fn signed_test_artifact_identity(wasm_hash: &str) -> ModuleArtifactIdentity {
    let source_hash = util::sha256_hex(format!("test-src:{wasm_hash}").as_bytes());
    let build_manifest_hash = util::sha256_hex(b"test-build-manifest-v1");
    let payload = ModuleArtifactIdentity::signing_payload_v1(
        wasm_hash,
        source_hash.as_str(),
        build_manifest_hash.as_str(),
        TEST_MODULE_ARTIFACT_SIGNER_NODE_ID,
    );
    let signing_key = test_module_artifact_signing_key();
    let signature = signing_key.sign(payload.as_slice());
    ModuleArtifactIdentity {
        source_hash,
        build_manifest_hash,
        signer_node_id: TEST_MODULE_ARTIFACT_SIGNER_NODE_ID.to_string(),
        signature_scheme: ModuleArtifactIdentity::SIGNATURE_SCHEME_ED25519.to_string(),
        artifact_signature: format!(
            "{}{}",
            ModuleArtifactIdentity::SIGNATURE_PREFIX_ED25519_V1,
            hex::encode(signature.to_bytes())
        ),
    }
}

fn test_module_artifact_signing_key() -> SigningKey {
    // Deterministic testing key; production verification only trusts its public key.
    let seed = util::sha256_hex(b"oasis7-test-module-artifact-signer-v1");
    let seed_bytes = hex::decode(seed).expect("decode test module signing seed");
    let private_key_bytes: [u8; 32] = seed_bytes
        .as_slice()
        .try_into()
        .expect("test module signing seed is 32 bytes");
    SigningKey::from_bytes(&private_key_bytes)
}

mod agent_claims;
mod agent_claims_auto_funding;
#[cfg(feature = "test_tier_full")]
mod agent_default_modules;
mod apply_domain_event_guards;
mod audit;
mod basic;
mod body;
mod builtin_wasm_identity;
mod builtin_wasm_materializer;
mod data_access_control;
mod economy;
mod economy_bootstrap;
mod economy_factory_lifecycle;
mod economy_module_requests;
mod economy_priority_logistics;
mod effects;
mod gameplay;
mod gameplay_bootstrap;
mod gameplay_protocol;
mod governance;
mod governance_validator_admission;
mod main_token;
mod main_token_economy_audit;
mod module_action_loop;
mod module_runtime_metering;
mod modules;
mod operability_release_gate;
mod persistence;
#[cfg(feature = "test_tier_full")]
mod power_bootstrap;
mod power_bootstrap_release_manifest_full;
mod reward_asset;
mod reward_asset_settlement_action;
mod rules;
mod storage_cold_index;
mod storage_footprint_fixture;
