mod support;
#[test]
fn standalone_ordinary_user_contract() {
    support::ordinary_contract(env!("CARGO_BIN_EXE_oasis7_context"), &[]);
}

#[test]
fn snapshot_roundtrip_without_configured_store() {
    let h = support::Harness::new(env!("CARGO_BIN_EXE_oasis7_context"), &[]);
    let identity = h.init()["store_id"].clone();
    let file = h.candidate("machine-test", "Fixture only\n");
    h.put("machine-test", &file, None);
    let encrypted = h.temp.path().join("snapshot.enc");
    let target = h.temp.path().join("restored");
    let pass = b"fixture-only-passphrase\n";
    let out = h.invoke(
        &[
            "snapshot",
            "export",
            "--root",
            h.root.to_str().unwrap(),
            "--output",
            encrypted.to_str().unwrap(),
            "--passphrase-stdin",
            "--json",
        ],
        Some(pass),
    );
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stdout)
    );
    let inspect = h.invoke(
        &[
            "snapshot",
            "inspect",
            "--input",
            encrypted.to_str().unwrap(),
            "--passphrase-stdin",
            "--json",
        ],
        Some(pass),
    );
    assert!(inspect.status.success());
    let summary: serde_json::Value = serde_json::from_slice(&inspect.stdout).unwrap();
    assert_eq!(summary["store_id"], identity);
    assert!(!String::from_utf8_lossy(&inspect.stdout).contains("Fixture only"));
    let restored = h.invoke(
        &[
            "snapshot",
            "restore",
            "--input",
            encrypted.to_str().unwrap(),
            "--to",
            target.to_str().unwrap(),
            "--expected-store-id",
            identity.as_str().unwrap(),
            "--passphrase-stdin",
            "--json",
        ],
        Some(pass),
    );
    assert!(
        restored.status.success(),
        "{}",
        String::from_utf8_lossy(&restored.stdout)
    );
    assert_eq!(
        std::fs::read(target.join("entries/machine-test.md")).unwrap(),
        std::fs::read(h.root.join("entries/machine-test.md")).unwrap()
    );
    assert!(!target.join(".git").exists());
}

#[test]
fn editor_conflict_retains_private_draft() {
    let h = support::Harness::new(env!("CARGO_BIN_EXE_oasis7_context"), &[]);
    h.init();
    let file = h.candidate("machine-test", "Original\n");
    h.put("machine-test", &file, None);
    let revised = h.candidate("machine-test", "Other writer\n");
    let revision = h.call(
        &[
            "get",
            "machine-test",
            "--root",
            h.root.to_str().unwrap(),
            "--json",
        ],
        0,
    )["data"]["revision"]
        .as_str()
        .unwrap()
        .to_owned();
    let script = h.temp.path().join("editor.sh");
    std::fs::write(&script,"#!/bin/sh\n\"$1\" put machine-test --root \"$2\" --file \"$3\" --expected-revision \"$4\" --json >/dev/null || exit 1\nprintf '\\nHuman draft\\n' >> \"$5\"\n").unwrap();
    let config = h.temp.path().join("config/oasis7");
    std::fs::create_dir_all(&config).unwrap();
    let local = serde_json::json!({"schema_version":"oasis7.context_local_config.v1","editor":{"program":"/bin/sh","args":[script,h.binary,h.root,revised,revision]}});
    std::fs::write(
        config.join("context.json"),
        serde_json::to_vec(&local).unwrap(),
    )
    .unwrap();
    let out = h.call(
        &[
            "edit",
            "machine-test",
            "--root",
            h.root.to_str().unwrap(),
            "--json",
        ],
        4,
    );
    let draft = out["error"]["draft_path"].as_str().unwrap();
    assert!(
        std::fs::read_to_string(draft)
            .unwrap()
            .contains("Human draft")
    );
    let got = h.call(
        &[
            "get",
            "machine-test",
            "--root",
            h.root.to_str().unwrap(),
            "--json",
        ],
        0,
    );
    assert!(
        got["data"]["raw_text"]
            .as_str()
            .unwrap()
            .contains("Other writer")
    );
    assert!(
        !got["data"]["raw_text"]
            .as_str()
            .unwrap()
            .contains("Human draft")
    );
}

#[test]
fn info_is_read_only_and_does_not_require_a_store() {
    let h = support::Harness::new(env!("CARGO_BIN_EXE_oasis7_context"), &[]);
    let out = h.invoke(&["info", "--json"], None);
    assert!(out.status.success());
    let value: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(value["schema_version"], 1);
    assert_eq!(value["tool_id"], "oasis7_context");
    assert!(
        value["capabilities"]
            .as_array()
            .unwrap()
            .iter()
            .any(|c| c["id"] == "context.write" && c["supported"] == true)
    );
    assert!(!h.root.exists());
}
