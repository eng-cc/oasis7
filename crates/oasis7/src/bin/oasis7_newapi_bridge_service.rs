use std::env;
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use oasis7::observability::{emit_stderr_or_event, init_tracing, resolve_trace_session_id};
use serde::Serialize;
use serde_json::json;
use tracing::{error, info, Level};

#[path = "oasis7_newapi_bridge_service/api.rs"]
mod api;
#[path = "oasis7_newapi_bridge_service/chain_client.rs"]
mod chain_client;
#[path = "oasis7_newapi_bridge_service/credit_adapter.rs"]
mod credit_adapter;
#[path = "oasis7_newapi_bridge_service/model.rs"]
mod model;
#[path = "oasis7_newapi_bridge_service/service.rs"]
mod service;
#[path = "oasis7_newapi_bridge_service/store.rs"]
mod store;
#[cfg(test)]
#[path = "oasis7_newapi_bridge_service/tests.rs"]
mod tests;

use api::{read_http_request, write_http_response, HttpRequest};
use model::{BindBridgeUserRequest, CreateDepositRouteRequest, OperatorReviewRequest};
use service::{BridgePricingRuleConfig, BridgeService, BridgeServiceConfig, BridgeServiceError};
use store::BridgeStateStore;

const DEFAULT_BIND_ADDR: &str = "127.0.0.1:5852";
const DEFAULT_STATE_PATH: &str = "output/newapi-bridge/bridge-state.json";
const DEFAULT_ROUTE_TTL_SECONDS: u64 = 15 * 60;
const DEFAULT_DEPOSIT_ACCOUNT_PREFIX: &str = "oc:bridge:";
const TRACE_SESSION_PROCESS_LABEL: &str = "oasis7_newapi_bridge_service";
const DEFAULT_CHAIN_TIMEOUT_MS: u64 = 5_000;
const DEFAULT_CHAIN_CONFIRMATIONS_REQUIRED: u64 = 1;
const DEFAULT_LETAI_TIMEOUT_MS: u64 = 5_000;
const DEFAULT_MAX_CREDIT_ATTEMPTS: u32 = 3;

#[derive(Debug, Clone)]
struct CliOptions {
    bind_addr: String,
    state_path: PathBuf,
    route_ttl_seconds: u64,
    deposit_account_prefix: String,
    chain_base_url: Option<String>,
    chain_timeout_ms: u64,
    chain_confirmations_required: u64,
    pricing_rules: Vec<BridgePricingRuleConfig>,
    letai_base_url: Option<String>,
    letai_platform_key: Option<String>,
    letai_parent_channel_id: Option<String>,
    letai_timeout_ms: u64,
    max_credit_attempts: u32,
    reconcile_interval_seconds: u64,
}

impl Default for CliOptions {
    fn default() -> Self {
        Self {
            bind_addr: DEFAULT_BIND_ADDR.to_string(),
            state_path: PathBuf::from(DEFAULT_STATE_PATH),
            route_ttl_seconds: DEFAULT_ROUTE_TTL_SECONDS,
            deposit_account_prefix: DEFAULT_DEPOSIT_ACCOUNT_PREFIX.to_string(),
            chain_base_url: None,
            chain_timeout_ms: DEFAULT_CHAIN_TIMEOUT_MS,
            chain_confirmations_required: DEFAULT_CHAIN_CONFIRMATIONS_REQUIRED,
            pricing_rules: Vec::new(),
            letai_base_url: None,
            letai_platform_key: None,
            letai_parent_channel_id: None,
            letai_timeout_ms: DEFAULT_LETAI_TIMEOUT_MS,
            max_credit_attempts: DEFAULT_MAX_CREDIT_ATTEMPTS,
            reconcile_interval_seconds: 0,
        }
    }
}

#[derive(Debug)]
struct EncodedResponse {
    status_code: u16,
    body: Vec<u8>,
}

