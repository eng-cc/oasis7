use std::env;
use std::process;
use std::thread;

use oasis7::observability::init_tracing;
use oasis7::simulator::WorldScenario;
use oasis7::viewer::{
    ViewerLiveDecisionMode, ViewerRuntimeLiveServer, ViewerRuntimeLiveServerConfig,
    ViewerWebBridge, ViewerWebBridgeConfig,
};
use tracing::{error, info, warn};

const DEFAULT_SCENARIO: &str = "llm_bootstrap";
const DEFAULT_BIND: &str = "127.0.0.1:5023";
const DEFAULT_WEB_BIND: &str = "127.0.0.1:5011";
const DEFAULT_DEPLOYMENT_MODE: &str = "trusted_local_only";
const REMOVAL_HINT: &str =
    "embedded node flags were removed from oasis7_viewer_live; use oasis7_chain_runtime (normally launched by oasis7_game_launcher)";
const RUNTIME_ALIAS_REMOVAL_HINT: &str =
    "`--runtime-world` was removed; oasis7_viewer_live is runtime/world only, start without this flag";

#[derive(Debug, Clone, PartialEq, Eq)]
struct CliOptions {
    scenario: WorldScenario,
    bind_addr: String,
    web_bind_addr: Option<String>,
    llm_mode: bool,
    deployment_mode: String,
    chain_status_bind: Option<String>,
}

impl Default for CliOptions {
    fn default() -> Self {
        Self {
            scenario: WorldScenario::LlmBootstrap,
            bind_addr: DEFAULT_BIND.to_string(),
            web_bind_addr: Some(DEFAULT_WEB_BIND.to_string()),
            llm_mode: true,
            deployment_mode: DEFAULT_DEPLOYMENT_MODE.to_string(),
            chain_status_bind: None,
        }
    }
}

fn main() {
    init_tracing("oasis7_viewer_live");
    let raw_args: Vec<String> = env::args().skip(1).collect();
    if raw_args.iter().any(|arg| arg == "--help" || arg == "-h") {
        print_help();
        return;
    }

    let options = match parse_options(raw_args.iter().map(|arg| arg.as_str())) {
        Ok(options) => options,
        Err(err) => {
            error!(error = %err, "failed to parse viewer live options");
            print_help();
            process::exit(1);
        }
    };

    if let Err(err) = run_viewer(options) {
        error!(error = %err, "oasis7_viewer_live failed");
        process::exit(1);
    }
}

fn run_viewer(options: CliOptions) -> Result<(), String> {
    let trace_session_id = oasis7::observability::resolve_trace_session_id("oasis7_viewer_live");
    info!(
        trace_session_id = %trace_session_id,
        bind_addr = %options.bind_addr,
        web_bind_addr = ?options.web_bind_addr,
        llm_mode = options.llm_mode,
        deployment_mode = %options.deployment_mode,
        chain_status_bind = ?options.chain_status_bind,
        scenario = %options.scenario.as_str(),
        "starting viewer live runtime"
    );
    if let Some(web_bind_addr) = options.web_bind_addr.clone() {
        let upstream_addr = options.bind_addr.clone();
        thread::spawn(move || {
            let bridge = ViewerWebBridge::new(ViewerWebBridgeConfig::new(
                web_bind_addr.clone(),
                upstream_addr,
            ));
            if let Err(err) = bridge.run() {
                warn!(bind_addr = %web_bind_addr, error = ?err, "viewer web bridge exited with error");
            }
        });
    }

    let config = ViewerRuntimeLiveServerConfig::new(options.scenario)
        .with_bind_addr(options.bind_addr)
        .with_hosted_public_join_mode(options.deployment_mode == "hosted_public_join")
        .with_decision_mode(if options.llm_mode {
            ViewerLiveDecisionMode::Llm
        } else {
            ViewerLiveDecisionMode::Script
        });
    let config = if let Some(chain_status_bind) = options.chain_status_bind {
        config.with_chain_status_bind(chain_status_bind)
    } else {
        config
    };
    let server = ViewerRuntimeLiveServer::new(config)
        .map_err(|err| format!("failed to create runtime viewer server: {err:?}"))?;
    server
        .run()
        .map_err(|err| format!("runtime viewer server exited with error: {err:?}"))
}

