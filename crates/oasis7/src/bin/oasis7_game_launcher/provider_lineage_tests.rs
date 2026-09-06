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
