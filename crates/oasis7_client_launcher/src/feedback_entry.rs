use serde::Serialize;
use serde_json::Value;
use std::collections::VecDeque;
use std::fs;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::time::Duration;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::http_helpers::{
    bracket_ipv6_authority_host, build_http_request_head, normalize_connect_host, parse_host_port,
    parse_http_status_code,
};

pub(crate) const DEFAULT_FEEDBACK_DIR: &str = "feedback";
const FEEDBACK_LOG_SNAPSHOT_LIMIT: usize = 200;
const CHAIN_FEEDBACK_SUBMIT_PATH: &str = "/v1/chain/feedback/submit";
const HTTP_TIMEOUT_MS: u64 = 3_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum FeedbackKind {
    Bug,
    Suggestion,
}

impl FeedbackKind {
    pub(crate) fn slug(self) -> &'static str {
        match self {
            Self::Bug => "bug",
            Self::Suggestion => "suggestion",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FeedbackDraft {
    pub(crate) kind: FeedbackKind,
    pub(crate) title: String,
    pub(crate) description: String,
    pub(crate) output_dir: String,
}

impl Default for FeedbackDraft {
    fn default() -> Self {
        Self {
            kind: FeedbackKind::Bug,
            title: String::new(),
            description: String::new(),
            output_dir: DEFAULT_FEEDBACK_DIR.to_string(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FeedbackDraftIssue {
    TitleRequired,
    DescriptionRequired,
    OutputDirRequired,
}

#[derive(Debug, Serialize)]
struct FeedbackReport {
    kind: FeedbackKind,
    title: String,
    description: String,
    created_at: String,
    launcher_config: Value,
    recent_logs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum FeedbackSubmitResult {
    Distributed {
        feedback_id: String,
        event_id: String,
    },
    Local {
        path: PathBuf,
        remote_error: Option<String>,
    },
}

#[derive(Debug, Serialize)]
struct RemoteFeedbackSubmitRequest {
    category: &'static str,
    title: String,
    description: String,
    platform: &'static str,
    game_version: &'static str,
}

#[derive(Debug, serde::Deserialize)]
struct RemoteFeedbackSubmitResponse {
    ok: bool,
    feedback_id: Option<String>,
    event_id: Option<String>,
    error: Option<String>,
}

pub(crate) fn validate_feedback_draft(draft: &FeedbackDraft) -> Vec<FeedbackDraftIssue> {
    let mut issues = Vec::new();
    if draft.title.trim().is_empty() {
        issues.push(FeedbackDraftIssue::TitleRequired);
    }
    if draft.description.trim().is_empty() {
        issues.push(FeedbackDraftIssue::DescriptionRequired);
    }
    if draft.output_dir.trim().is_empty() {
        issues.push(FeedbackDraftIssue::OutputDirRequired);
    }
    issues
}

pub(crate) fn collect_recent_logs(logs: &VecDeque<String>) -> Vec<String> {
    let mut recent = logs
        .iter()
        .rev()
        .take(FEEDBACK_LOG_SNAPSHOT_LIMIT)
        .cloned()
        .collect::<Vec<_>>();
    recent.reverse();
    recent
}

pub(crate) fn submit_feedback_report(
    draft: &FeedbackDraft,
    launcher_config: Value,
    recent_logs: Vec<String>,
) -> Result<PathBuf, String> {
    let output_dir = draft.output_dir.trim();
    if output_dir.is_empty() {
        return Err("feedback output directory cannot be empty".to_string());
    }

    let output_path = Path::new(output_dir);
    fs::create_dir_all(output_path)
        .map_err(|err| format!("create feedback directory `{output_dir}` failed: {err}"))?;

    let now = SystemTime::now();
    let report = FeedbackReport {
        kind: draft.kind,
        title: draft.title.trim().to_string(),
        description: draft.description.trim().to_string(),
        created_at: format_rfc3339_utc(now),
        launcher_config,
        recent_logs,
    };

    let file_name = format!(
        "{}-{}.json",
        format_filename_timestamp(now),
        draft.kind.slug()
    );
    let file_path = output_path.join(file_name);
    let content = serde_json::to_vec_pretty(&report)
        .map_err(|err| format!("serialize feedback report failed: {err}"))?;
    fs::write(&file_path, content).map_err(|err| {
        format!(
            "write feedback report `{}` failed: {err}",
            file_path.display()
        )
    })?;
    Ok(file_path)
}

pub(crate) fn submit_feedback_with_fallback(
    draft: &FeedbackDraft,
    launcher_config: Value,
    recent_logs: Vec<String>,
    chain_enabled: bool,
    chain_status_bind: &str,
) -> Result<FeedbackSubmitResult, String> {
    if chain_enabled {
        match submit_feedback_remote(draft, chain_status_bind) {
            Ok((feedback_id, event_id)) => {
                return Ok(FeedbackSubmitResult::Distributed {
                    feedback_id,
                    event_id,
                });
            }
            Err(remote_error) => {
                let path = submit_feedback_report(draft, launcher_config, recent_logs)?;
                return Ok(FeedbackSubmitResult::Local {
                    path,
                    remote_error: Some(remote_error),
                });
            }
        }
    }

    let path = submit_feedback_report(draft, launcher_config, recent_logs)?;
    Ok(FeedbackSubmitResult::Local {
        path,
        remote_error: None,
    })
}

fn submit_feedback_remote(
    draft: &FeedbackDraft,
    chain_status_bind: &str,
) -> Result<(String, String), String> {
    let request = RemoteFeedbackSubmitRequest {
        category: draft.kind.slug(),
        title: draft.title.trim().to_string(),
        description: draft.description.trim().to_string(),
        platform: "client_launcher",
        game_version: "unknown",
    };
    let payload = serde_json::to_vec(&request)
        .map_err(|err| format!("serialize remote feedback request failed: {err}"))?;
    let response = post_json_request(chain_status_bind, CHAIN_FEEDBACK_SUBMIT_PATH, &payload)?;
    if !response.ok {
        return Err(response
            .error
            .unwrap_or_else(|| "remote feedback submit rejected".to_string()));
    }
    let feedback_id = response
        .feedback_id
        .ok_or_else(|| "remote feedback submit missing feedback_id".to_string())?;
    let event_id = response
        .event_id
        .ok_or_else(|| "remote feedback submit missing event_id".to_string())?;
    Ok((feedback_id, event_id))
}

fn post_json_request(
    bind: &str,
    path: &str,
    payload: &[u8],
) -> Result<RemoteFeedbackSubmitResponse, String> {
    let (host, port) = parse_host_port(bind, "chain status bind")?;
    let host = normalize_connect_host(host.as_str());
    let socket_host = bracket_ipv6_authority_host(host.as_str());
    let mut stream = TcpStream::connect(format!("{socket_host}:{port}"))
        .map_err(|err| format!("connect chain status server failed: {err}"))?;
    let timeout = Some(Duration::from_millis(HTTP_TIMEOUT_MS));
    let _ = stream.set_read_timeout(timeout);
    let _ = stream.set_write_timeout(timeout);

    let host_header = bracket_ipv6_authority_host(host.as_str());
    let request_head = build_http_request_head(
        "POST",
        path,
        host_header.as_str(),
        port,
        Some(payload.len()),
    );

    stream
        .write_all(request_head.as_bytes())
        .map_err(|err| format!("write request header failed: {err}"))?;
    stream
        .write_all(payload)
        .map_err(|err| format!("write request body failed: {err}"))?;
    stream
        .flush()
        .map_err(|err| format!("flush request failed: {err}"))?;

    let mut response_bytes = Vec::new();
    stream
        .read_to_end(&mut response_bytes)
        .map_err(|err| format!("read response failed: {err}"))?;
    parse_http_json_response(&response_bytes)
}

fn parse_http_json_response(bytes: &[u8]) -> Result<RemoteFeedbackSubmitResponse, String> {
    let Some(boundary) = bytes.windows(4).position(|window| window == b"\r\n\r\n") else {
        return Err("invalid HTTP response: missing header terminator".to_string());
    };
    let header = std::str::from_utf8(&bytes[..boundary])
        .map_err(|_| "invalid HTTP response: header is not UTF-8".to_string())?;
    let body = &bytes[(boundary + 4)..];
    let status_code = parse_http_status_code(header)?;
    let response: RemoteFeedbackSubmitResponse =
        serde_json::from_slice(body).map_err(|err| format!("parse response json failed: {err}"))?;

    if !(200..=299).contains(&status_code) {
        return Err(response
            .error
            .unwrap_or_else(|| format!("remote feedback submit failed with HTTP {status_code}")));
    }
    Ok(response)
}

fn unix_seconds(now: SystemTime) -> i64 {
    match now.duration_since(UNIX_EPOCH) {
        Ok(duration) => duration.as_secs().min(i64::MAX as u64) as i64,
        Err(_) => 0,
    }
}

fn format_rfc3339_utc(now: SystemTime) -> String {
    let unix = unix_seconds(now);
    let (year, month, day, hour, minute, second) = split_unix_time(unix);
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z")
}

fn format_filename_timestamp(now: SystemTime) -> String {
    let unix = unix_seconds(now);
    let (year, month, day, hour, minute, second) = split_unix_time(unix);
    format!("{year:04}{month:02}{day:02}T{hour:02}{minute:02}{second:02}Z")
}

fn split_unix_time(unix_seconds: i64) -> (i32, u32, u32, u32, u32, u32) {
    let days = unix_seconds.div_euclid(86_400);
    let seconds_of_day = unix_seconds.rem_euclid(86_400);
    let (year, month, day) = civil_from_days(days);
    let hour = (seconds_of_day / 3_600) as u32;
    let minute = ((seconds_of_day % 3_600) / 60) as u32;
    let second = (seconds_of_day % 60) as u32;
    (year, month, day, hour, minute, second)
}

fn civil_from_days(days_since_unix_epoch: i64) -> (i32, u32, u32) {
    let z = days_since_unix_epoch + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let day_of_era = z - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_prime = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_prime + 2) / 5 + 1;
    let month = month_prime + if month_prime < 10 { 3 } else { -9 };
    let year = year + if month <= 2 { 1 } else { 0 };
    (year as i32, month as u32, day as u32)
}

#[cfg(test)]
mod tests {
    use super::{
        FeedbackDraft, FeedbackDraftIssue, FeedbackKind, FeedbackSubmitResult, collect_recent_logs,
        format_filename_timestamp, parse_host_port, parse_http_json_response,
        submit_feedback_report, submit_feedback_with_fallback, validate_feedback_draft,
    };
    use std::collections::VecDeque;
    use std::time::{Duration, UNIX_EPOCH};

    #[test]
    fn validate_feedback_draft_reports_missing_required_fields() {
        let draft = FeedbackDraft {
            kind: FeedbackKind::Bug,
            title: " ".to_string(),
            description: "".to_string(),
            output_dir: "".to_string(),
        };

        let issues = validate_feedback_draft(&draft);
        assert!(issues.contains(&FeedbackDraftIssue::TitleRequired));
        assert!(issues.contains(&FeedbackDraftIssue::DescriptionRequired));
        assert!(issues.contains(&FeedbackDraftIssue::OutputDirRequired));
    }

    #[test]
    fn collect_recent_logs_keeps_latest_entries_only() {
        let mut logs = VecDeque::new();
        for index in 0..220 {
            logs.push_back(format!("line-{index}"));
        }

        let snapshot = collect_recent_logs(&logs);
        assert_eq!(snapshot.len(), 200);
        assert_eq!(snapshot.first().map(String::as_str), Some("line-20"));
        assert_eq!(snapshot.last().map(String::as_str), Some("line-219"));
    }

    #[test]
    fn submit_feedback_report_writes_json_bundle() {
        let temp_dir =
            std::env::temp_dir().join(format!("oasis7-feedback-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&temp_dir);

        let draft = FeedbackDraft {
            kind: FeedbackKind::Suggestion,
            title: "Need better explanation".to_string(),
            description: "Mission progress should explain step conditions".to_string(),
            output_dir: temp_dir.to_string_lossy().to_string(),
        };

        let path = submit_feedback_report(
            &draft,
            serde_json::json!({"scenario": "llm_bootstrap"}),
            vec!["[stdout] launcher started".to_string()],
        )
        .expect("feedback report should be written");

        let data = std::fs::read_to_string(&path).expect("feedback report should exist");
        assert!(data.contains("\"kind\": \"suggestion\""));
        assert!(data.contains("\"scenario\": \"llm_bootstrap\""));
        assert!(data.contains("launcher started"));

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn format_filename_timestamp_uses_compact_utc_shape() {
        let timestamp = format_filename_timestamp(UNIX_EPOCH + Duration::from_secs(1_700_000_001));
        assert_eq!(timestamp.len(), 16);
        assert!(timestamp.ends_with('Z'));
        assert!(timestamp.contains('T'));
    }

    #[test]
    fn parse_host_port_accepts_bracketed_ipv6() {
        let (host, port) = parse_host_port("[::1]:5121", "chain status bind").expect("valid");
        assert_eq!(host, "::1");
        assert_eq!(port, 5121);
    }

    #[test]
    fn parse_http_json_response_reads_success_payload() {
        let raw = b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\r\n{\"ok\":true,\"feedback_id\":\"fb-1\",\"event_id\":\"ev-1\"}";
        let response = parse_http_json_response(raw).expect("parse response");
        assert!(response.ok);
        assert_eq!(response.feedback_id.as_deref(), Some("fb-1"));
        assert_eq!(response.event_id.as_deref(), Some("ev-1"));
    }

    #[test]
    fn submit_feedback_with_fallback_writes_local_on_remote_failure() {
        let temp_dir = std::env::temp_dir().join(format!(
            "oasis7-feedback-fallback-test-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&temp_dir);

        let draft = FeedbackDraft {
            kind: FeedbackKind::Bug,
            title: "viewer bug".to_string(),
            description: "viewer freeze on zoom".to_string(),
            output_dir: temp_dir.to_string_lossy().to_string(),
        };

        let outcome = submit_feedback_with_fallback(
            &draft,
            serde_json::json!({"scenario": "llm_bootstrap"}),
            vec!["[stderr] test".to_string()],
            true,
            "127.0.0.1:1",
        )
        .expect("fallback should save local report");

        match outcome {
            FeedbackSubmitResult::Distributed { .. } => {
                panic!("remote submit should fail for unavailable local port")
            }
            FeedbackSubmitResult::Local { path, remote_error } => {
                assert!(path.is_file());
                let remote_error = remote_error
                    .as_deref()
                    .expect("remote submit failure should preserve remote_error");
                assert!(remote_error.contains("connect chain status server failed"));
                assert!(
                    remote_error.to_ascii_lowercase().contains("refused"),
                    "expected connection refused signature, got: {remote_error}"
                );
            }
        }

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