fn parse_options<'a>(args: impl Iterator<Item = &'a str>) -> Result<CliOptions, String> {
    let mut options = CliOptions::default();
    let mut iter = args.peekable();
    let mut scenario_set = false;

    while let Some(arg) = iter.next() {
        if !arg.starts_with('-') {
            if scenario_set {
                return Err(format!("unexpected positional argument `{arg}`"));
            }
            options.scenario = parse_world_scenario(arg)?;
            scenario_set = true;
            continue;
        }

        match arg {
            "--bind" => {
                options.bind_addr = parse_required_value(&mut iter, "--bind")?;
            }
            "--web-bind" => {
                options.web_bind_addr = Some(parse_required_value(&mut iter, "--web-bind")?);
            }
            "--no-web-bind" => {
                options.web_bind_addr = None;
            }
            "--llm" => {
                options.llm_mode = true;
            }
            "--no-llm" => {
                options.llm_mode = false;
            }
            "--deployment-mode" => {
                let raw = parse_required_value(&mut iter, "--deployment-mode")?;
                options.deployment_mode = parse_deployment_mode(raw.as_str())?.to_string();
            }
            "--chain-status-bind" => {
                options.chain_status_bind =
                    Some(parse_required_value(&mut iter, "--chain-status-bind")?);
            }
            "--runtime-world" => {
                return Err(RUNTIME_ALIAS_REMOVAL_HINT.to_string());
            }
            "--no-runtime-world" => {
                return Err(
                    "`--no-runtime-world` is no longer supported: oasis7_viewer_live is runtime-only"
                        .to_string(),
                );
            }
            "--release-config" => {
                return Err(format!("`{arg}` is no longer supported: {REMOVAL_HINT}"));
            }
            "--topology" | "--no-node" | "--viewer-no-consensus-gate" => {
                return Err(format!("`{arg}` is no longer supported: {REMOVAL_HINT}"));
            }
            _ if arg.starts_with("--node-")
                || arg.starts_with("--triad-")
                || arg.starts_with("--reward-runtime-") =>
            {
                return Err(format!("`{arg}` is no longer supported: {REMOVAL_HINT}"));
            }
            _ => {
                return Err(format!("unknown option: {arg}"));
            }
        }
    }

    parse_socket_addr(options.bind_addr.as_str(), "--bind")?;
    if let Some(web_bind_addr) = options.web_bind_addr.as_deref() {
        parse_socket_addr(web_bind_addr, "--web-bind")?;
    }
    if let Some(chain_status_bind) = options.chain_status_bind.as_deref() {
        parse_socket_addr(chain_status_bind, "--chain-status-bind")?;
    }
    let _ = parse_deployment_mode(options.deployment_mode.as_str())?;

    Ok(options)
}

fn parse_deployment_mode(raw: &str) -> Result<&'static str, String> {
    match raw.trim() {
        "trusted_local_only" => Ok("trusted_local_only"),
        "hosted_public_join" => Ok("hosted_public_join"),
        _ => Err(format!(
            "--deployment-mode must be one of trusted_local_only|hosted_public_join, got `{}`",
            raw.trim()
        )),
    }
}

fn parse_required_value<'a, I>(
    iter: &mut std::iter::Peekable<I>,
    flag: &str,
) -> Result<String, String>
where
    I: Iterator<Item = &'a str>,
{
    let Some(value) = iter.next() else {
        return Err(format!("{flag} requires a value"));
    };
    let value = value.trim();
    if value.is_empty() {
        return Err(format!("{flag} requires a non-empty value"));
    }
    Ok(value.to_string())
}

fn parse_socket_addr(raw: &str, label: &str) -> Result<std::net::SocketAddr, String> {
    raw.parse::<std::net::SocketAddr>()
        .map_err(|_| format!("{label} must be in <host:port> format"))
}

fn parse_world_scenario(raw: &str) -> Result<WorldScenario, String> {
    let normalized = raw.trim();
    if normalized.is_empty() {
        return Err("scenario cannot be empty".to_string());
    }
    WorldScenario::parse(normalized).ok_or_else(|| {
        format!(
            "unknown scenario `{normalized}`; supported: {}",
            WorldScenario::variants().join(", ")
        )
    })
}

