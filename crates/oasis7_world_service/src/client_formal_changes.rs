//! Ordinary Changes consume privately verified cursor lineage.
use super::*;
use crate::formal_result::{
    VerifiedFormalChangesCursorV1, VerifiedFormalChangesPageV1,
};

impl RemoteWorldServiceClient {
    pub fn formal_changes_cursor(
        &self,
        view: &VerifiedWorldView,
    ) -> Result<VerifiedFormalChangesCursorV1, WorldServiceClientError> {
        let identity = self
            .read_authority_identity()
            .map_err(WorldServiceClientError::Assurance)?;
        let key = authority::sign_read_request(VIEW_PATH, (), &self.config.read_private_key_hex)
            .map_err(WorldServiceClientError::Assurance)?
            .subject_public_key;
        let cursor = VerifiedFormalChangesCursorV1::from_world_view(view, &key, &identity)
            .map_err(WorldServiceClientError::Assurance)?;
        if self.formal_anchor_ref()? != *cursor.anchor() {
            return Err(WorldServiceClientError::Assurance(
                "formal Changes view is not the current verified cache".into(),
            ));
        }
        Ok(cursor)
    }

    pub fn read_formal_changes(
        &self,
        source: &VerifiedFormalChangesCursorV1,
        max_items: u32,
        max_bytes: u64,
    ) -> Result<VerifiedFormalChangesPageV1, WorldServiceClientError> {
        if max_items == 0 || max_items > 1024 || max_bytes == 0 || max_bytes > 4 * 1024 * 1024 {
            return Err(WorldServiceClientError::Configuration(
                "formal Changes bounds".into(),
            ));
        }
        let read = self.prepare_formal_request(ReadWorldViewRequest {
            observation_nonce: Some("07".repeat(32)),
            contract_version: WORLD_SERVICE_CONTRACT_VERSION,
            world: self.config.expected_world.clone(),
            scope_id: self.config.scope_id.clone(),
            formal_anchor: Some(source.anchor().clone()),
            min_commit: None,
            fixed_commit: None,
            deadline_unix_ms: None,
        })?;
        let message = FormalAuthenticatedChangesV1 {
            schema_version: 1,
            cursor: source.cursor().clone(),
            max_items,
            max_bytes,
            read: authority::sign_read_request(VIEW_PATH, read, &self.config.read_private_key_hex)
                .map_err(WorldServiceClientError::Assurance)?,
        };
        let page: FormalBusinessChangesPageV1<WorldServiceProjection> =
            self.call(CHANGES_PATH, &message)?;
        if page.target_request != message.read.request
            || page.from_exclusive != message.cursor
            || page.changes.len() > max_items as usize
            || serde_json::to_vec(&page)
                .map_err(|e| WorldServiceClientError::Assurance(e.to_string()))?
                .len()
                > max_bytes as usize
        {
            return Err(WorldServiceClientError::Assurance(
                "formal Changes response differs from signed read or requested bounds".into(),
            ));
        }
        let identity = self
            .read_authority_identity()
            .map_err(WorldServiceClientError::Assurance)?;
        let mut verifier = self
            .formal_read
            .as_ref()
            .ok_or_else(|| {
                WorldServiceClientError::Configuration(
                    "formal Changes requires external proof trust".into(),
                )
            })?
            .lock()
            .map_err(|_| WorldServiceClientError::Assurance("formal cache poisoned".into()))?;
        crate::formal_result::verify_formal_changes_page(
            &mut verifier,
            source,
            &identity,
            &page,
            &message.read.subject_public_key,
        )
        .map_err(WorldServiceClientError::Assurance)
    }
}
