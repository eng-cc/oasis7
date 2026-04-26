use super::*;

pub(super) fn signing_key_from_hex(private_key_hex: &str) -> Result<SigningKey, NodeError> {
    let private_key = decode_hex_array::<32>(private_key_hex, "replication private key")?;
    Ok(SigningKey::from_bytes(&private_key))
}

pub(super) fn normalize_replication_public_key_hex_for_config(
    raw: &str,
    field: &str,
) -> Result<String, NodeError> {
    let normalized = raw.trim();
    if normalized.is_empty() {
        return Err(NodeError::InvalidConfig {
            reason: format!("{field} cannot be empty"),
        });
    }
    let bytes = hex::decode(normalized).map_err(|_| NodeError::InvalidConfig {
        reason: format!("{field} must be valid hex"),
    })?;
    let key_bytes: [u8; 32] = bytes.try_into().map_err(|_| NodeError::InvalidConfig {
        reason: format!("{field} must be 32-byte hex"),
    })?;
    Ok(hex::encode(key_bytes))
}

pub(super) fn normalize_replication_public_key_hex_for_request(
    raw: &str,
    field: &str,
) -> Result<String, NodeError> {
    let normalized = raw.trim();
    if normalized.is_empty() {
        return Err(NodeError::Replication {
            reason: format!("{field} cannot be empty"),
        });
    }
    let key_bytes = decode_hex_array::<32>(normalized, field)?;
    Ok(hex::encode(key_bytes))
}

pub(super) fn sign_replication_message(
    message: &GossipReplicationMessage,
    signer: &ReplicationSigningKey,
) -> Result<String, NodeError> {
    let payload = replication_signing_bytes(message)?;
    let signature: Signature = signer.signing_key.sign(&payload);
    Ok(hex::encode(signature.to_bytes()))
}

pub(super) fn verify_replication_message_signature(
    message: &GossipReplicationMessage,
) -> Result<(), NodeError> {
    let public_key_hex =
        message
            .public_key_hex
            .as_deref()
            .ok_or_else(|| NodeError::Replication {
                reason: "replication signature missing public_key_hex".to_string(),
            })?;
    let signature_hex = message
        .signature_hex
        .as_deref()
        .ok_or_else(|| NodeError::Replication {
            reason: "replication signature missing signature_hex".to_string(),
        })?;

    let public_key_bytes = decode_hex_array::<32>(public_key_hex, "replication public key")?;
    let signature_bytes = decode_hex_array::<64>(signature_hex, "replication signature")?;
    let public_key =
        VerifyingKey::from_bytes(&public_key_bytes).map_err(|err| NodeError::Replication {
            reason: format!("parse replication public key failed: {}", err),
        })?;
    let signature = Signature::from_bytes(&signature_bytes);
    let payload = replication_signing_bytes(message)?;
    public_key
        .verify(&payload, &signature)
        .map_err(|err| NodeError::Replication {
            reason: format!("verify replication signature failed: {}", err),
        })
}

pub(super) fn sign_fetch_commit_request(
    request: &FetchCommitRequest,
    signer: &ReplicationSigningKey,
) -> Result<String, NodeError> {
    let payload = fetch_commit_request_signing_bytes(request)?;
    let signature: Signature = signer.signing_key.sign(&payload);
    Ok(hex::encode(signature.to_bytes()))
}

pub(super) fn sign_fetch_blob_request(
    request: &FetchBlobRequest,
    signer: &ReplicationSigningKey,
) -> Result<String, NodeError> {
    let payload = fetch_blob_request_signing_bytes(request)?;
    let signature: Signature = signer.signing_key.sign(&payload);
    Ok(hex::encode(signature.to_bytes()))
}

pub(super) fn verify_signed_fetch_request(
    requester_public_key_hex: &str,
    requester_signature_hex: &str,
    payload: &[u8],
    request_label: &str,
) -> Result<(), NodeError> {
    let public_key_label = format!("{request_label} requester public key");
    let signature_label = format!("{request_label} requester signature");
    let public_key_bytes =
        decode_hex_array::<32>(requester_public_key_hex, public_key_label.as_str())?;
    let signature_bytes =
        decode_hex_array::<64>(requester_signature_hex, signature_label.as_str())?;
    let public_key =
        VerifyingKey::from_bytes(&public_key_bytes).map_err(|err| NodeError::Replication {
            reason: format!("parse {request_label} requester public key failed: {err}"),
        })?;
    let signature = Signature::from_bytes(&signature_bytes);
    public_key
        .verify(payload, &signature)
        .map_err(|err| NodeError::Replication {
            reason: format!("verify {request_label} requester signature failed: {err}"),
        })
}

pub(super) fn fetch_commit_request_signing_bytes(
    request: &FetchCommitRequest,
) -> Result<Vec<u8>, NodeError> {
    let payload = FetchCommitRequestSigningPayload {
        version: REPLICATION_VERSION,
        world_id: request.world_id.as_str(),
        height: request.height,
        requester_public_key_hex: request.requester_public_key_hex.as_deref(),
    };
    serde_json::to_vec(&payload).map_err(|err| NodeError::Replication {
        reason: format!("serialize fetch-commit signing payload failed: {err}"),
    })
}

