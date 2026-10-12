use crate::{
    error::{ContextError, Result},
    format,
    model::*,
    store::Store,
};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ListQuery {
    pub kind: Option<String>,
    pub environment: Option<String>,
    pub tags: Vec<String>,
    pub include_archived: bool,
    pub limit: usize,
    pub cursor: Option<String>,
}
impl Default for ListQuery {
    fn default() -> Self {
        Self {
            kind: None,
            environment: None,
            tags: vec![],
            include_archived: false,
            limit: 20,
            cursor: None,
        }
    }
}
#[derive(Clone, Debug)]
pub struct SearchQuery {
    pub query: String,
    pub filters: ListQuery,
}
#[derive(Debug, Serialize)]
pub struct EntrySummary {
    pub id: String,
    pub title: String,
    pub kind: String,
    pub environment: String,
    pub state: String,
    pub revision: String,
    pub verification_state: VerificationState,
    pub summary: String,
}
#[derive(Debug, Serialize)]
pub struct QueryResult {
    pub entries: Vec<EntrySummary>,
    pub complete: bool,
    pub total_matches: Option<usize>,
    pub has_more: bool,
    pub next_cursor: Option<String>,
    pub warnings: Vec<String>,
    pub read_at: String,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Cursor {
    version: String,
    store_id: String,
    query_sha256: String,
    view_sha256: String,
    next_offset: usize,
}
impl Store {
    pub fn list(&self, q: &ListQuery) -> Result<QueryResult> {
        self.query(q, None)
    }
    pub fn search(&self, q: &SearchQuery) -> Result<QueryResult> {
        self.query(&q.filters, Some(&q.query))
    }
    fn query(&self, q: &ListQuery, search: Option<&str>) -> Result<QueryResult> {
        if q.limit == 0
            || q.limit > 200
            || search.is_some_and(|s| s.len() > 512)
            || q.tags.len() > 32
        {
            return Err(ContextError::new("INVALID_INPUT", "invalid query limits"));
        }
        if q.kind
            .as_ref()
            .is_some_and(|s| !["machine", "environment", "runbook"].contains(&s.as_str()))
        {
            return Err(ContextError::new("INVALID_INPUT", "invalid kind filter"));
        }
        if let Some(e) = &q.environment {
            validate_id(e)?;
        }
        let terms: Vec<String> = search
            .unwrap_or("")
            .split_whitespace()
            .map(str::to_ascii_lowercase)
            .collect();
        let mut tags = q.tags.clone();
        tags.sort();
        tags.dedup();
        let query_digest = format::digest(&format::canonical_json(
            &serde_json::json!({"terms":terms,"kind":q.kind,"environment":q.environment,"tags":tags,"include_archived":q.include_archived,"order":"oasis7.context_order.v1","limit":q.limit}),
        ));
        let (mut entries, warnings) = self.scan()?;
        entries.sort_by(|a, b| a.entry.metadata.id.cmp(&b.entry.metadata.id));
        let view = format::digest(&format::canonical_json(
            &serde_json::json!({"config":format::digest(&self.config_raw()?),"entries":entries.iter().map(|e|(&e.entry.metadata.id,&e.revision)).collect::<Vec<_>>()}),
        ));
        let complete = warnings.is_empty();
        let offset = if let Some(c) = &q.cursor {
            if c.len() > 2048 {
                return Err(ContextError::new("INVALID_INPUT", "cursor exceeds limit"));
            }
            let bytes = URL_SAFE_NO_PAD
                .decode(c)
                .map_err(|_| ContextError::new("INVALID_INPUT", "invalid cursor"))?;
            let c: Cursor = format::strict_json_bounded(&bytes, 2048)?;
            if !complete
                || c.version != "oasis7.context_cursor.v1"
                || c.store_id != self.config.store_id
                || c.query_sha256 != query_digest
                || c.view_sha256 != view
                || c.next_offset > MAX_ENTRIES
            {
                return Err(ContextError::new("CURSOR_STALE", "query view changed"));
            }
            c.next_offset
        } else {
            0
        };
        let mut matches = Vec::new();
        for e in entries {
            let m = &e.entry.metadata;
            if (!q.include_archived && m.state != "active")
                || q.kind.as_ref().is_some_and(|k| k != &m.kind)
                || q.environment.as_ref().is_some_and(|x| x != &m.environment)
                || !tags.iter().all(|t| m.tags.contains(t))
            {
                continue;
            }
            let id = m.id.to_ascii_lowercase();
            let title = m.title.to_ascii_lowercase();
            let tags_text = m.tags.join(" ").to_ascii_lowercase();
            let env = m.environment.to_ascii_lowercase();
            let body = e.entry.body.to_ascii_lowercase();
            if !terms.iter().all(|t| {
                [&id, &title, &tags_text, &env, &body]
                    .iter()
                    .any(|s| s.contains(t))
            }) {
                continue;
            }
            let rank = if terms.len() == 1 && id == terms[0] {
                0
            } else if terms.iter().all(|t| title.contains(t)) {
                1
            } else if terms.iter().all(|t| tags_text.contains(t)) {
                2
            } else if terms.iter().all(|t| env.contains(t)) {
                3
            } else {
                4
            };
            matches.push((
                rank,
                EntrySummary {
                    id: m.id.clone(),
                    title: m.title.clone(),
                    kind: m.kind.clone(),
                    environment: m.environment.clone(),
                    state: m.state.clone(),
                    revision: e.revision,
                    verification_state: e.verification_state,
                    summary: e.entry.body.chars().take(320).collect(),
                },
            ));
        }
        matches.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.id.cmp(&b.1.id)));
        if offset > matches.len() {
            return Err(ContextError::new(
                "CURSOR_STALE",
                "cursor offset no longer valid",
            ));
        }
        let count = matches.len();
        let end = (offset + q.limit).min(count);
        let has_more = end < count;
        let next_cursor = if complete && has_more {
            Some(
                URL_SAFE_NO_PAD.encode(
                    serde_json::to_vec(&Cursor {
                        version: "oasis7.context_cursor.v1".into(),
                        store_id: self.config.store_id.clone(),
                        query_sha256: query_digest,
                        view_sha256: view,
                        next_offset: end,
                    })
                    .unwrap(),
                ),
            )
        } else {
            None
        };
        let entries = matches
            .into_iter()
            .skip(offset)
            .take(q.limit)
            .map(|x| x.1)
            .collect();
        let result = QueryResult {
            entries,
            complete,
            total_matches: complete.then_some(count),
            has_more,
            next_cursor,
            warnings,
            read_at: format::now(),
        };
        if serde_json::to_vec(&result).unwrap().len() > 2 * 1024 * 1024 {
            return Err(ContextError::new(
                "LIMIT_EXCEEDED",
                "query output exceeds limit",
            ));
        }
        Ok(result)
    }
}
