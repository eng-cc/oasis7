#![allow(dead_code)] // Shared contract helpers are used by several separate test binaries.
use serde_json::{Value, json};
use std::{
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};
pub struct Harness {
    pub binary: PathBuf,
    pub prefix: Vec<String>,
    pub temp: tempfile::TempDir,
    pub root: PathBuf,
}
impl Harness {
    pub fn new(binary: &str, prefix: &[&str]) -> Self {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("private-store");
        Self {
            binary: binary.into(),
            prefix: prefix.iter().map(|s| s.to_string()).collect(),
            temp,
            root,
        }
    }
    pub fn invoke(&self, args: &[&str], input: Option<&[u8]>) -> std::process::Output {
        let mut c = Command::new(&self.binary);
        c.args(&self.prefix)
            .args(args)
            .env("XDG_CONFIG_HOME", self.temp.path().join("config"))
            .env_remove("OASIS7_CONTEXT_DIR")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let mut child = c.spawn().unwrap();
        if let Some(input) = input {
            child.stdin.take().unwrap().write_all(input).unwrap();
        } else {
            drop(child.stdin.take());
        }
        child.wait_with_output().unwrap()
    }
    pub fn call(&self, args: &[&str], code: i32) -> Value {
        let out = self.invoke(args, None);
        assert_eq!(
            out.status.code(),
            Some(code),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        let v: Value = serde_json::from_slice(&out.stdout)
            .unwrap_or_else(|_| panic!("invalid JSON: {}", String::from_utf8_lossy(&out.stdout)));
        assert_eq!(v["schema_version"], "oasis7.context_result.v1");
        v
    }
    pub fn init(&self) -> Value {
        self.call(
            &[
                "init",
                "--root",
                self.root.to_str().unwrap(),
                "--project",
                "fixture",
                "--json",
            ],
            0,
        )
    }
    pub fn candidate(&self, id: &str, body: &str) -> PathBuf {
        let value = json!({"schema_version":"oasis7.context_entry.v1","project_id":"fixture","id":id,"kind":"machine","title":"虚构编译机","environment":"development","owner":"test:owner","tags":["build"],"state":"active","updated_at":"2026-01-01T00:00:00Z","updated_by":"fixture","verification":null,"credential_refs":[{"id":"ssh","provider":"external","locator":"fixture/ssh","usage":"test reference"}],"links":[]});
        let p = self.temp.path().join(format!("{id}-candidate.md"));
        std::fs::write(
            &p,
            format!(
                "---\n{}\n---\n{body}",
                serde_json::to_string_pretty(&value).unwrap()
            ),
        )
        .unwrap();
        p
    }
    pub fn put(&self, id: &str, file: &Path, rev: Option<&str>) -> Value {
        let mut args = vec![
            "put",
            id,
            "--root",
            self.root.to_str().unwrap(),
            "--file",
            file.to_str().unwrap(),
            "--json",
        ];
        if let Some(rev) = rev {
            args.extend(["--expected-revision", rev]);
        } else {
            args.push("--if-absent");
        }
        self.call(&args, 0)
    }
}
pub fn ordinary_contract(binary: &str, prefix: &[&str]) {
    let h = Harness::new(binary, prefix);
    let root = h.root.to_str().unwrap();
    let initial = h.init();
    let identity = initial["store_id"].clone();
    assert_eq!(h.init()["store_id"], identity);
    let candidate = h.candidate("machine-test", "# Purpose\nFixture 测试 machine\n");
    let created = h.put("machine-test", &candidate, None);
    let revision = created["data"]["revision"].as_str().unwrap();
    let got = h.call(&["--root", root, "get", "machine-test", "--json"], 0);
    assert_eq!(got["data"]["revision"], revision);
    assert!(
        got["data"]["raw_text"]
            .as_str()
            .unwrap()
            .contains("Fixture 测试")
    );
    assert_eq!(got["data"]["verification_state"], "unverified");
    assert_eq!(
        h.put("machine-test", &candidate, Some(revision))["data"]["changed"],
        false
    );
    let missing = h.call(
        &[
            "put",
            "machine-test",
            "--root",
            root,
            "--file",
            candidate.to_str().unwrap(),
            "--json",
        ],
        2,
    );
    assert_eq!(missing["error"]["effect"], "none");
    let stale = format!("sha256:{}", "0".repeat(64));
    assert_eq!(
        h.call(
            &[
                "put",
                "machine-test",
                "--root",
                root,
                "--file",
                candidate.to_str().unwrap(),
                "--expected-revision",
                &stale,
                "--json"
            ],
            4
        )["error"]["current_revision"],
        revision
    );
    let list = h.call(&["search", "fixture 测试", "--root", root, "--json"], 0);
    assert_eq!(list["data"]["total_matches"], 1);
    let verified = h.call(
        &[
            "verify",
            "machine-test",
            "--root",
            root,
            "--expected-revision",
            revision,
            "--method",
            "observation",
            "--evidence-ref",
            "fixture:check",
            "--json",
        ],
        0,
    );
    let revision = verified["data"]["revision"].as_str().unwrap();
    assert_eq!(
        h.call(&["get", "machine-test", "--root", root, "--json"], 0)["data"]["verification_state"],
        "current"
    );
    h.call(
        &[
            "archive",
            "machine-test",
            "--root",
            root,
            "--expected-revision",
            revision,
            "--json",
        ],
        0,
    );
    assert_eq!(
        h.call(&["list", "--root", root, "--json"], 0)["data"]["total_matches"],
        0
    );
    assert_eq!(
        h.call(&["list", "--root", root, "--include-archived", "--json"], 0)["data"]["total_matches"],
        1
    );
    h.call(&["nonsense", "--root", root, "--json"], 2);
    h.call(&["get", "../outside", "--root", root, "--json"], 2);
    let raw = h.invoke(
        &[
            "get",
            "machine-test",
            "--root",
            root,
            "--format",
            "markdown",
        ],
        None,
    );
    assert!(raw.status.success());
    assert!(raw.stdout.starts_with(b"---\n"));
}
