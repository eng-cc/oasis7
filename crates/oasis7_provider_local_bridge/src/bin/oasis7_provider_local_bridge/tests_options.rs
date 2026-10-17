use super::*;

#[test]
fn parse_options_rejects_short_auth_token() {
    let err = parse_options(["--auth-token", "too-short"].into_iter()).expect_err("short token");
    assert!(err.contains("at least 24 characters"));
}

#[test]
fn parse_options_rejects_auth_route_map_with_short_tokens() {
    let auth_map_path = std::env::temp_dir().join(format!(
        "oasis7-provider-bridge-auth-map-{}.json",
        std::process::id()
    ));
    fs::write(
        auth_map_path.as_path(),
        serde_json::to_vec(&serde_json::json!({
            "too-short": "alice"
        }))
        .expect("encode auth map"),
    )
    .expect("write auth map");
    let err = parse_options(
        [
            "--auth-route-map",
            auth_map_path.to_str().expect("utf8 path"),
        ]
        .into_iter(),
    )
    .expect_err("short auth route token should fail");
    assert!(err.contains("did not contain any usable entries"));
    let _ = fs::remove_file(auth_map_path);
}

#[test]
fn route_label_env_clears_label_when_absent() {
    assert_eq!(
        route_label_env(None),
        vec![("OASIS7_REMOTE_LLM_ROUTE_LABEL", String::new())]
    );
}
