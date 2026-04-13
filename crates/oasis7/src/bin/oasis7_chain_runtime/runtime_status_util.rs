use super::*;

pub(super) fn consensus_status_to_string(status: PosConsensusStatus) -> String {
    match status {
        PosConsensusStatus::Pending => "pending".to_string(),
        PosConsensusStatus::Committed => "committed".to_string(),
        PosConsensusStatus::Rejected => "rejected".to_string(),
    }
}

pub(super) fn now_unix_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or(0)
}
