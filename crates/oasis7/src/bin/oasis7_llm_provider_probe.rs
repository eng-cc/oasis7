use oasis7::simulator::{
    LlmAgentConfig, LlmClientError, OpenAiChatCompletionClient, DEFAULT_CONFIG_FILE_NAME,
};
use serde::Serialize;
use std::fmt::Write as _;
use std::path::Path;
use std::process::ExitCode;
use std::time::Instant;

fn main() -> ExitCode {
    let pretty = match parse_options(std::env::args().skip(1)) {
        Ok(pretty) => pretty,
        Err(message) => {
            eprintln!("{message}");
            return ExitCode::from(2);
        }
    };

    let payload = run_probe();
    let serialized = if pretty {
        serde_json::to_string_pretty(&payload)
    } else {
        serde_json::to_string(&payload)
    };
    match serialized {
        Ok(text) => println!("{text}"),
        Err(err) => {
            eprintln!("serialize probe result failed: {err}");
            return ExitCode::from(2);
        }
    }

    if payload.status == "ok" {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}

fn usage() -> &'static str {
    "Usage: oasis7_llm_provider_probe [--pretty]\n\n\
Probe the active oasis7 LLM provider using the same config resolution as runtime gameplay, plus both a hello text response check and a required tool-call contract check.\n"
}

fn parse_options(args: impl IntoIterator<Item = String>) -> Result<bool, String> {
    let mut pretty = false;
    for arg in args {
        match arg.as_str() {
            "--pretty" => pretty = true,
            "-h" | "--help" => return Err(usage().to_string()),
            other => return Err(format!("unknown option: {other}\n\n{}", usage())),
        }
    }
    Ok(pretty)
}

#[derive(Debug, Serialize)]
struct ProbePayload {
    status: &'static str,
    config_source: &'static str,
    config_path: Option<String>,
    model: Option<String>,
    base_url: Option<String>,
    timeout_ms: Option<u64>,
    latency_ms: u128,
    response_preview: Option<String>,
    hello_response_preview: Option<String>,
    tool_call_preview: Option<String>,
    tool_call_turn_count: Option<usize>,
    error_kind: Option<String>,
    error_message: Option<String>,
}

fn run_probe() -> ProbePayload {
    let config_path = Path::new(DEFAULT_CONFIG_FILE_NAME);
    let config_source = if config_path.exists() {
        "config_file"
    } else {
        "env"
    };
    let config_path = config_path
        .exists()
        .then(|| config_path.display().to_string());

    let started_at = Instant::now();
    let config = match LlmAgentConfig::from_default_sources() {
        Ok(config) => config,
        Err(err) => {
            let error_message = err.to_string();
            return ProbePayload {
                status: "error",
                config_source,
                config_path,
                model: None,
                base_url: None,
                timeout_ms: None,
                latency_ms: started_at.elapsed().as_millis(),
                response_preview: None,
                hello_response_preview: None,
                tool_call_preview: None,
                tool_call_turn_count: None,
                error_kind: Some(classify_config_error_message(error_message.as_str()).to_string()),
                error_message: Some(error_message),
            };
        }
    };

    let client = match OpenAiChatCompletionClient::from_config(&config) {
        Ok(client) => client,
        Err(err) => {
            return ProbePayload {
                status: "error",
                config_source,
                config_path,
                model: Some(config.model.clone()),
                base_url: Some(config.base_url.clone()),
                timeout_ms: Some(config.timeout_ms),
                latency_ms: started_at.elapsed().as_millis(),
                response_preview: None,
                hello_response_preview: None,
                tool_call_preview: None,
                tool_call_turn_count: None,
                error_kind: Some(classify_client_error(&err).to_string()),
                error_message: Some(err.to_string()),
            };
        }
    };

    let hello_response = match client.probe_hello_response(config.model.as_str()) {
        Ok(text) => text,
        Err(err) => {
            return ProbePayload {
                status: "error",
                config_source,
                config_path,
                model: Some(config.model),
                base_url: Some(config.base_url),
                timeout_ms: Some(config.timeout_ms),
                latency_ms: started_at.elapsed().as_millis(),
                response_preview: None,
                hello_response_preview: None,
                tool_call_preview: None,
                tool_call_turn_count: None,
                error_kind: Some(classify_client_error(&err).to_string()),
                error_message: Some(err.to_string()),
            };
        }
    };
    let hello_preview = preview_text(hello_response.as_str(), 160);

    match client.probe_required_tool_response(config.model.as_str()) {
        Ok(result) => ProbePayload {
            status: "ok",
            config_source,
            config_path,
            model: Some(config.model),
            base_url: Some(config.base_url),
            timeout_ms: Some(config.timeout_ms),
            latency_ms: started_at.elapsed().as_millis(),
            response_preview: Some(hello_preview.clone()),
            hello_response_preview: Some(hello_preview),
            tool_call_preview: Some(preview_text(result.output.as_str(), 160)),
            tool_call_turn_count: Some(result.turns.len()),
            error_kind: None,
            error_message: None,
        },
        Err(err) => ProbePayload {
            status: "error",
            config_source,
            config_path,
            model: Some(config.model),
            base_url: Some(config.base_url),
            timeout_ms: Some(config.timeout_ms),
            latency_ms: started_at.elapsed().as_millis(),
            response_preview: Some(hello_preview.clone()),
            hello_response_preview: Some(hello_preview),
            tool_call_preview: None,
            tool_call_turn_count: None,
            error_kind: Some(classify_client_error(&err).to_string()),
            error_message: Some(err.to_string()),
        },
    }
}

