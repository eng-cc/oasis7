//! Proven journal ranges and privately verified continuation cursors.
use super::*;

/// Formal Changes are computed from a complete already-authorized projection.
/// The global retention floor and high-watermark make an empty batch and sparse
/// Agent event IDs distinguishable from missing or expired history.
pub fn build_formal_changes_page(
    from_anchor: FormalReadAnchorRefV1,
    from_exclusive: FormalBusinessChangesCursorV1,
    target_request: ReadWorldViewRequest,
    target_view: ReadWorldViewResponse<WorldServiceProjection>,
    max_items: u32,
    max_bytes: u64,
) -> Result<FormalBusinessChangesPageV1<WorldServiceProjection>, String> {
    if max_items == 0
        || max_bytes == 0
        || max_items as usize > MAX_PAGE_ITEMS
        || max_bytes > MAX_PAGE_BYTES as u64
    {
        return Err("formal Changes page bounds".into());
    }
    target_view
        .validate(&target_request)
        .map_err(|error| error.to_string())?;
    let metadata = projection_metadata(&target_view.view)?;
    let expected_stream_id = authority::request_digest(
        "formal-events-v1",
        &(&target_request.world, &target_request.scope_id),
    )?;
    if target_request.scope_id != from_exclusive.scope_id
        || from_exclusive.stream_id != expected_stream_id
        || target_request.world.world_id != from_exclusive.commit.world.world_id
        || target_request.world.genesis_digest != from_exclusive.commit.world.genesis_digest
        || target_request.formal_anchor.as_ref() != Some(&from_anchor)
        || from_anchor.position != from_exclusive.commit.position
        || from_anchor.record_id != from_exclusive.commit.execution_block_hash
        || from_exclusive.event_id_era != metadata.event_id_era
        || from_exclusive.sequence > metadata.journal_high_watermark
        || from_exclusive.sequence.saturating_add(1) < metadata.journal_retention_floor
        || target_view.continuation.era != metadata.event_id_era
        || target_view.continuation.sequence != metadata.journal_high_watermark
        || target_view.continuation.stream_id != expected_stream_id
    {
        return Err(
            "HistoryUnavailable: formal Changes cursor is outside retained verified history".into(),
        );
    }
    let target_commit = target_view.version.commit.clone();
    if target_view.continuation.stream_id.trim().is_empty()
        || target_view.continuation.scope_id != from_exclusive.scope_id
        || target_view.continuation.commit != target_commit
    {
        return Err("formal Changes target continuation mismatch".into());
    }
    let mut visible = target_view
        .view
        .events
        .iter()
        .filter(|event| event.id > from_exclusive.sequence)
        .map(|event| {
            Ok((
                event.id,
                serde_json::to_value(event).map_err(|error| error.to_string())?,
            ))
        })
        .collect::<Result<Vec<_>, String>>()?;
    visible.sort_by_key(|(sequence, _)| *sequence);
    if visible.windows(2).any(|pair| pair[0].0 >= pair[1].0)
        || visible
            .iter()
            .any(|(sequence, _)| *sequence > metadata.journal_high_watermark)
    {
        return Err("formal Changes projection has invalid event sequence".into());
    }
    let item_limit = max_items as usize;
    let more_visible = visible.len() > item_limit;
    let visible_count = visible.len();
    visible.truncate(item_limit);
    let mut page = FormalBusinessChangesPageV1 {
        schema_version: FORMAL_BUSINESS_RESULT_SCHEMA_V1,
        from_anchor,
        from_exclusive: from_exclusive.clone(),
        to_inclusive: from_exclusive.clone(),
        next_cursor: from_exclusive.clone(),
        target_request,
        target_view,
        changes: Vec::new(),
    };
    for (sequence, change) in visible {
        let cursor = FormalBusinessChangesCursorV1 {
            stream_id: page.target_view.continuation.stream_id.clone(),
            scope_id: from_exclusive.scope_id.clone(),
            event_id_era: metadata.event_id_era,
            sequence,
            commit: target_commit.clone(),
        };
        page.changes.push(FormalBusinessChangeV1 { cursor, change });
        page.to_inclusive = page.changes.last().unwrap().cursor.clone();
        page.next_cursor = page.to_inclusive.clone();
        if serde_json::to_vec(&page)
            .map_err(|error| error.to_string())?
            .len()
            > (max_bytes as usize).min(MAX_PAGE_BYTES)
        {
            page.changes.pop();
            if let Some(last) = page.changes.last() {
                page.to_inclusive = last.cursor.clone();
                page.next_cursor = page.to_inclusive.clone();
            } else {
                return Err("formal Changes response bound cannot hold one event".into());
            }
            break;
        }
    }
    let selected_count = if more_visible {
        item_limit
    } else {
        visible_count
    };
    if page.changes.len() == item_limit && more_visible {
        // Stop at the last delivered visible event so the next page cannot
        // skip a scoped event hidden behind a numerical event-ID gap.
    } else if page.changes.len() == selected_count {
        // No authorized event remains in this retained interval. Advancing to
        // the global watermark proves that the page examined the whole range.
        page.to_inclusive = FormalBusinessChangesCursorV1 {
            stream_id: page.target_view.continuation.stream_id.clone(),
            scope_id: from_exclusive.scope_id.clone(),
            event_id_era: metadata.event_id_era,
            sequence: metadata.journal_high_watermark,
            commit: target_commit,
        };
        page.next_cursor = page.to_inclusive.clone();
    }
    page.validate().map_err(|error| error.to_string())?;
    if serde_json::to_vec(&page)
        .map_err(|error| error.to_string())?
        .len()
        > (max_bytes as usize).min(MAX_PAGE_BYTES)
    {
        return Err("formal Changes response exceeds its byte limit".into());
    }
    Ok(page)
}