fn print_help() {
    println!(
        "Usage: oasis7_viewer_live [scenario] [options]\n\n\
Starts pure viewer live server (no embedded chain/node runtime).\n\n\
Options:\n\
  [scenario]                world scenario (default: {DEFAULT_SCENARIO})\n\
  --bind <host:port>        viewer live server bind (default: {DEFAULT_BIND})\n\
  --web-bind <host:port>    websocket bridge bind (default: {DEFAULT_WEB_BIND})\n\
  --no-web-bind             disable websocket bridge\n\
  --llm                     enable llm mode (default; required for gameplay)\n\
  --no-llm                  disable llm mode (observer/debug only; gameplay blocked)\n\
  --chain-status-bind <addr> follow committed chain world from oasis7_chain_runtime status bind\n\
  --deployment-mode <mode>  trusted_local_only|hosted_public_join (default: {DEFAULT_DEPLOYMENT_MODE})\n\
  -h, --help                show help\n\n\
Removed:\n\
  --release-config, --runtime-world, all --node-*, --topology, --triad-*, --reward-runtime-*, --no-node, --viewer-no-consensus-gate\n\
  -> use oasis7_chain_runtime (usually managed by oasis7_game_launcher)"
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_options_defaults() {
        let options = parse_options(std::iter::empty()).expect("defaults");
        assert_eq!(options.scenario, WorldScenario::LlmBootstrap);
        assert_eq!(options.bind_addr, DEFAULT_BIND);
        assert_eq!(options.web_bind_addr.as_deref(), Some(DEFAULT_WEB_BIND));
        assert!(options.llm_mode);
        assert_eq!(options.deployment_mode, DEFAULT_DEPLOYMENT_MODE);
        assert_eq!(options.chain_status_bind, None);
    }

    #[test]
    fn parse_options_reads_custom_values() {
        let options = parse_options(
            [
                "asteroid_fragment",
                "--bind",
                "127.0.0.1:6200",
                "--web-bind",
                "127.0.0.1:6300",
                "--llm",
                "--chain-status-bind",
                "127.0.0.1:7123",
                "--deployment-mode",
                "hosted_public_join",
            ]
            .into_iter(),
        )
        .expect("custom values");
        assert_eq!(options.scenario, WorldScenario::AsteroidFragmentBootstrap);
        assert_eq!(options.bind_addr, "127.0.0.1:6200");
        assert_eq!(options.web_bind_addr.as_deref(), Some("127.0.0.1:6300"));
        assert!(options.llm_mode);
        assert_eq!(options.deployment_mode, "hosted_public_join");
        assert_eq!(options.chain_status_bind.as_deref(), Some("127.0.0.1:7123"));
    }

    #[test]
    fn parse_options_supports_no_web_bind() {
        let options = parse_options(["--no-web-bind"].into_iter()).expect("no web bind");
        assert_eq!(options.web_bind_addr, None);
    }

    #[test]
    fn parse_options_rejects_invalid_bind() {
        let err = parse_options(["--bind", "bad-bind"].into_iter()).expect_err("invalid bind");
        assert!(err.contains("--bind"));
    }

    #[test]
    fn parse_options_rejects_invalid_deployment_mode() {
        let err = parse_options(["--deployment-mode", "invalid"].into_iter())
            .expect_err("invalid deployment mode");
        assert!(err.contains("--deployment-mode"));
    }

    #[test]
    fn parse_options_rejects_legacy_node_flags() {
        let err = parse_options(["--no-node"].into_iter()).expect_err("legacy flag should fail");
        assert!(err.contains("no longer supported"));
        assert!(err.contains("oasis7_chain_runtime"));
    }

    #[test]
    fn parse_options_rejects_legacy_node_prefix_flags() {
        let err = parse_options(["--node-id", "n1"].into_iter()).expect_err("node-id should fail");
        assert!(err.contains("no longer supported"));
        assert!(err.contains("oasis7_chain_runtime"));
    }

    #[test]
    fn parse_options_rejects_unknown_option() {
        let err = parse_options(["--wat"].into_iter()).expect_err("unknown option");
        assert!(err.contains("unknown option"));
    }

    #[test]
    fn parse_options_rejects_unknown_scenario() {
        let err = parse_options(["wat"].into_iter()).expect_err("unknown scenario");
        assert!(err.contains("unknown scenario"));
    }

    #[test]
    fn parse_options_rejects_runtime_world_alias() {
        let err =
            parse_options(["--runtime-world"].into_iter()).expect_err("runtime alias should fail");
        assert!(err.contains("removed"));
        assert!(err.contains("runtime/world"));
    }

    #[test]
    fn parse_options_rejects_release_config_flag() {
        let err = parse_options(["--release-config", "legacy.toml"].into_iter()).expect_err("flag");
        assert!(err.contains("no longer supported"));
        assert!(err.contains("oasis7_chain_runtime"));
    }

    #[test]
    fn parse_options_rejects_no_runtime_world() {
        let err = parse_options(["--no-runtime-world"].into_iter()).expect_err("flag should fail");
        assert!(err.contains("no longer supported"));
        assert!(err.contains("runtime-only"));
    }
}