fn main() {
    init_tracing(TRACE_SESSION_PROCESS_LABEL);
    let trace_session_id = resolve_trace_session_id(TRACE_SESSION_PROCESS_LABEL);
    if let Err(err) = run() {
        error!(trace_session_id = %trace_session_id, error = %err, "oasis7_newapi_bridge_service failed");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let options = parse_cli_options(env::args().skip(1).collect())?;
    let store = Arc::new(BridgeStateStore::new(options.state_path.clone())?);
    let service = Arc::new(BridgeService::new(
        store,
        BridgeServiceConfig {
            route_ttl_seconds: options.route_ttl_seconds,
            deposit_account_prefix: options.deposit_account_prefix.clone(),
            chain_base_url: options.chain_base_url.clone(),
            chain_timeout_ms: options.chain_timeout_ms,
            chain_confirmations_required: options.chain_confirmations_required,
            pricing_rules: options.pricing_rules.clone(),
            letai_base_url: options.letai_base_url.clone(),
            letai_platform_key: options.letai_platform_key.clone(),
            letai_parent_channel_id: options.letai_parent_channel_id.clone(),
            letai_timeout_ms: options.letai_timeout_ms,
            max_credit_attempts: options.max_credit_attempts,
        },
    ));
    if options.reconcile_interval_seconds > 0 {
        let service = Arc::clone(&service);
        let interval = options.reconcile_interval_seconds;
        thread::spawn(move || loop {
            thread::sleep(Duration::from_secs(interval));
            if let Err(err) = service.reconcile_once(now_unix_ms()) {
                eprintln!(
                    "warning: bridge-service reconcile loop failed: {}",
                    err.message
                );
            }
        });
    }
    let listener = TcpListener::bind(options.bind_addr.as_str())
        .map_err(|err| format!("bind {} failed: {err}", options.bind_addr))?;
    let trace_session_id = resolve_trace_session_id(TRACE_SESSION_PROCESS_LABEL);
    info!(
        trace_session_id = %trace_session_id,
        bind_addr = %options.bind_addr,
        state_path = %options.state_path.display(),
        route_ttl_seconds = options.route_ttl_seconds,
        deposit_account_prefix = %options.deposit_account_prefix,
        "newapi bridge service listening"
    );
    println!(
        "oasis7_newapi_bridge_service listening on {} with state {}",
        options.bind_addr,
        options.state_path.display()
    );
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let service = Arc::clone(&service);
                thread::spawn(move || {
                    if let Err(err) = handle_connection(stream, service.as_ref()) {
                        let stderr_message =
                            format!("warning: bridge-service connection failed: {err}");
                        emit_stderr_or_event(
                            Level::WARN,
                            stderr_message.as_str(),
                            "newapi bridge service connection failed",
                        );
                    }
                });
            }
            Err(err) => {
                return Err(format!("accept failed: {err}"));
            }
        }
    }
    Ok(())
}

fn parse_cli_options(args: Vec<String>) -> Result<CliOptions, String> {
    let mut options = CliOptions::default();
    let mut index = 0usize;
    while index < args.len() {
        let arg = args[index].as_str();
        match arg {
            "--bind-addr" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| "--bind-addr requires a value".to_string())?;
                options.bind_addr = value.to_string();
                index += 2;
            }
            "--state-path" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| "--state-path requires a value".to_string())?;
                options.state_path = PathBuf::from(value);
                index += 2;
            }
            "--route-ttl-seconds" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| "--route-ttl-seconds requires a value".to_string())?;
                options.route_ttl_seconds = value
                    .parse::<u64>()
                    .map_err(|_| format!("invalid --route-ttl-seconds value `{value}`"))?;
                index += 2;
            }
            "--deposit-account-prefix" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| "--deposit-account-prefix requires a value".to_string())?;
                options.deposit_account_prefix = value.trim().to_string();
                index += 2;
            }
            "--chain-base-url" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| "--chain-base-url requires a value".to_string())?;
                options.chain_base_url = Some(value.trim().to_string());
                index += 2;
            }
            "--chain-timeout-ms" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| "--chain-timeout-ms requires a value".to_string())?;
                options.chain_timeout_ms = value
                    .parse::<u64>()
                    .map_err(|_| format!("invalid --chain-timeout-ms value `{value}`"))?;
                index += 2;
            }
            "--chain-confirmations-required" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| "--chain-confirmations-required requires a value".to_string())?;
                options.chain_confirmations_required = value.parse::<u64>().map_err(|_| {
                    format!("invalid --chain-confirmations-required value `{value}`")
                })?;
                index += 2;
            }
            "--pricing-rule" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| "--pricing-rule requires a value".to_string())?;
                options.pricing_rules.push(parse_pricing_rule(value)?);
                index += 2;
            }
            "--letai-base-url" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| "--letai-base-url requires a value".to_string())?;
                options.letai_base_url = Some(value.trim().to_string());
                index += 2;
            }
            "--letai-platform-key" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| "--letai-platform-key requires a value".to_string())?;
                options.letai_platform_key = Some(value.to_string());
                index += 2;
            }
            "--letai-parent-channel-id" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| "--letai-parent-channel-id requires a value".to_string())?;
                options.letai_parent_channel_id = Some(value.trim().to_string());
                index += 2;
            }
            "--letai-timeout-ms" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| "--letai-timeout-ms requires a value".to_string())?;
                options.letai_timeout_ms = value
                    .parse::<u64>()
                    .map_err(|_| format!("invalid --letai-timeout-ms value `{value}`"))?;
                index += 2;
            }
            "--max-credit-attempts" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| "--max-credit-attempts requires a value".to_string())?;
                options.max_credit_attempts = value
                    .parse::<u32>()
                    .map_err(|_| format!("invalid --max-credit-attempts value `{value}`"))?;
                index += 2;
            }
            "--reconcile-interval-seconds" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| "--reconcile-interval-seconds requires a value".to_string())?;
                options.reconcile_interval_seconds = value
                    .parse::<u64>()
                    .map_err(|_| format!("invalid --reconcile-interval-seconds value `{value}`"))?;
                index += 2;
            }
            "--help" | "-h" => {
                print_help();
                std::process::exit(0);
            }
            _ => {
                return Err(format!("unknown argument `{arg}`"));
            }
        }
    }
    if options.bind_addr.trim().is_empty() {
        return Err("--bind-addr must not be empty".to_string());
    }
    if options.route_ttl_seconds == 0 {
        return Err("--route-ttl-seconds must be greater than 0".to_string());
    }
    validate_route_ttl_seconds(options.route_ttl_seconds)?;
    if options.deposit_account_prefix.trim().is_empty() {
        return Err("--deposit-account-prefix must not be empty".to_string());
    }
    if options.chain_timeout_ms == 0 {
        return Err("--chain-timeout-ms must be greater than 0".to_string());
    }
    if options.chain_confirmations_required == 0 {
        return Err("--chain-confirmations-required must be greater than 0".to_string());
    }
    if options.letai_timeout_ms == 0 {
        return Err("--letai-timeout-ms must be greater than 0".to_string());
    }
    if options.max_credit_attempts == 0 {
        return Err("--max-credit-attempts must be greater than 0".to_string());
    }
    Ok(options)
}

