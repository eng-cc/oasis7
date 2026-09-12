use super::*;

#[test]
fn parse_options_accepts_provider_lineage_store() {
    let options = parse_options(
        [
            "--provider-lineage-store",
            "/var/lib/oasis7/provider-lineage.json",
            "--no-open-browser",
        ]
        .into_iter(),
    )
    .expect("provider lineage store");
    assert_eq!(
        options.provider_lineage_store,
        "/var/lib/oasis7/provider-lineage.json"
    );
}

#[test]
fn build_viewer_live_command_wires_provider_lineage_store() {
    let mut options = CliOptions::default();
    options.provider_lineage_store = "/var/lib/oasis7/provider-lineage.json".to_string();
    let command = build_oasis7_viewer_live_command(Path::new("/bin/echo"), &options, false, false);
    let args: Vec<String> = command
        .get_args()
        .map(|arg| arg.to_string_lossy().into_owned())
        .collect();
    let flag_index = args
        .iter()
        .position(|arg| arg == "--provider-lineage-store")
        .expect("provider lineage flag");
    assert_eq!(
        args.get(flag_index + 1).map(String::as_str),
        Some("/var/lib/oasis7/provider-lineage.json")
    );
}

#[test]
fn parse_options_collects_repeat_provider_bootstrap_authority_paths() {
    let options = parse_options(
        [
            "--provider-bootstrap-authority",
            "/tmp/provider-authority-one.json",
            "--provider-bootstrap-authority",
            "output/authority bundles/provider-authority-two.json",
        ]
        .into_iter(),
    )
    .expect("provider authority paths should parse");

    assert_eq!(
        options.provider_bootstrap_authority_paths,
        vec![
            "/tmp/provider-authority-one.json".to_string(),
            "output/authority bundles/provider-authority-two.json".to_string(),
        ]
    );
}

#[test]
fn build_oasis7_chain_runtime_args_forwards_provider_authority_paths_for_enabled_chain() {
    let options = CliOptions {
        chain_enabled: true,
        provider_bootstrap_authority_paths: vec![
            "/tmp/provider-authority-one.json".to_string(),
            "output/authority bundles/provider-authority-two.json".to_string(),
        ],
        ..CliOptions::default()
    };

    let args = build_oasis7_chain_runtime_args(&options);
    let authority_args: Vec<String> = args
        .windows(2)
        .filter(|pair| pair[0] == "--provider-bootstrap-authority")
        .map(|pair| pair[1].clone())
        .collect();

    assert_eq!(
        authority_args,
        vec![
            "/tmp/provider-authority-one.json".to_string(),
            "output/authority bundles/provider-authority-two.json".to_string(),
        ]
    );
}

#[test]
fn build_oasis7_chain_runtime_args_omits_provider_authority_paths_when_chain_disabled() {
    let options = CliOptions {
        chain_enabled: false,
        provider_bootstrap_authority_paths: vec!["/tmp/provider-authority.json".to_string()],
        ..CliOptions::default()
    };

    let args = build_oasis7_chain_runtime_args(&options);
    assert!(
        !args
            .iter()
            .any(|arg| arg == "--provider-bootstrap-authority")
    );
    assert!(!args.iter().any(|arg| arg == "/tmp/provider-authority.json"));
}
