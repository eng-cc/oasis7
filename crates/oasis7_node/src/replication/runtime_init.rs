use super::*;

impl ReplicationRuntime {
    /// Construct the stateful attachment helper without replaying startup
    /// reconciliation. FetchCommit already selected one validated message;
    /// request-time checkpoint export must not rescan unrelated history.
    pub(crate) fn new_for_fetch_commit_attachment(
        config: &NodeReplicationConfig,
        node_id: &str,
    ) -> Result<Self, NodeError> {
        Self::from_config(config, node_id)
    }

    pub(super) fn from_config(
        config: &NodeReplicationConfig,
        node_id: &str,
    ) -> Result<Self, NodeError> {
        fs::create_dir_all(&config.root_dir).map_err(|err| NodeError::Replication {
            reason: format!(
                "create replication root {} failed: {}",
                config.root_dir.display(),
                err
            ),
        })?;

        let guard = load_json_or_default::<SingleWriterReplicationGuard>(
            config.guard_state_path().as_path(),
        )?;
        let remote_guards = load_json_or_default::<BTreeMap<String, SingleWriterReplicationGuard>>(
            config.remote_guard_state_path().as_path(),
        )?;
        let signer = config.signing_keypair()?;
        let mut writer_state =
            load_json_or_default::<LocalWriterState>(config.writer_state_path(node_id).as_path())?;
        if writer_state.writer_epoch == 0 {
            writer_state.writer_epoch = DEFAULT_WRITER_EPOCH;
        }
        if writer_state.last_sequence == 0
            && writer_state.last_replicated_height == 0
            && writer_state.writer_epoch == DEFAULT_WRITER_EPOCH
        {
            writer_state.writer_epoch =
                seeded_writer_epoch(signer.as_ref().map(|signer| signer.public_key_hex.as_str()));
        }

        Ok(Self {
            config: config.clone(),
            store: LocalCasStore::new(config.store_root()),
            guard,
            remote_guards,
            writer_state,
            enforce_signature: config.enforce_signature || signer.is_some(),
            remote_writer_allowlist: config.remote_writer_allowlist().clone(),
            signer,
        })
    }
}