fn print_help() {
    println!("oasis7_newapi_bridge_service");
    println!("  --bind-addr <host:port>              default: {DEFAULT_BIND_ADDR}");
    println!("  --state-path <path>                  default: {DEFAULT_STATE_PATH}");
    println!("  --route-ttl-seconds <seconds>        default: {DEFAULT_ROUTE_TTL_SECONDS}");
    println!("  --deposit-account-prefix <prefix>    default: {DEFAULT_DEPOSIT_ACCOUNT_PREFIX}");
    println!("  --chain-base-url <url>               optional chain explorer base URL");
    println!("  --chain-timeout-ms <ms>              default: {DEFAULT_CHAIN_TIMEOUT_MS}");
    println!(
        "  --chain-confirmations-required <n>   default: {DEFAULT_CHAIN_CONFIRMATIONS_REQUIRED}"
    );
    println!("  --pricing-rule <version:oc:credit[:bonus]>  repeatable exact-match pricing rule");
    println!("  --letai-base-url <url>               optional LetAI OpenAPI base URL");
    println!("  --letai-platform-key <token>         optional LetAI platform management key");
    println!("  --letai-parent-channel-id <id>       optional LetAI parent channel identifier");
    println!("  --letai-timeout-ms <ms>              default: {DEFAULT_LETAI_TIMEOUT_MS}");
    println!("  --max-credit-attempts <n>            default: {DEFAULT_MAX_CREDIT_ATTEMPTS}");
    println!("  --reconcile-interval-seconds <n>     default: disabled");
}

fn handle_connection(mut stream: TcpStream, service: &BridgeService) -> Result<(), String> {
    let request = match read_http_request(&mut stream) {
        Ok(request) => request,
        Err(err) => {
            let response = json_response(
                400,
                &json!({"ok": false, "error": {"code": "bad_request", "message": err}}),
            )?;
            return write_http_response(
                &mut stream,
                response.status_code,
                "application/json; charset=utf-8",
                response.body.as_slice(),
                false,
            );
        }
    };
    let response = dispatch_request(service, request)?;
    write_http_response(
        &mut stream,
        response.status_code,
        "application/json; charset=utf-8",
        response.body.as_slice(),
        false,
    )
}

