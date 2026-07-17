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

#[test]
fn build_viewer_live_command_derives_trusted_registration_issuer_key() {
    let options = parse_options(["--deployment-mode", "hosted_public_join"].into_iter())
        .expect("hosted public join should parse");
    unsafe {
        std::env::set_var(
            oasis7::viewer::HOSTED_REGISTRATION_ISSUER_PRIVATE_KEY_ENV,
            hex::encode([81_u8; 32]),
        );
    }

    let command = build_oasis7_viewer_live_command(Path::new("/bin/echo"), &options, false, false);
    let trusted_public_key = command
        .get_envs()
        .find_map(|(name, value)| {
            (name == oasis7::viewer::HOSTED_REGISTRATION_ISSUER_PUBLIC_KEY_ENV)
                .then(|| value.map(|value| value.to_string_lossy().into_owned()))
                .flatten()
        })
        .expect("trusted issuer public key env");

    unsafe {
        std::env::remove_var(oasis7::viewer::HOSTED_REGISTRATION_ISSUER_PRIVATE_KEY_ENV);
    }
    assert_eq!(trusted_public_key.len(), 64);
}