#[derive(Debug, Clone, Copy)]
struct ProjectionRangeMetadata {
    event_id_era: u64,
    journal_retention_floor: u64,
    journal_high_watermark: u64,
}

fn projection_metadata(
    projection: &WorldServiceProjection,
) -> Result<ProjectionRangeMetadata, String> {
    let header = projection
        .formal_commit_header
        .as_ref()
        .ok_or("formal Changes projection lacks retention metadata")?;
    Ok(ProjectionRangeMetadata {
        event_id_era: header.event_id_era,
        journal_retention_floor: header.journal_retention_floor,
        journal_high_watermark: header.journal_high_watermark,
    })
}

/// The new cursor has a dedicated era field: it is the committed journal ID
/// era, not a reorg generation or the legacy route's cursor era.
pub type FormalBusinessChangesCursorV1 =
    oasis7_client_api::world_service::FormalBusinessChangesCursorV1;

/// Private cursor minted only from a verified full audience projection.
#[derive(Debug, Clone)]
pub struct VerifiedFormalChangesCursorV1 {
    cursor: FormalBusinessChangesCursorV1,
    anchor: FormalReadAnchorRefV1,
    read_authority_digest: String,
    reader_public_key: String,
}
impl VerifiedFormalChangesCursorV1 {
    /// Mint a cursor from an ordinary view which already holds qualified proof
    /// and exact configured read-authority lineage.
    pub(crate) fn from_world_view(
        view: &super::super::verified_view::VerifiedWorldView,
        reader_public_key: &str,
        read_authority_digest: &str,
    ) -> Result<Self, String> {
        if !view.read_authority_matches(read_authority_digest) {
            return Err("formal Changes view belongs to another read authority".into());
        }
        let anchor = view
            .formal_read_anchor()
            .ok_or("formal Changes view lacks qualified anchor")?;
        let commit = view
            .formal_commit()
            .ok_or("formal Changes view lacks qualified commit")?;
        let metadata = metadata_from_projection(view.projection(), commit.record())?;
        if committed_reference(commit.record(), &metadata, view.projection())?
            != view.version().commit
        {
            return Err("formal Changes view aliases differ from qualified proof".into());
        }
        let cursor = FormalBusinessChangesCursorV1 {
            stream_id: view.continuation().stream_id.clone(),
            scope_id: view.version().visibility_scope.clone(),
            event_id_era: metadata.event_id_era,
            sequence: metadata.journal_high_watermark,
            commit: view.version().commit.clone(),
        };
        let expected_stream = authority::request_digest(
            "formal-events-v1",
            &(&cursor.commit.world, &cursor.scope_id),
        )?;
        if cursor.stream_id != expected_stream
            || view.continuation().sequence != cursor.sequence
            || view.continuation().era != cursor.event_id_era
            || view.continuation().commit != cursor.commit
        {
            return Err("formal Changes view continuation differs from committed journal".into());
        }
        Ok(Self {
            cursor,
            anchor: FormalReadAnchorRefV1 {
                position: commit.record().position,
                record_id: commit.head().record_id,
                active_authority_id: anchor
                    .authority()
                    .authority_id()
                    .map_err(|e| e.to_string())?,
            },
            read_authority_digest: read_authority_digest.into(),
            reader_public_key: reader_public_key.into(),
        })
    }