pub(super) fn fetch_blob_request_signing_bytes(
    request: &FetchBlobRequest,
) -> Result<Vec<u8>, NodeError> {
    let payload = FetchBlobRequestSigningPayload {
        version: REPLICATION_VERSION,
        content_hash: request.content_hash.as_str(),
        requester_public_key_hex: request.requester_public_key_hex.as_deref(),
    };
    serde_json::to_vec(&payload).map_err(|err| NodeError::Replication {
        reason: format!("serialize fetch-blob signing payload failed: {err}"),
    })
}

pub(crate) fn load_commit_message_from_root(
    root_dir: &Path,
    world_id: &str,
    height: u64,
) -> Result<Option<GossipReplicationMessage>, NodeError> {
    let Some(source) =
        super::commit_retention::resolve_commit_message_readback_source(root_dir, height)?
    else {
        return Ok(None);
    };

    let message = match source {
        super::commit_retention::CommitMessageReadbackSource::HotMirror { path } => {
            let bytes = fs::read(&path).map_err(|err| NodeError::Replication {
                reason: format!("read {} failed: {}", path.display(), err),
            })?;
            serde_json::from_slice::<GossipReplicationMessage>(&bytes).map_err(|err| {
                NodeError::Replication {
                    reason: format!("parse {} failed: {}", path.display(), err),
                }
            })?
        }
        super::commit_retention::CommitMessageReadbackSource::ColdArchive { content_hash } => {
            let store = LocalCasStore::new(root_dir.join("store"));
            let bytes = store
                .get_verified(content_hash.as_str())
                .map_err(distfs_error_to_node_error)?;
            serde_json::from_slice::<GossipReplicationMessage>(&bytes).map_err(|err| {
                NodeError::Replication {
                    reason: format!(
                        "parse cold commit message for height {} hash {} failed: {}",
                        height, content_hash, err
                    ),
                }
            })?
        }
    };
    if message.version != REPLICATION_VERSION
        || message.world_id != world_id
        || message.record.world_id != world_id
    {
        return Ok(None);
    }
    Ok(Some(message))
}

pub(crate) fn load_blob_from_root(
    root_dir: &Path,
    content_hash: &str,
) -> Result<Option<Vec<u8>>, NodeError> {
    let store = LocalCasStore::new(root_dir.join("store"));
    match store.get(content_hash) {
        Ok(blob) => Ok(Some(blob)),
        Err(WorldError::BlobNotFound { .. }) => Ok(None),
        Err(err) => Err(distfs_error_to_node_error(err)),
    }
}

pub(super) fn load_json_or_default<T>(path: &Path) -> Result<T, NodeError>
where
    T: for<'de> Deserialize<'de> + Default,
{
    if !path.exists() {
        return Ok(T::default());
    }
    let bytes = fs::read(path).map_err(|err| NodeError::Replication {
        reason: format!("read {} failed: {}", path.display(), err),
    })?;
    serde_json::from_slice::<T>(&bytes).map_err(|err| NodeError::Replication {
        reason: format!("parse {} failed: {}", path.display(), err),
    })
}

pub(super) fn write_json_pretty<T: Serialize>(path: &Path, value: &T) -> Result<(), NodeError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| NodeError::Replication {
            reason: format!("create dir {} failed: {}", parent.display(), err),
        })?;
    }
    let bytes = serde_json::to_vec_pretty(value).map_err(|err| NodeError::Replication {
        reason: format!("serialize {} failed: {}", path.display(), err),
    })?;
    fs::write(path, bytes).map_err(|err| NodeError::Replication {
        reason: format!("write {} failed: {}", path.display(), err),
    })
}

pub(super) fn write_json_compact<T: Serialize>(path: &Path, value: &T) -> Result<(), NodeError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| NodeError::Replication {
            reason: format!("create dir {} failed: {}", parent.display(), err),
        })?;
    }
    let bytes = serde_json::to_vec(value).map_err(|err| NodeError::Replication {
        reason: format!("serialize {} failed: {}", path.display(), err),
    })?;
    fs::write(path, bytes).map_err(|err| NodeError::Replication {
        reason: format!("write {} failed: {}", path.display(), err),
    })
}

pub(super) fn distfs_error_to_node_error<E>(err: E) -> NodeError
where
    E: std::fmt::Debug,
{
    NodeError::Replication {
        reason: format!("{err:?}"),
    }
}

fn replication_signing_bytes(message: &GossipReplicationMessage) -> Result<Vec<u8>, NodeError> {
    let payload = ReplicationSigningPayload {
        version: message.version,
        world_id: message.world_id.as_str(),
        node_id: message.node_id.as_str(),
        record: &message.record,
        payload: &message.payload,
        public_key_hex: message.public_key_hex.as_deref(),
    };
    serde_json::to_vec(&payload).map_err(|err| NodeError::Replication {
        reason: format!("serialize replication signing payload failed: {}", err),
    })
}

fn decode_hex_array<const N: usize>(value: &str, label: &str) -> Result<[u8; N], NodeError> {
    let bytes = hex::decode(value).map_err(|_| NodeError::Replication {
        reason: format!("{} must be valid hex", label),
    })?;
    bytes.try_into().map_err(|_| NodeError::Replication {
        reason: format!("{} must be {} bytes hex", label, N),
    })
}
