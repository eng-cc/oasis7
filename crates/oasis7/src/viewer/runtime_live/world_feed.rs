use super::*;
use crate::viewer::protocol;
use serde::{Deserialize, Serialize};

const DEFAULT_LIMIT: usize = 50;
const MAX_LIMIT: usize = 200;

#[derive(Debug, Serialize, Deserialize)]
struct Cursor {
    world_id: String,
    reorg_epoch: u64,
    last_event_seq: u64,
}

pub(super) fn build_world_feed(
    world_id: &str,
    reorg_epoch: u64,
    journal: &RuntimeJournal,
    cursor: Option<&str>,
    requested_limit: usize,
) -> protocol::WorldFeedEnvelope {
    let decoded = cursor.map(decode_cursor);
    let gap_reason = match decoded.as_ref() {
        Some(Err(())) => Some(protocol::WorldFeedGapReason::CursorInvalid),
        Some(Ok(cursor)) if cursor.world_id != world_id => {
            Some(protocol::WorldFeedGapReason::CursorInvalid)
        }
        Some(Ok(cursor)) if cursor.reorg_epoch != reorg_epoch => {
            Some(protocol::WorldFeedGapReason::ReorgEpochChanged)
        }
        Some(Ok(cursor)) => cursor_gap_reason(journal, cursor.last_event_seq),
        None => None,
    };
    if let Some(reason) = gap_reason {
        return envelope(
            world_id,
            reorg_epoch,
            encode_cursor(world_id, reorg_epoch, latest_seq(journal)),
            Vec::new(),
            protocol::WorldFeedStatus::Gap,
            Some(reason),
        );
    }

    let after_seq = decoded
        .as_ref()
        .and_then(|cursor| cursor.as_ref().ok())
        .map_or(0, |cursor| cursor.last_event_seq);
    let limit = if requested_limit == 0 {
        DEFAULT_LIMIT
    } else {
        requested_limit.min(MAX_LIMIT)
    };
    let mut source_events: Vec<_> = journal
        .events
        .iter()
        .filter(|event| event.id > after_seq)
        .collect();
    source_events.sort_by_key(|event| event.id);
    source_events.dedup_by_key(|event| event.id);
    let events: Vec<_> = source_events
        .into_iter()
        .take(limit)
        .map(project_event)
        .collect();
    let last_seq = events.last().map_or(after_seq, |event| event.event_seq);
    let status = if events.is_empty() {
        protocol::WorldFeedStatus::Empty
    } else if cursor.is_some() {
        protocol::WorldFeedStatus::Replay
    } else {
        protocol::WorldFeedStatus::Ready
    };
    envelope(
        world_id,
        reorg_epoch,
        encode_cursor(world_id, reorg_epoch, last_seq),
        events,
        status,
        None,
    )
}

fn envelope(
    world_id: &str,
    reorg_epoch: u64,
    cursor: String,
    events: Vec<protocol::WorldFeedEvent>,
    status: protocol::WorldFeedStatus,
    gap_reason: Option<protocol::WorldFeedGapReason>,
) -> protocol::WorldFeedEnvelope {
    protocol::WorldFeedEnvelope {
        schema_version: protocol::WORLD_FEED_SCHEMA_VERSION.to_string(),
        world_id: world_id.to_string(),
        reorg_epoch,
        cursor,
        events,
        status,
        gap_reason,
        unavailable_reason: None,
        snapshot_reload_required: status == protocol::WorldFeedStatus::Gap,
    }
}

fn cursor_gap_reason(
    journal: &RuntimeJournal,
    last_event_seq: u64,
) -> Option<protocol::WorldFeedGapReason> {
    let first = journal.events.iter().map(|event| event.id).min();
    let latest = latest_seq(journal);
    if last_event_seq > latest {
        Some(protocol::WorldFeedGapReason::CursorInvalid)
    } else if first.is_some_and(|first| last_event_seq.saturating_add(1) < first) {
        Some(protocol::WorldFeedGapReason::CursorGap)
    } else {
        None
    }
}

fn latest_seq(journal: &RuntimeJournal) -> u64 {
    journal
        .events
        .iter()
        .map(|event| event.id)
        .max()
        .unwrap_or(0)
}