    pub(crate) fn anchor(&self) -> &FormalReadAnchorRefV1 {
        &self.anchor
    }
    pub fn from_verified_view(
        verified: &VerifiedFormalProjectionV1,
        response: &ReadWorldViewResponse<WorldServiceProjection>,
        request: &ReadWorldViewRequest,
        reader_public_key: &str,
        read_authority_digest: &str,
    ) -> Result<Self, String> {
        verified.matches(response, request)?;
        if response.formal_profile.as_ref() != Some(&FormalReadProfileV1::controlled())
            || response.execution_evidence_scope.is_some()
            || request.scope_id != response.version.visibility_scope
        {
            return Err("formal Changes cursor requires the exact verified read authority".into());
        }
        let record = verified.commit().record();
        let metadata = metadata_from_projection(&response.view, record)?;
        let verified_commit = committed_reference(record, &metadata, &response.view)?;
        if response.version.commit != verified_commit {
            return Err("formal Changes cursor CommitRef aliases differ from proof".into());
        }
        let cursor = FormalBusinessChangesCursorV1 {
            stream_id: response.continuation.stream_id.clone(),
            scope_id: request.scope_id.clone(),
            event_id_era: metadata.event_id_era,
            sequence: metadata.journal_high_watermark,
            commit: response.version.commit.clone(),
        };
        let anchor = FormalReadAnchorRefV1 {
            position: verified.commit().record().position,
            record_id: verified.commit().head().record_id,
            active_authority_id: verified
                .read_anchor()
                .authority()
                .authority_id()
                .map_err(|error| error.to_string())?,
        };
        if anchor.position != cursor.commit.position
            || anchor.record_id != cursor.commit.execution_block_hash
            || read_authority_digest.trim().is_empty()
        {
            return Err("formal Changes cursor head/authority mismatch".into());
        }
        Ok(Self {
            cursor,
            anchor,
            read_authority_digest: read_authority_digest.into(),
            reader_public_key: reader_public_key.into(),
        })
    }
    pub fn cursor(&self) -> &FormalBusinessChangesCursorV1 {
        &self.cursor
    }
}

#[derive(Debug, Clone)]
pub struct VerifiedFormalChangesPageV1 {
    changes: Vec<FormalBusinessChangeV1<Value>>,
    next: VerifiedFormalChangesCursorV1,
}
impl VerifiedFormalChangesPageV1 {
    pub fn changes(&self) -> &[FormalBusinessChangeV1<Value>] {
        &self.changes
    }
    pub fn next_cursor(&self) -> &VerifiedFormalChangesCursorV1 {
        &self.next
    }
}

