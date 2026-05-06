use std::env;
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::Serialize;
use serde_json::json;

#[path = "oasis7_newapi_bridge_service/api.rs"]
mod api;
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
use model::{BindBridgeUserRequest, CreateDepositRouteRequest};
use service::{BridgeService, BridgeServiceConfig, BridgeServiceError};
use store::BridgeStateStore;

const DEFAULT_BIND_ADDR: &str = "127.0.0.1:5852";
const DEFAULT_STATE_PATH: &str = "output/newapi-bridge/bridge-state.json";
const DEFAULT_ROUTE_TTL_SECONDS: u64 = 15 * 60;
const DEFAULT_DEPOSIT_ACCOUNT_PREFIX: &str = "oc:bridge:";

#[derive(Debug, Clone)]
struct CliOptions {
    bind_addr: String,
    state_path: PathBuf,
    route_ttl_seconds: u64,
    deposit_account_prefix: String,
}

impl Default for CliOptions {
    fn default() -> Self {
        Self {
            bind_addr: DEFAULT_BIND_ADDR.to_string(),
            state_path: PathBuf::from(DEFAULT_STATE_PATH),
            route_ttl_seconds: DEFAULT_ROUTE_TTL_SECONDS,
            deposit_account_prefix: DEFAULT_DEPOSIT_ACCOUNT_PREFIX.to_string(),
        }
    }
}

#[derive(Debug)]
struct EncodedResponse {
    status_code: u16,
    body: Vec<u8>,
}

fn main() {
    if let Err(err) = run() {
        eprintln!("error: {err}");
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
        },
    ));
    let listener = TcpListener::bind(options.bind_addr.as_str())
        .map_err(|err| format!("bind {} failed: {err}", options.bind_addr))?;
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
                        eprintln!("warning: bridge-service connection failed: {err}");
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
    Ok(options)
}

fn print_help() {
    println!("oasis7_newapi_bridge_service");
    println!("  --bind-addr <host:port>              default: {DEFAULT_BIND_ADDR}");
    println!("  --state-path <path>                  default: {DEFAULT_STATE_PATH}");
    println!("  --route-ttl-seconds <seconds>        default: {DEFAULT_ROUTE_TTL_SECONDS}");
    println!("  --deposit-account-prefix <prefix>    default: {DEFAULT_DEPOSIT_ACCOUNT_PREFIX}");
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
        (_, "/v1/bridge/bind") | (_, "/v1/bridge/deposit-route") => json_response(
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