fn classify_config_error_message(message: &str) -> &'static str {
    if message.contains("missing env variable") || message.contains("empty env variable") {
        "config_missing"
    } else if message.contains("read config file failed")
        || message.contains("parse config file failed")
    {
        "config_file_error"
    } else if message.contains("invalid ") {
        "config_invalid"
    } else {
        "config_error"
    }
}

fn classify_client_error(err: &LlmClientError) -> &'static str {
    match err {
        LlmClientError::BuildClient { .. } => "client_build_error",
        LlmClientError::Http { .. } => "http_error",
        LlmClientError::HttpStatus { .. } => "http_status_error",
        LlmClientError::DecodeResponse { .. } => "decode_error",
        LlmClientError::EmptyChoice => "empty_choice",
    }
}

fn preview_text(text: &str, limit: usize) -> String {
    if text.chars().count() <= limit {
        return text.to_string();
    }
    let mut preview = String::new();
    for ch in text.chars().take(limit) {
        let _ = preview.write_char(ch);
    }
    preview.push_str("...");
    preview
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::thread;
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    #[test]
    fn probe_hello_response_succeeds_against_mock_stream() {
        let base_url = spawn_success_responses_server();
        let config_path = write_temp_config(base_url.as_str(), 5000);
        let config = LlmAgentConfig::from_config_file(config_path.as_path()).expect("load config");
        let client = OpenAiChatCompletionClient::from_config(&config).expect("client");

        let result = client
            .probe_hello_response(config.model.as_str())
            .expect("probe success");

        assert_eq!(result, "hello back");
    }

    #[test]
    fn probe_hello_response_reports_timeout_from_mock_server() {
        let base_url = spawn_slow_responses_server(Duration::from_millis(400));
        let config_path = write_temp_config(base_url.as_str(), 100);
        let config = LlmAgentConfig::from_config_file(config_path.as_path()).expect("load config");
        let client = OpenAiChatCompletionClient::from_config(&config).expect("client");

        let err = client
            .probe_hello_response(config.model.as_str())
            .expect_err("probe should time out");

        match err {
            LlmClientError::Http { message } => {
                assert!(message.contains("request timed out after 100ms"));
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[test]
    fn probe_required_tool_response_succeeds_against_mock_stream() {
        let base_url = spawn_tool_call_responses_server();
        let config_path = write_temp_config(base_url.as_str(), 5000);
        let config = LlmAgentConfig::from_config_file(config_path.as_path()).expect("load config");
        let client = OpenAiChatCompletionClient::from_config(&config).expect("client");

        let result = client
            .probe_required_tool_response(config.model.as_str())
            .expect("tool probe success");

        assert_eq!(result.turns.len(), 1);
        assert!(result.output.contains("\"module\":\"world.rules.guide\""));
        assert!(result.output.contains("\"topic\":\"quickstart\""));
    }

    #[test]
    fn classify_config_error_message_distinguishes_common_failures() {
        assert_eq!(
            classify_config_error_message("missing env variable: OASIS7_LLM_MODEL"),
            "config_missing"
        );
        assert_eq!(
            classify_config_error_message("parse config file failed (config.toml): invalid type"),
            "config_file_error"
        );
        assert_eq!(
            classify_config_error_message("invalid timeout value: nope"),
            "config_invalid"
        );
    }

    fn spawn_success_responses_server() -> String {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind success server");
        let bind = listener.local_addr().expect("listener addr");
        thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept request");
            let _ = stream.set_read_timeout(Some(Duration::from_millis(200)));
            let mut buffer = [0_u8; 8192];
            let _ = stream.read(&mut buffer);

            let event_one = serde_json::json!({
                "type": "response.output_text.done",
                "sequence_number": 1,
                "item_id": "msg_1",
                "output_index": 0,
                "content_index": 0,
                "text": "hello back",
                "logprobs": null
            });
            let event_two = serde_json::json!({
                "type": "response.completed",
                "sequence_number": 2,
                "response": {
                    "id": "resp_probe_success",
                    "object": "response",
                    "created_at": 1,
                    "completed_at": 2,
                    "model": "gpt-test",
                    "output": [{
                        "type": "message",
                        "id": "msg_1",
                        "role": "assistant",
                        "status": "completed",
                        "content": [{
                            "type": "output_text",
                            "text": "hello back",
                            "annotations": []
                        }]
                    }],
                    "status": "completed",
                    "parallel_tool_calls": false
                }
            });
            let body = format!("data: {event_one}\n\ndata: {event_two}\n\ndata: [DONE]\n\n");
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nCache-Control: no-cache\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            let _ = stream.write_all(response.as_bytes());
        });
        format!("http://127.0.0.1:{}/v1", bind.port())
    }

    fn spawn_slow_responses_server(response_delay: Duration) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind slow server");
        let bind = listener.local_addr().expect("listener addr");
        thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept request");
            let _ = stream.set_read_timeout(Some(Duration::from_millis(200)));
            let mut buffer = [0_u8; 4096];
            let _ = stream.read(&mut buffer);
            thread::sleep(response_delay);
            let body = "data: [DONE]\n\n";
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            let _ = stream.write_all(response.as_bytes());
        });
        format!("http://127.0.0.1:{}/v1", bind.port())
    }

    fn spawn_tool_call_responses_server() -> String {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind tool server");
        let bind = listener.local_addr().expect("listener addr");
        thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept request");
            let _ = stream.set_read_timeout(Some(Duration::from_millis(200)));
            let mut buffer = [0_u8; 8192];
            let _ = stream.read(&mut buffer);

            let event_one = serde_json::json!({
                "type": "response.output_item.done",
                "sequence_number": 1,
                "output_index": 0,
                "item": {
                    "type": "function_call",
                    "call_id": "call_world_rules",
                    "name": "world_rules_guide",
                    "arguments": "{\"topic\":\"quickstart\"}",
                    "status": "completed"
                }
            });
            let event_two = serde_json::json!({
                "type": "response.completed",
                "sequence_number": 2,
                "response": {
                    "id": "resp_probe_tool_success",
                    "object": "response",
                    "created_at": 1,
                    "completed_at": 2,
                    "model": "gpt-test",
                    "output": [],
                    "status": "completed",
                    "parallel_tool_calls": false
                }
            });
            let body = format!("data: {event_one}\n\ndata: {event_two}\n\ndata: [DONE]\n\n");
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nCache-Control: no-cache\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            let _ = stream.write_all(response.as_bytes());
        });
        format!("http://127.0.0.1:{}/v1", bind.port())
    }

    fn write_temp_config(base_url: &str, timeout_ms: u64) -> std::path::PathBuf {
        let mut path = std::env::temp_dir();
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        path.push(format!(
            "oasis7-llm-provider-probe-{}-{nonce}.toml",
            std::process::id()
        ));
        let content = format!(
            "[llm]\nmodel = \"gpt-test\"\nbase_url = \"{}\"\napi_key = \"test-key\"\ntimeout_ms = {}\n",
            base_url, timeout_ms
        );
        fs::write(&path, content).expect("write temp config");
        path
    }
}