/// The source cursor must be from the same exact read-authority identity,
/// caller key, scope, and current verified head. The current full projection
/// supplies range completeness; changes are independently compared against it.
pub fn verify_formal_changes_page(
    verifier: &mut FormalReadVerifier,
    source: &VerifiedFormalChangesCursorV1,
    read_authority_digest: &str,
    page: &FormalBusinessChangesPageV1<WorldServiceProjection>,
    reader_public_key: &str,
) -> Result<VerifiedFormalChangesPageV1, String> {
    page.validate().map_err(|error| error.to_string())?;
    if source.read_authority_digest != read_authority_digest
        || source.reader_public_key != reader_public_key
        || page.from_anchor != source.anchor
        || page.from_exclusive != source.cursor
        || page.target_request.scope_id != source.cursor.scope_id
        || page.target_request.world != source.cursor.commit.world
        || verifier.anchor_ref()? != source.anchor
    {
        return Err(
            "HistoryUnavailable: formal Changes source cursor authority/audience/head mismatch"
                .into(),
        );
    }
    let mut next_verifier = verifier.clone();
    let verified =
        next_verifier.verify_view(&page.target_view, &page.target_request, reader_public_key)?;
    let metadata = projection_metadata(&page.target_view.view)?;
    let expected_stream_id = authority::request_digest(
        "formal-events-v1",
        &(&page.target_request.world, &page.target_request.scope_id),
    )?;
    if page.from_exclusive.event_id_era != metadata.event_id_era
        || page.from_exclusive.stream_id != expected_stream_id
        || page.target_view.continuation.era != metadata.event_id_era
        || page.target_view.continuation.sequence != metadata.journal_high_watermark
        || page.from_exclusive.sequence > metadata.journal_high_watermark
        || page.from_exclusive.sequence.saturating_add(1) < metadata.journal_retention_floor
        || page.to_inclusive.sequence > metadata.journal_high_watermark
        || page.to_inclusive.sequence < page.from_exclusive.sequence
        || page.to_inclusive.commit != page.target_view.version.commit
        || page.target_view.continuation.stream_id != page.from_exclusive.stream_id
        || page.target_view.continuation.scope_id != page.from_exclusive.scope_id
    {
        return Err(
            "HistoryUnavailable: formal Changes range is expired or outside the committed journal"
                .into(),
        );
    }
    let expected = page
        .target_view
        .view
        .events
        .iter()
        .filter(|event| {
            event.id > page.from_exclusive.sequence && event.id <= page.to_inclusive.sequence
        })
        .map(|event| {
            Ok((
                event.id,
                serde_json::to_value(event).map_err(|error| error.to_string())?,
            ))
        })
        .collect::<Result<Vec<_>, String>>()?;
    if expected.len() != page.changes.len()
        || expected
            .iter()
            .zip(&page.changes)
            .any(|((sequence, value), change)| {
                *sequence != change.cursor.sequence || value != &change.change
            })
    {
        return Err("formal Changes page omits, reorders, or alters an authorized event".into());
    }
    if page.to_inclusive.sequence != metadata.journal_high_watermark
        && page.changes.last().map(|change| change.cursor.sequence)
            != Some(page.to_inclusive.sequence)
    {
        return Err("formal Changes page stops between visible events".into());
    }
    let next = VerifiedFormalChangesCursorV1::from_verified_view(
        &verified,
        &page.target_view,
        &page.target_request,
        reader_public_key,
        read_authority_digest,
    )?;
    let mut next = next;
    next.cursor = page.next_cursor.clone();
    next.anchor = FormalReadAnchorRefV1 {
        position: verified.commit().record().position,
        record_id: verified.commit().head().record_id,
        active_authority_id: verified
            .read_anchor()
            .authority()
            .authority_id()
            .map_err(|error| error.to_string())?,
    };
    *verifier = next_verifier;
    Ok(VerifiedFormalChangesPageV1 {
        changes: page.changes.clone(),
        next,
    })
}
