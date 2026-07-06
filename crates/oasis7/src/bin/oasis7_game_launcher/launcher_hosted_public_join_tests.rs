use std::path::Path;

use super::{build_oasis7_viewer_live_command, parse_options};

#[test]
fn build_viewer_live_command_keeps_chain_status_bind_for_hosted_public_join() {
    let options = parse_options(
        [
            "--deployment-mode",
            "hosted_public_join",
            "--chain-status-bind",
            "39.104.204.172:6631",
            "--chain-link-policy",
            "enforcing",
        ]
        .into_iter(),
    )
    .expect("hosted public join should parse");
    assert!(!options.chain_enabled);

    let command = build_oasis7_viewer_live_command(Path::new("/bin/echo"), &options, false, false);
    let args: Vec<String> = command
        .get_args()
        .map(|arg| arg.to_string_lossy().into_owned())
        .collect();

    assert!(args.contains(&"--deployment-mode".to_string()));
    assert!(args.contains(&"hosted_public_join".to_string()));
    assert!(args.contains(&"--chain-status-bind".to_string()));
    assert!(args.contains(&"39.104.204.172:6631".to_string()));
    assert!(args.contains(&"--chain-link-policy".to_string()));
    assert!(args.contains(&"enforcing".to_string()));
}
