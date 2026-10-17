//! Child-startup environment scenarios for launcher tests; no process-global writes.
use std::ffi::{OsStr, OsString};
use std::process::Command;
pub const ISSUER: &str = "OASIS7_HOSTED_REGISTRATION_ISSUER_PRIVATE_KEY";
pub const LOGIN: &str = "OASIS7_HOSTED_TEST_LOGIN_ENABLED";
const CHILD_MARKER: &str = "OASIS7_LAUNCHER_TEST_ENV_SCENARIO";
const CHILD_ARGUMENT: &str = "__oasis7_child_environment__";
const TRACKED: [&str; 7] = [
    ISSUER,
    LOGIN,
    "OASIS7_HOSTED_STRONG_AUTH_PUBLIC_KEY",
    "OASIS7_HOSTED_STRONG_AUTH_PRIVATE_KEY",
    "OASIS7_HOSTED_STRONG_AUTH_APPROVAL_CODE",
    "OASIS7_RUNTIME_AGENT_CHAT_ECHO",
    CHILD_MARKER,
];
type Scenario<'a> = (&'a str, &'a [(&'a str, &'a OsStr)]);
/// Configure each exact test child before startup and preserve parent values.
/// A private --skip argument distinguishes children from inherited markers.
pub fn run_scenarios(scenarios: &[Scenario<'_>]) -> Option<usize> {
    assert!(!scenarios.is_empty());
    let thread = std::thread::current();
    let test = thread.name().expect("named libtest scenario thread");
    for (index, (name, values)) in scenarios.iter().enumerate() {
        assert!(!scenarios[..index].iter().any(|(prior, _)| prior == name));
        for (offset, (key, _)) in values.iter().enumerate() {
            assert!(TRACKED.contains(key) && *key != CHILD_MARKER);
            assert!(!values[..offset].iter().any(|(prior, _)| prior == key));
        }
    }
    let args: Vec<_> = std::env::args_os().collect();
    let is_child = args
        .windows(2)
        .any(|pair| pair[0] == "--skip" && pair[1] == CHILD_ARGUMENT);
    if is_child {
        let marker = std::env::var_os(CHILD_MARKER).expect("child scenario marker missing");
        let index = scenarios
            .iter()
            .position(|(name, _)| marker == OsString::from(format!("{test}:{name}")))
            .expect("unknown child scenario marker; refusing recursive spawn");
        for key in TRACKED.into_iter().filter(|key| *key != CHILD_MARKER) {
            let expected = scenarios[index]
                .1
                .iter()
                .find_map(|(name, value)| (*name == key).then_some(*value));
            assert_eq!(
                std::env::var_os(key).as_deref(),
                expected,
                "child startup {key}"
            );
        }
        return Some(index);
    }
    for (name, values) in scenarios {
        let before: Vec<_> = TRACKED.iter().map(std::env::var_os).collect();
        let mut command = Command::new(std::env::current_exe().expect("current test executable"));
        command.args(["--exact", test, "--nocapture", "--skip", CHILD_ARGUMENT]);
        for key in TRACKED {
            command.env_remove(key);
        }
        command.env(CHILD_MARKER, format!("{test}:{name}"));
        for (key, value) in *values {
            command.env(key, value);
        }
        let output = command.output().expect("spawn isolated test scenario");
        let after: Vec<_> = TRACKED.iter().map(std::env::var_os).collect();
        assert_eq!(
            after, before,
            "parent environment changed after {test}:{name}"
        );
        assert!(
            output.status.success(),
            "child {test}:{name} failed: {}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            String::from_utf8_lossy(&output.stdout).contains("running 1 test"),
            "exact scenario must execute one test: {}",
            String::from_utf8_lossy(&output.stdout)
        );
    }
    None
}
pub fn run(values: &[(&str, &OsStr)]) -> bool {
    run_scenarios(&[("isolated", values)]).is_some()
}
pub fn with_issuer(seed: Option<u8>) -> bool {
    match seed {
        Some(seed) => run(&[(ISSUER, OsStr::new(&hex::encode([seed; 32])))]),
        None => run(&[]),
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn child_startup_preserves_absent_and_present_parent_values() {
        if !with_issuer(Some(71)) {
            return;
        }
        assert_eq!(
            std::env::var_os(ISSUER),
            Some(OsString::from(hex::encode([71; 32])))
        );
        assert_eq!(std::env::var_os(LOGIN), None);
    }
    #[cfg(unix)]
    #[test]
    fn child_startup_preserves_non_unicode_value() {
        use std::os::unix::ffi::OsStringExt;
        let prior = OsString::from_vec(vec![0xff, b'x']);
        if !run(&[(ISSUER, &prior)]) {
            return;
        }
        assert_eq!(std::env::var_os(ISSUER), Some(prior));
    }
}