fn project_event(event: &crate::runtime::WorldEvent) -> protocol::WorldFeedEvent {
    let body = serde_json::to_value(&event.body).unwrap_or(serde_json::Value::Null);
    let source_kind = body
        .get("kind")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("RuntimeEvent");
    let kind = snake_case(source_kind);
    protocol::WorldFeedEvent {
        event_seq: event.id,
        summary: format!("{} event", kind.replace('_', " ")),
        kind,
        detail: serde_json::to_string(&event.body).unwrap_or_else(|_| "null".to_string()),
        // No durable runtime receipt identity is available yet; never infer one.
        receipt_ref: None,
    }
}

fn snake_case(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    for (index, character) in value.chars().enumerate() {
        if character.is_ascii_uppercase() {
            if index > 0 {
                output.push('_');
            }
            output.push(character.to_ascii_lowercase());
        } else {
            output.push(character);
        }
    }
    output
}

fn encode_cursor(world_id: &str, reorg_epoch: u64, last_event_seq: u64) -> String {
    let cursor = Cursor {
        world_id: world_id.to_string(),
        reorg_epoch,
        last_event_seq,
    };
    let bytes = serde_json::to_vec(&cursor).expect("serialize world feed cursor");
    format!("wf1.{}", hex::encode(bytes))
}

fn decode_cursor(value: &str) -> Result<Cursor, ()> {
    let bytes = hex::decode(value.strip_prefix("wf1.").ok_or(())?).map_err(|_| ())?;
    serde_json::from_slice(&bytes).map_err(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::{Journal, SnapshotMeta, WorldEvent, WorldEventBody};

    fn event(id: u64) -> WorldEvent {
        WorldEvent {
            id,
            time: id,
            caused_by: None,
            body: WorldEventBody::SnapshotCreated(SnapshotMeta {
                journal_len: id as usize,
            }),
        }
    }

    #[test]
    fn projection_is_ascending_deduplicated_and_receipt_free() {
        let journal = Journal {
            events: vec![event(3), event(1), event(3), event(2)],
        };
        let feed = build_world_feed("world-a", 4, &journal, None, 50);
        assert_eq!(feed.status, protocol::WorldFeedStatus::Ready);
        assert_eq!(
            feed.events
                .iter()
                .map(|event| event.event_seq)
                .collect::<Vec<_>>(),
            vec![1, 2, 3]
        );
        assert!(feed.events.iter().all(|event| event.receipt_ref.is_none()));
        assert!(!feed.snapshot_reload_required);
    }

    #[test]
    fn cursor_replays_only_newer_events() {
        let journal = Journal {
            events: vec![event(1), event(2), event(3)],
        };
        let first = build_world_feed("world-a", 4, &journal, None, 2);
        let replay = build_world_feed("world-a", 4, &journal, Some(&first.cursor), 50);
        assert_eq!(replay.status, protocol::WorldFeedStatus::Replay);
        assert_eq!(
            replay
                .events
                .iter()
                .map(|event| event.event_seq)
                .collect::<Vec<_>>(),
            vec![3]
        );
    }

    #[test]
    fn reorg_and_retention_gap_require_snapshot_reload() {
        let journal = Journal {
            events: vec![event(5), event(6)],
        };
        let stale = encode_cursor("world-a", 3, 4);
        let reorg = build_world_feed("world-a", 4, &journal, Some(&stale), 50);
        assert_eq!(
            reorg.gap_reason,
            Some(protocol::WorldFeedGapReason::ReorgEpochChanged)
        );
        assert!(reorg.snapshot_reload_required);

        let evicted = encode_cursor("world-a", 4, 2);
        let gap = build_world_feed("world-a", 4, &journal, Some(&evicted), 50);
        assert_eq!(
            gap.gap_reason,
            Some(protocol::WorldFeedGapReason::CursorGap)
        );
        assert!(gap.snapshot_reload_required);
    }

    #[test]
    fn invalid_cursor_fails_closed_as_gap() {
        let feed = build_world_feed("world-a", 4, &Journal::new(), Some("invalid"), 50);
        assert_eq!(feed.status, protocol::WorldFeedStatus::Gap);
        assert_eq!(
            feed.gap_reason,
            Some(protocol::WorldFeedGapReason::CursorInvalid)
        );
        assert!(feed.snapshot_reload_required);
    }
}