fn dispatch_request(
    service: &BridgeService,
    request: HttpRequest,
) -> Result<EncodedResponse, String> {
    let path = request
        .path
        .split('?')
        .next()
        .unwrap_or(request.path.as_str())
        .to_string();
    match (request.method.as_str(), path.as_str()) {
        ("GET", "/health") | ("GET", "/v1/bridge/health") => {
            json_response(200, &service.health(now_unix_ms()))
        }
        ("POST", "/v1/bridge/bind") => {
            let payload: BindBridgeUserRequest =
                match serde_json::from_slice(request.body.as_slice()) {
                    Ok(payload) => payload,
                    Err(err) => {
                        return json_response(
                            400,
                            &json!({
                                "ok": false,
                                "error": {
                                    "code": "invalid_json",
                                    "message": format!("decode bind request failed: {err}"),
                                }
                            }),
                        );
                    }
                };
            match service.bind_user(payload, now_unix_ms()) {
                Ok(response) => json_response(200, &response),
                Err(err) => json_error_response(&err),
            }
        }
        ("POST", "/v1/bridge/deposit-route") => {
            let payload: CreateDepositRouteRequest = match serde_json::from_slice(
                request.body.as_slice(),
            ) {
                Ok(payload) => payload,
                Err(err) => {
                    return json_response(
                        400,
                        &json!({
                            "ok": false,
                            "error": {
                                "code": "invalid_json",
                                "message": format!("decode deposit-route request failed: {err}"),
                            }
                        }),
                    );
                }
            };
            match service.create_deposit_route(payload, now_unix_ms()) {
                Ok(response) => json_response(200, &response),
                Err(err) => json_error_response(&err),
            }
        }
        ("POST", "/v1/bridge/reconcile") => match service.reconcile_once(now_unix_ms()) {
            Ok(response) => json_response(200, &response),
            Err(err) => json_error_response(&err),
        },
        ("POST", _) if path.starts_with("/v1/bridge/operator/review/") => {
            let bridge_deposit_id = path
                .trim_start_matches("/v1/bridge/operator/review/")
                .trim();
            let payload: OperatorReviewRequest = match serde_json::from_slice(
                request.body.as_slice(),
            ) {
                Ok(payload) => payload,
                Err(err) => {
                    return json_response(
                        400,
                        &json!({
                            "ok": false,
                            "error": {
                                "code": "invalid_json",
                                "message": format!("decode operator review request failed: {err}"),
                            }
                        }),
                    );
                }
            };
            match service.apply_operator_review(bridge_deposit_id, payload, now_unix_ms()) {
                Ok(response) => json_response(200, &response),
                Err(err) => json_error_response(&err),
            }
        }
        (_, "/v1/bridge/bind")
        | (_, "/v1/bridge/deposit-route")
        | (_, "/v1/bridge/reconcile")
        | (_, "/v1/bridge/operator/review") => json_response(
            405,
            &json!({
                "ok": false,
                "error": {
                    "code": "method_not_allowed",
                    "message": format!("{} is not allowed for {}", request.method, path),
                }
            }),
        ),
        _ => json_response(
            404,
            &json!({
                "ok": false,
                "error": {
                    "code": "not_found",
                    "message": format!("no route for {} {}", request.method, path),
                }
            }),
        ),
    }
}

fn json_error_response(err: &BridgeServiceError) -> Result<EncodedResponse, String> {
    json_response(
        err.status_code,
        &json!({
            "ok": false,
            "error": {
                "code": err.code,
                "message": err.message,
            }
        }),
    )
}

fn json_response(status_code: u16, payload: &impl Serialize) -> Result<EncodedResponse, String> {
    let body =
        serde_json::to_vec(payload).map_err(|err| format!("encode JSON response failed: {err}"))?;
    Ok(EncodedResponse { status_code, body })
}

fn now_unix_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::from_secs(0))
        .as_millis() as i64
}

fn validate_route_ttl_seconds(route_ttl_seconds: u64) -> Result<(), String> {
    let ttl_ms = route_ttl_seconds.checked_mul(1000).ok_or_else(|| {
        "--route-ttl-seconds is too large to convert into milliseconds".to_string()
    })?;
    if ttl_ms > i64::MAX as u64 {
        return Err("--route-ttl-seconds exceeds the supported millisecond range".to_string());
    }
    Ok(())
}

fn parse_pricing_rule(raw: &str) -> Result<BridgePricingRuleConfig, String> {
    let parts = raw.split(':').collect::<Vec<_>>();
    if parts.len() < 3 || parts.len() > 4 {
        return Err(format!(
            "invalid --pricing-rule `{raw}`; expected version:oc_amount:credit_units[:bonus_units]"
        ));
    }
    let pricing_version = parts[0].trim();
    if pricing_version.is_empty() {
        return Err("pricing rule version must not be empty".to_string());
    }
    let oc_amount = parts[1]
        .trim()
        .parse::<u64>()
        .map_err(|_| format!("invalid pricing rule OC amount in `{raw}`"))?;
    let credit_units = parts[2]
        .trim()
        .parse::<u64>()
        .map_err(|_| format!("invalid pricing rule credit units in `{raw}`"))?;
    let bonus_units = if let Some(raw_bonus) = parts.get(3) {
        raw_bonus
            .trim()
            .parse::<u64>()
            .map_err(|_| format!("invalid pricing rule bonus units in `{raw}`"))?
    } else {
        0
    };
    Ok(BridgePricingRuleConfig {
        pricing_version: pricing_version.to_string(),
        oc_amount,
        credit_units,
        bonus_units,
    })
}
