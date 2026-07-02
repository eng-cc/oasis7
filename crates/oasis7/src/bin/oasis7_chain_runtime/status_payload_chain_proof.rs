use std::fs;
use std::path::Path;

use serde::Serialize;

#[derive(Debug, Serialize)]
pub(crate) struct ChainProofStatus {
    pub(crate) schema_version: String,
    pub(crate) proof_contract: String,
    pub(crate) claim_boundary: String,
    pub(crate) status: String,
    pub(crate) latest_world_head_proof: Option<LatestWorldHeadProofStatus>,
    pub(crate) source_record_path: Option<String>,
    pub(crate) load_error: Option<String>,
    pub(crate) does_not_claim: Vec<String>,
}

#[derive(Debug, Serialize)]
pub(crate) struct LatestWorldHeadProofStatus {
    pub(crate) schema_version: u16,
    pub(crate) world_id: String,
    pub(crate) height: u64,
    pub(crate) execution_block_hash: String,
    pub(crate) execution_state_root: String,
    pub(crate) node_block_hash: String,
    pub(crate) action_root: String,
    pub(crate) world_head_proof_ref: String,
    pub(crate) proof_hash: String,
    pub(crate) checkpoint_ref: Option<String>,
}

pub(crate) fn build_chain_proof_status(execution_records_dir: Option<&Path>) -> ChainProofStatus {
    let schema_version = "oasis7.chain_proof_status.v1".to_string();
    let proof_contract = "WorldHeadProofV1".to_string();
    let claim_boundary =
        "head_execution_checkpoint_evidence_only_not_light_client_or_mainnet_readiness".to_string();
    let does_not_claim = vec![
        "public_testnet ready".to_string(),
        "ready_for_live_candidate".to_string(),
        "mainnet-grade".to_string(),
        "light-client verified".to_string(),
        "state proof verified".to_string(),
        "receipt proof verified".to_string(),
        "DA sampling verified".to_string(),
    ];
    let Some(execution_records_dir) = execution_records_dir else {
        return ChainProofStatus {
            schema_version,
            proof_contract,
            claim_boundary,
            status: "unavailable".to_string(),
            latest_world_head_proof: None,
            source_record_path: None,
            load_error: Some("execution_records_dir_unconfigured".to_string()),
            does_not_claim,
        };
    };
    let latest_path = execution_records_dir.join("latest.json");
    if !latest_path.exists() {
        return ChainProofStatus {
            schema_version,
            proof_contract,
            claim_boundary,
            status: "unavailable".to_string(),
            latest_world_head_proof: None,
            source_record_path: Some(latest_path.display().to_string()),
            load_error: Some("execution_bridge_latest_record_missing".to_string()),
            does_not_claim,
        };
    }
    let bytes = match fs::read(latest_path.as_path()) {
        Ok(bytes) => bytes,
        Err(err) => {
            return ChainProofStatus {
                schema_version,
                proof_contract,
                claim_boundary,
                status: "stale_or_invalid".to_string(),
                latest_world_head_proof: None,
                source_record_path: Some(latest_path.display().to_string()),
                load_error: Some(format!("read latest execution record failed: {err}")),
                does_not_claim,
            };
        }
    };
    let latest: serde_json::Value = match serde_json::from_slice(bytes.as_slice()) {
        Ok(latest) => latest,
        Err(err) => {
            return ChainProofStatus {
                schema_version,
                proof_contract,
                claim_boundary,
                status: "stale_or_invalid".to_string(),
                latest_world_head_proof: None,
                source_record_path: Some(latest_path.display().to_string()),
                load_error: Some(format!("parse latest execution record failed: {err}")),
                does_not_claim,
            };
        }
    };
    let record_schema_version = latest
        .get("schema_version")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(0);
    if record_schema_version < 3 {
        return ChainProofStatus {
            schema_version: "oasis7.chain_proof_status.v1".to_string(),
            proof_contract,
            claim_boundary,
            status: "stale_or_invalid".to_string(),
            latest_world_head_proof: None,
            source_record_path: Some(latest_path.display().to_string()),
            load_error: Some(format!(
                "execution_bridge_record_schema_v{record_schema_version}_has_no_world_head_proof"
            )),
            does_not_claim,
        };
    }

    let string_field = |name: &str| -> Result<String, String> {
        latest
            .get(name)
            .and_then(serde_json::Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned)
            .ok_or_else(|| format!("{name}_missing"))
    };
    let height = latest
        .get("height")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(0);
    if height == 0 {
        return ChainProofStatus {
            schema_version: "oasis7.chain_proof_status.v1".to_string(),
            proof_contract,
            claim_boundary,
            status: "stale_or_invalid".to_string(),
            latest_world_head_proof: None,
            source_record_path: Some(latest_path.display().to_string()),
            load_error: Some("height_missing".to_string()),
            does_not_claim,
        };
    }

    let latest_world_head_proof = match (
        string_field("world_id"),
        string_field("execution_block_hash"),
        string_field("execution_state_root"),
        string_field("node_block_hash"),
        string_field("action_root"),
        string_field("world_head_proof_ref"),
        string_field("world_head_proof_hash"),
    ) {
        (
            Ok(world_id),
            Ok(execution_block_hash),
            Ok(execution_state_root),
            Ok(node_block_hash),
            Ok(action_root),
            Ok(world_head_proof_ref),
            Ok(proof_hash),
        ) => LatestWorldHeadProofStatus {
            schema_version: 1,
            world_id,
            height,
            execution_block_hash,
            execution_state_root,
            node_block_hash,
            action_root,
            world_head_proof_ref,
            proof_hash,
            checkpoint_ref: latest
                .get("checkpoint_ref")
                .and_then(serde_json::Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToOwned::to_owned),
        },
        _ => {
            let missing_fields = [
                "world_id",
                "execution_block_hash",
                "execution_state_root",
                "node_block_hash",
                "action_root",
                "world_head_proof_ref",
                "world_head_proof_hash",
            ]
            .into_iter()
            .filter(|field| string_field(field).is_err())
            .collect::<Vec<_>>();
            return ChainProofStatus {
                schema_version: "oasis7.chain_proof_status.v1".to_string(),
                proof_contract,
                claim_boundary,
                status: "stale_or_invalid".to_string(),
                latest_world_head_proof: None,
                source_record_path: Some(latest_path.display().to_string()),
                load_error: Some(format!(
                    "latest execution record missing proof metadata: {}",
                    missing_fields.join(",")
                )),
                does_not_claim,
            };
        }
    };

    ChainProofStatus {
        schema_version,
        proof_contract,
        claim_boundary,
        status: "available".to_string(),
        latest_world_head_proof: Some(latest_world_head_proof),
        source_record_path: Some(latest_path.display().to_string()),
        load_error: None,
        does_not_claim,
    }
}
