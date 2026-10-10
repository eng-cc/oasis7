//! Explicit, read-only GitHub access using the user's existing gh credential store.
use crate::{Error, Result, process};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct GitHubSnapshot {
    pub state: String,
    pub repository: Option<String>,
    pub account: Option<String>,
    pub observed_at: Option<String>,
    pub last_attempt: Option<String>,
    pub coverage: String,
    pub error: Option<String>,
    pub pulls: Vec<PullRequest>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct PullRequest {
    pub id: String,
    pub title: String,
    pub url: String,
    pub state: String,
    pub head_oid: String,
    pub review_decision: Option<String>,
    pub checks: Vec<Check>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct Check {
    pub name: String,
    pub state: String,
    pub tested_oid: Option<String>,
    pub url: Option<String>,
}

pub struct GitHubClient {
    current: GitHubSnapshot,
}

impl Default for GitHubClient {
    fn default() -> Self {
        Self::new()
    }
}

impl GitHubClient {
    pub fn new() -> Self {
        Self {
            current: GitHubSnapshot {
                state: "unconnected".into(),
                repository: None,
                account: None,
                observed_at: None,
                last_attempt: None,
                coverage: "未接入 GitHub".into(),
                error: None,
                pulls: vec![],
            },
        }
    }
    pub fn snapshot(&self) -> GitHubSnapshot {
        self.current.clone()
    }
    pub fn restore(mut snapshot: GitHubSnapshot) -> Self {
        if snapshot.observed_at.is_some() {
            snapshot.state = "stale".into();
        }
        Self { current: snapshot }
    }
    /// A project switch must never display another repository's retained facts.
    pub fn reset(&mut self) {
        *self = Self::new();
    }
    pub fn refresh(&mut self, root: &Path, gh_path: Option<&str>) -> GitHubSnapshot {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
            .to_string();
        self.current.last_attempt = Some(now.clone());
        let result = read(root, gh_path);
        match result {
            Ok(mut candidate) => {
                candidate.observed_at = Some(now.clone());
                candidate.last_attempt = Some(now);
                self.current = candidate;
            }
            Err(_) => {
                self.current.state = "error".into();
                // Never relay CLI stderr: authentication/proxy diagnostics may contain secrets.
                self.current.error = Some("GitHub 读取失败；请检查 gh 路径、登录、仓库权限或网络。保留最近成功读取的事实。".into());
            }
        }
        self.snapshot()
    }
}

fn json(program: &str, args: &[&str], root: &Path) -> Result<Value> {
    Ok(serde_json::from_slice(&process::run(program, args, root)?)?)
}

fn read(root: &Path, explicit: Option<&str>) -> Result<GitHubSnapshot> {
    let path = if let Some(path) = explicit {
        if !Path::new(path).is_absolute() || !Path::new(path).is_file() {
            return Err(Error::Invalid(
                "gh path must be an absolute file path".into(),
            ));
        }
        path.to_owned()
    } else {
        ["/opt/homebrew/bin/gh", "/usr/local/bin/gh", "/usr/bin/gh"]
            .into_iter()
            .find(|p| Path::new(p).is_file())
            .ok_or_else(|| Error::Invalid("gh is not installed; select its absolute path".into()))?
            .to_owned()
    };
    let repo = json(
        &path,
        &["repo", "view", "--json", "nameWithOwner,url"],
        root,
    )?;
    let repository = repo["nameWithOwner"]
        .as_str()
        .ok_or_else(|| Error::Invalid("missing repository identity".into()))?
        .to_owned();
    let repository_url = repo["url"]
        .as_str()
        .filter(|url| *url == format!("https://github.com/{repository}"))
        .ok_or_else(|| {
            Error::Invalid("this connector currently supports github.com only".into())
        })?;
    let account = json(&path, &["api", "--hostname", "github.com", "user"], root)?["login"]
        .as_str()
        .map(str::to_owned);
    let raw = json(
        &path,
        &[
            "pr",
            "list",
            "--repo",
            repository_url,
            "--state",
            "all",
            "--limit",
            "100",
            "--json",
            "number,title,url,state,headRefOid,reviewDecision,statusCheckRollup",
        ],
        root,
    )?;
    let pulls = decode_pulls(&raw)?;
    // The bounded recent window is intentionally partial; it is not a complete repository inventory.
    Ok(GitHubSnapshot {
        state: if pulls.is_empty() { "empty" } else { "partial" }.into(),
        repository: Some(repository),
        account,
        observed_at: None,
        last_attempt: None,
        coverage:
            "最近最多 100 个 PR；含 gh 返回的检查汇总，非全仓库快照。检查测试版本未取得时保持未知。"
                .into(),
        error: None,
        pulls,
    })
}

fn field(value: &Value, key: &str) -> Result<String> {
    value[key]
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| Error::Invalid(format!("missing GitHub field {key}")))
}

pub fn decode_pulls(raw: &Value) -> Result<Vec<PullRequest>> {
    raw.as_array()
        .ok_or_else(|| Error::Invalid("invalid GitHub PR list".into()))?
        .iter()
        .map(|pr| {
            let id = pr["number"]
                .as_u64()
                .ok_or_else(|| Error::Invalid("missing PR number".into()))?
                .to_string();
            let checks = pr["statusCheckRollup"]
                .as_array()
                .map(|list| {
                    list.iter()
                        .map(|check| Check {
                            name: check["name"]
                                .as_str()
                                .or(check["context"].as_str())
                                .unwrap_or("未命名检查")
                                .into(),
                            state: check["conclusion"]
                                .as_str()
                                .filter(|s| !s.is_empty())
                                .or(check["status"].as_str())
                                .or(check["state"].as_str())
                                .unwrap_or("UNKNOWN")
                                .into(),
                            // A PR head is not proof of a check's tested commit.
                            tested_oid: None,
                            url: check["detailsUrl"]
                                .as_str()
                                .or(check["targetUrl"].as_str())
                                .map(str::to_owned),
                        })
                        .collect()
                })
                .unwrap_or_default();
            Ok(PullRequest {
                id,
                title: field(pr, "title")?,
                url: field(pr, "url")?,
                state: field(pr, "state")?,
                head_oid: field(pr, "headRefOid")?,
                review_decision: pr["reviewDecision"]
                    .as_str()
                    .filter(|s| !s.is_empty())
                    .map(str::to_owned),
                checks,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(unix)]
    #[test]
    fn read_only_cli_window_and_offline_restore() {
        use std::os::unix::fs::PermissionsExt;
        let tmp = tempfile::tempdir().unwrap();
        let program = tmp.path().join("gh-fixture");
        let calls = tmp.path().join("calls");
        // The fixture checks fixed read-only command selection, then simulates disconnect.
        std::fs::write(&program, format!(r##"#!/bin/sh
echo "$1" >> '{}'
case "$1" in
repo) echo '{{"nameWithOwner":"o/r","url":"https://github.com/o/r"}}' ;;
api) echo '{{"login":"reader"}}' ;;
pr) echo '[{{"number":1,"title":"purpose","url":"https://github.com/o/r/pull/1","state":"OPEN","headRefOid":"abc","reviewDecision":null,"statusCheckRollup":[]}}]' ;;
*) exit 2 ;;
esac
"##, calls.display())).unwrap();
        std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o700)).unwrap();
        let mut client = GitHubClient::new();
        let read = client.refresh(tmp.path(), program.to_str());
        assert_eq!(read.state, "partial");
        assert_eq!(read.account.as_deref(), Some("reader"));
        assert_eq!(read.pulls.len(), 1);
        assert_eq!(std::fs::read_to_string(calls).unwrap(), "repo\napi\npr\n");
        let mut restored = GitHubClient::restore(read);
        assert_eq!(restored.snapshot().state, "stale");
        std::fs::remove_file(&program).unwrap();
        let failed = restored.refresh(tmp.path(), program.to_str());
        assert_eq!(failed.state, "error");
        assert_eq!(failed.pulls.len(), 1);
        assert!(failed.observed_at.is_some());
    }
    #[test]
    fn check_head_is_not_assumed_tested_commit() {
        let rows = decode_pulls(&serde_json::json!([{"number":42,"title":"work without issue","url":"https://github.com/o/r/pull/42","state":"OPEN","headRefOid":"abc","statusCheckRollup":[{"name":"test","conclusion":"SUCCESS"}]}])).unwrap();
        assert_eq!(rows[0].checks[0].tested_oid, None);
        assert_eq!(rows[0].checks[0].state, "SUCCESS");
    }
    #[test]
    fn failure_retains_observation_and_reset_clears_other_project() {
        let mut client = GitHubClient::new();
        client.current.repository = Some("one/repo".into());
        client.current.observed_at = Some("123".into());
        let failed = client.refresh(Path::new("/"), Some("/not/a/program"));
        assert_eq!(failed.state, "error");
        assert_eq!(failed.observed_at.as_deref(), Some("123"));
        assert_eq!(failed.repository.as_deref(), Some("one/repo"));
        client.reset();
        assert_eq!(client.snapshot().state, "unconnected");
        assert!(client.snapshot().repository.is_none());
    }
    #[test]
    fn malformed_is_not_successful_empty() {
        assert!(decode_pulls(&serde_json::json!({"error":"rate limit"})).is_err());
        assert!(decode_pulls(&serde_json::json!([])).unwrap().is_empty());
    }
}
