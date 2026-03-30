use std::convert::TryInto;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::net::TcpStream;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use ed25519_dalek::SigningKey;
use oasis7::simulator::WorldSnapshot;
use oasis7::viewer::{
    sign_agent_chat_auth_proof, sign_gameplay_action_auth_proof,
    sign_prompt_control_apply_auth_proof, sign_prompt_control_rollback_auth_proof,
    AgentChatRequest, AuthoritativeReconnectSyncRequest, AuthoritativeRecoveryCommand,
    AuthoritativeSessionRevokeRequest, AuthoritativeSessionRotateRequest, GameplayActionRequest,
    LiveControl, PromptControlApplyRequest, PromptControlAuthIntent, PromptControlCommand,
    PromptControlRollbackRequest, ViewerRequest, ViewerResponse, ViewerStream,
    VIEWER_PROTOCOL_VERSION,
};
use rand_core::OsRng;
use serde_json::{json, Value};

const DEFAULT_ADDR: &str = "127.0.0.1:5023";
const DEFAULT_CLIENT: &str = "oasis7_pure_api_client";
const DEFAULT_TIMEOUT_MS: u64 = 3_000;

#[path = "oasis7_pure_api_client/support.rs"]
mod oasis7_pure_api_client_support;
#[cfg(test)]
#[path = "oasis7_pure_api_client/tests.rs"]
mod tests;

use self::oasis7_pure_api_client_support::{
    build_signed_agent_chat_request, build_signed_gameplay_action_request,
    build_signed_prompt_apply_request, build_signed_prompt_rollback_request, command_output,
    derive_public_key_hex, is_terminal_error, keygen_output, latest_snapshot,
    maybe_request_snapshot, next_u64_id, parse_bool_flag, parse_u64_flag, parse_usize_flag,
    print_json, required_flag, resolve_public_key_hex, subscribe_for_control,
    terminal_agent_chat, terminal_control_ack, terminal_gameplay_action, terminal_hello,
    terminal_prompt_control, terminal_recovery, terminal_snapshot,
};

fn main() {
    if let Err(err) = run() {
        eprintln!("error: {err}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let mut args = ArgCursor::from_env()?;
    let config = parse_cli(&mut args)?;
    if args.has_more() {
        return Err(format!(
            "unexpected trailing arguments: {}",
            args.remaining().join(" ")
        ));
    }
    let addr = config.addr.clone();
    let client = config.client.clone();
    let timeout = config.timeout();

    match config.command {
        Command::Keygen => {
            print_json(&keygen_output()?)?;
            Ok(())
        }
        Command::Snapshot {
            player_gameplay_only,
        } => {
            let mut conn = ViewerConnection::connect(addr.as_str(), client.as_str(), timeout)?;
            conn.send(&ViewerRequest::RequestSnapshot)?;
            let response =
                conn.collect_until(timeout, terminal_snapshot, "waiting for snapshot response")?;
            let latest_snapshot = latest_snapshot(&response).ok_or_else(|| {
                "snapshot response did not include a snapshot payload".to_string()
            })?;
            if player_gameplay_only {
                print_json(
                    &serde_json::to_value(latest_snapshot.player_gameplay.clone())
                        .map_err(|err| format!("serialize player_gameplay failed: {err}"))?,
                )?;
            } else {
                print_json(
                    &serde_json::to_value(latest_snapshot)
                        .map_err(|err| format!("serialize snapshot failed: {err}"))?,
                )?;
            }
            Ok(())
        }
        Command::Step {
            count,
            request_id,
            include_events,
            include_metrics,
        } => {
            let mut conn = ViewerConnection::connect(addr.as_str(), client.as_str(), timeout)?;
            subscribe_for_control(&mut conn, include_events, include_metrics)?;
            let request_id = request_id.unwrap_or_else(next_u64_id);
            conn.send(&ViewerRequest::LiveControl {
                mode: LiveControl::Step { count },
                request_id: Some(request_id),
            })?;
            let responses = conn.collect_until(
                timeout,
                |response| terminal_control_ack(response, request_id),
                "waiting for live_control step ack",
            )?;
            print_json(&command_output(&conn.hello_ack, &responses))?;
            Ok(())
        }
        Command::Play {
            include_events,
            include_metrics,
        } => {
            let mut conn = ViewerConnection::connect(addr.as_str(), client.as_str(), timeout)?;
            subscribe_for_control(&mut conn, include_events, include_metrics)?;
            conn.send(&ViewerRequest::LiveControl {
                mode: LiveControl::Play,
                request_id: None,
            })?;
            let responses = conn.collect_for(timeout)?;
            print_json(&command_output(&conn.hello_ack, &responses))?;
            Ok(())
        }
        Command::Pause {
            include_events,
            include_metrics,
        } => {
            let mut conn = ViewerConnection::connect(addr.as_str(), client.as_str(), timeout)?;
            subscribe_for_control(&mut conn, include_events, include_metrics)?;
            conn.send(&ViewerRequest::LiveControl {
                mode: LiveControl::Pause,
                request_id: None,
            })?;
            let responses = conn.collect_for(timeout)?;
            print_json(&command_output(&conn.hello_ack, &responses))?;
            Ok(())
        }
        Command::Chat {
            agent_id,
            player_id,
            private_key_hex,
            public_key_hex,
            message,
            intent_tick,
            intent_seq,
            with_snapshot,
        } => {
            let mut conn = ViewerConnection::connect(addr.as_str(), client.as_str(), timeout)?;
            let signed = build_signed_agent_chat_request(
                agent_id.as_str(),
                player_id.as_str(),
                message.as_str(),
                private_key_hex.as_str(),
                public_key_hex.as_deref(),
                intent_tick,
                intent_seq,
            )?;
            conn.send(&ViewerRequest::AgentChat { request: signed })?;
            let mut responses = conn.collect_until(
                timeout,
                terminal_agent_chat,
                "waiting for agent_chat ack/error",
            )?;
            maybe_request_snapshot(&mut conn, with_snapshot, &mut responses, timeout)?;
            print_json(&command_output(&conn.hello_ack, &responses))?;
            Ok(())
        }
        Command::GameplayAction {
            action_id,
            target_agent_id,
            player_id,
            private_key_hex,
            public_key_hex,
            with_snapshot,
        } => {
            let mut conn = ViewerConnection::connect(addr.as_str(), client.as_str(), timeout)?;
            let request = build_signed_gameplay_action_request(
                action_id.as_str(),
                target_agent_id.as_str(),
                player_id.as_str(),
                private_key_hex.as_str(),
                public_key_hex.as_deref(),
            )?;
            conn.send(&ViewerRequest::GameplayAction { request })?;
            let mut responses = conn.collect_until(
                timeout,
                terminal_gameplay_action,
                "waiting for gameplay_action ack/error",
            )?;
            maybe_request_snapshot(&mut conn, with_snapshot, &mut responses, timeout)?;
            print_json(&command_output(&conn.hello_ack, &responses))?;
            Ok(())
        }
        Command::PromptApply {
            agent_id,
            player_id,
            private_key_hex,
            public_key_hex,
            expected_version,
            updated_by,
            system_prompt_override,
            short_term_goal_override,
            long_term_goal_override,
            preview,
            with_snapshot,
        } => {
            let mut conn = ViewerConnection::connect(addr.as_str(), client.as_str(), timeout)?;
            let request = build_signed_prompt_apply_request(
                agent_id.as_str(),
                player_id.as_str(),
                private_key_hex.as_str(),
                public_key_hex.as_deref(),
                expected_version,
                updated_by,
                system_prompt_override,
                short_term_goal_override,
                long_term_goal_override,
                preview,
            )?;
            let command = if preview {
                PromptControlCommand::Preview { request }
            } else {
                PromptControlCommand::Apply { request }
            };
            conn.send(&ViewerRequest::PromptControl { command })?;
            let mut responses = conn.collect_until(
                timeout,
                terminal_prompt_control,
                "waiting for prompt_control ack/error",
            )?;
            maybe_request_snapshot(&mut conn, with_snapshot, &mut responses, timeout)?;
            print_json(&command_output(&conn.hello_ack, &responses))?;
            Ok(())
        }
        Command::PromptRollback {
            agent_id,
            player_id,
            private_key_hex,
            public_key_hex,
            to_version,
            expected_version,
            updated_by,
            with_snapshot,
        } => {
            let mut conn = ViewerConnection::connect(addr.as_str(), client.as_str(), timeout)?;
            let request = build_signed_prompt_rollback_request(
                agent_id.as_str(),
                player_id.as_str(),
                private_key_hex.as_str(),
                public_key_hex.as_deref(),
                to_version,
                expected_version,
                updated_by,
            )?;
            conn.send(&ViewerRequest::PromptControl {
                command: PromptControlCommand::Rollback { request },
            })?;
            let mut responses = conn.collect_until(
                timeout,
                terminal_prompt_control,
                "waiting for prompt_control rollback ack/error",
            )?;
            maybe_request_snapshot(&mut conn, with_snapshot, &mut responses, timeout)?;
            print_json(&command_output(&conn.hello_ack, &responses))?;
            Ok(())
        }
        Command::ReconnectSync {
            player_id,
            session_pubkey,
            last_known_log_cursor,
            expected_reorg_epoch,
            with_snapshot,
        } => {
            let mut conn = ViewerConnection::connect(addr.as_str(), client.as_str(), timeout)?;
            conn.send(&ViewerRequest::AuthoritativeRecovery {
                command: AuthoritativeRecoveryCommand::ReconnectSync {
                    request: AuthoritativeReconnectSyncRequest {
                        player_id,
                        session_pubkey,
                        last_known_log_cursor,
                        expected_reorg_epoch,
                    },
                },
            })?;
            let mut responses = conn.collect_until(
                timeout,
                terminal_recovery,
                "waiting for reconnect_sync ack/error",
            )?;
            maybe_request_snapshot(&mut conn, with_snapshot, &mut responses, timeout)?;
            print_json(&command_output(&conn.hello_ack, &responses))?;
            Ok(())
        }
        Command::RotateSession {
            player_id,
            old_session_pubkey,
            new_session_pubkey,
            rotate_reason,
        } => {
            let mut conn = ViewerConnection::connect(addr.as_str(), client.as_str(), timeout)?;
            conn.send(&ViewerRequest::AuthoritativeRecovery {
                command: AuthoritativeRecoveryCommand::RotateSession {
                    request: AuthoritativeSessionRotateRequest {
                        player_id,
                        old_session_pubkey,
                        new_session_pubkey,
                        rotate_reason,
                        rotated_by: Some(client.clone()),
                    },
                },
            })?;
            let responses = conn.collect_until(
                timeout,
                terminal_recovery,
                "waiting for rotate_session ack/error",
            )?;
            print_json(&command_output(&conn.hello_ack, &responses))?;
            Ok(())
        }
        Command::RevokeSession {
            player_id,
            session_pubkey,
            revoke_reason,
        } => {
            let mut conn = ViewerConnection::connect(addr.as_str(), client.as_str(), timeout)?;
            conn.send(&ViewerRequest::AuthoritativeRecovery {
                command: AuthoritativeRecoveryCommand::RevokeSession {
                    request: AuthoritativeSessionRevokeRequest {
                        player_id,
                        session_pubkey,
                        revoke_reason,
                        revoked_by: Some(client.clone()),
                    },
                },
            })?;
            let responses = conn.collect_until(
                timeout,
                terminal_recovery,
                "waiting for revoke_session ack/error",
            )?;
            print_json(&command_output(&conn.hello_ack, &responses))?;
            Ok(())
        }
    }
}

#[derive(Debug, Clone)]
struct CliConfig {
    addr: String,
    client: String,
    timeout_ms: u64,
    command: Command,
}

impl CliConfig {
    fn timeout(&self) -> Duration {
        Duration::from_millis(self.timeout_ms.max(1))
    }
}

#[derive(Debug, Clone)]
enum Command {
    Keygen,
    Snapshot {
        player_gameplay_only: bool,
    },
    Step {
        count: usize,
        request_id: Option<u64>,
        include_events: bool,
        include_metrics: bool,
    },
    Play {
        include_events: bool,
        include_metrics: bool,
    },
    Pause {
        include_events: bool,
        include_metrics: bool,
    },
    Chat {
        agent_id: String,
        player_id: String,
        private_key_hex: String,
        public_key_hex: Option<String>,
        message: String,
        intent_tick: Option<u64>,
        intent_seq: Option<u64>,
        with_snapshot: bool,
    },
    GameplayAction {
        action_id: String,
        target_agent_id: String,
        player_id: String,
        private_key_hex: String,
        public_key_hex: Option<String>,
        with_snapshot: bool,
    },
    PromptApply {
        agent_id: String,
        player_id: String,
        private_key_hex: String,
        public_key_hex: Option<String>,
        expected_version: Option<u64>,
        updated_by: Option<String>,
        system_prompt_override: Option<Option<String>>,
        short_term_goal_override: Option<Option<String>>,
        long_term_goal_override: Option<Option<String>>,
        preview: bool,
        with_snapshot: bool,
    },
    PromptRollback {
        agent_id: String,
        player_id: String,
        private_key_hex: String,
        public_key_hex: Option<String>,
        to_version: u64,
        expected_version: Option<u64>,
        updated_by: Option<String>,
        with_snapshot: bool,
    },
    ReconnectSync {
        player_id: String,
        session_pubkey: Option<String>,
        last_known_log_cursor: Option<u64>,
        expected_reorg_epoch: Option<u64>,
        with_snapshot: bool,
    },
    RotateSession {
        player_id: String,
        old_session_pubkey: String,
        new_session_pubkey: String,
        rotate_reason: String,
    },
    RevokeSession {
        player_id: String,
        session_pubkey: Option<String>,
        revoke_reason: String,
    },
}

struct ViewerConnection {
    reader: BufReader<TcpStream>,
    writer: BufWriter<TcpStream>,
    hello_ack: Value,
}

impl ViewerConnection {
    fn connect(addr: &str, client: &str, timeout: Duration) -> Result<Self, String> {
        let stream =
            TcpStream::connect(addr).map_err(|err| format!("connect {addr} failed: {err}"))?;
        stream
            .set_nodelay(true)
            .map_err(|err| format!("set_nodelay failed: {err}"))?;
        let reader = BufReader::new(
            stream
                .try_clone()
                .map_err(|err| format!("clone tcp stream failed: {err}"))?,
        );
        let writer = BufWriter::new(stream);
        let mut conn = Self {
            reader,
            writer,
            hello_ack: Value::Null,
        };
        conn.send(&ViewerRequest::Hello {
            client: client.to_string(),
            version: VIEWER_PROTOCOL_VERSION,
        })?;
        let hello = conn.collect_until(timeout, terminal_hello, "waiting for hello ack")?;
        let hello_ack = hello
            .last()
            .map(|item| item.raw.clone())
            .unwrap_or(Value::Null);
        conn.hello_ack = hello_ack;
        Ok(conn)
    }

    fn send(&mut self, request: &ViewerRequest) -> Result<(), String> {
        let payload = serde_json::to_string(request)
            .map_err(|err| format!("serialize request failed: {err}"))?;
        self.writer
            .write_all(payload.as_bytes())
            .map_err(|err| format!("write request failed: {err}"))?;
        self.writer
            .write_all(b"\n")
            .map_err(|err| format!("write request delimiter failed: {err}"))?;
        self.writer
            .flush()
            .map_err(|err| format!("flush request failed: {err}"))?;
        Ok(())
    }

    fn collect_until<F>(
        &mut self,
        timeout: Duration,
        is_terminal: F,
        timeout_context: &str,
    ) -> Result<Vec<CollectedResponse>, String>
    where
        F: Fn(&ViewerResponse) -> bool,
    {
        let deadline = Instant::now() + timeout;
        let mut responses = Vec::new();
        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return Err(format!("{timeout_context}: timeout after {timeout:?}"));
            }
            let Some(response) = self.read_response(remaining)? else {
                return Err(format!("{timeout_context}: connection closed"));
            };
            let terminal = is_terminal(&response.response) || is_terminal_error(&response.response);
            responses.push(response);
            if terminal {
                return Ok(responses);
            }
        }
    }

    fn collect_for(&mut self, timeout: Duration) -> Result<Vec<CollectedResponse>, String> {
        let deadline = Instant::now() + timeout;
        let mut responses = Vec::new();
        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return Ok(responses);
            }
            match self.read_response(remaining)? {
                Some(response) => responses.push(response),
                None => return Ok(responses),
            }
        }
    }

    fn read_response(&mut self, timeout: Duration) -> Result<Option<CollectedResponse>, String> {
        self.reader
            .get_mut()
            .set_read_timeout(Some(timeout))
            .map_err(|err| format!("set_read_timeout failed: {err}"))?;
        let mut line = String::new();
        match self.reader.read_line(&mut line) {
            Ok(0) => Ok(None),
            Ok(_) => {
                let raw: Value = serde_json::from_str(line.trim_end())
                    .map_err(|err| format!("parse response json failed: {err}"))?;
                let response: ViewerResponse = serde_json::from_value(raw.clone())
                    .map_err(|err| format!("decode response failed: {err}"))?;
                Ok(Some(CollectedResponse { response, raw }))
            }
            Err(err)
                if matches!(
                    err.kind(),
                    std::io::ErrorKind::WouldBlock
                        | std::io::ErrorKind::TimedOut
                        | std::io::ErrorKind::Interrupted
                ) =>
            {
                Ok(None)
            }
            Err(err) => Err(format!("read response failed: {err}")),
        }
    }
}

struct CollectedResponse {
    response: ViewerResponse,
    raw: Value,
}

fn parse_cli(args: &mut ArgCursor) -> Result<CliConfig, String> {
    let mut addr = DEFAULT_ADDR.to_string();
    let mut client = DEFAULT_CLIENT.to_string();
    let mut timeout_ms = DEFAULT_TIMEOUT_MS;
    loop {
        match args.peek() {
            Some("--addr") => {
                args.next();
                addr = args.value("--addr")?;
            }
            Some("--client") => {
                args.next();
                client = args.value("--client")?;
            }
            Some("--timeout-ms") => {
                args.next();
                timeout_ms = parse_u64_flag(args.value("--timeout-ms")?, "--timeout-ms")?;
            }
            Some("-h") | Some("--help") => return Err(usage()),
            _ => break,
        }
    }
    let subcommand = args.next().ok_or_else(usage)?.to_ascii_lowercase();
    let command = match subcommand.as_str() {
        "keygen" => Command::Keygen,
        "snapshot" => {
            let mut player_gameplay_only = false;
            while let Some(flag) = args.peek() {
                match flag {
                    "--player-gameplay-only" => {
                        args.next();
                        player_gameplay_only = true;
                    }
                    "-h" | "--help" => return Err(usage()),
                    _ => return Err(format!("unknown snapshot flag `{flag}`")),
                }
            }
            Command::Snapshot {
                player_gameplay_only,
            }
        }
        "step" => {
            let mut count = 1usize;
            let mut request_id = None;
            let mut include_events = false;
            let mut include_metrics = false;
            while let Some(flag) = args.peek() {
                match flag {
                    "--count" => {
                        args.next();
                        count = parse_usize_flag(args.value("--count")?, "--count")?;
                    }
                    "--request-id" => {
                        args.next();
                        request_id =
                            Some(parse_u64_flag(args.value("--request-id")?, "--request-id")?);
                    }
                    "--events" => {
                        args.next();
                        include_events = true;
                    }
                    "--metrics" => {
                        args.next();
                        include_metrics = true;
                    }
                    "-h" | "--help" => return Err(usage()),
                    _ => return Err(format!("unknown step flag `{flag}`")),
                }
            }
            Command::Step {
                count,
                request_id,
                include_events,
                include_metrics,
            }
        }
        "play" => Command::Play {
            include_events: parse_bool_flag(args, "--events")?,
            include_metrics: parse_bool_flag(args, "--metrics")?,
        },
        "pause" => Command::Pause {
            include_events: parse_bool_flag(args, "--events")?,
            include_metrics: parse_bool_flag(args, "--metrics")?,
        },
        "chat" => {
            let mut agent_id = None;
            let mut player_id = None;
            let mut private_key_hex = None;
            let mut public_key_hex = None;
            let mut message = None;
            let mut intent_tick = None;
            let mut intent_seq = None;
            let mut with_snapshot = false;
            while let Some(flag) = args.peek() {
                match flag {
                    "--agent-id" => {
                        args.next();
                        agent_id = Some(args.value("--agent-id")?);
                    }
                    "--player-id" => {
                        args.next();
                        player_id = Some(args.value("--player-id")?);
                    }
                    "--private-key-hex" => {
                        args.next();
                        private_key_hex = Some(args.value("--private-key-hex")?);
                    }
                    "--public-key-hex" => {
                        args.next();
                        public_key_hex = Some(args.value("--public-key-hex")?);
                    }
                    "--message" => {
                        args.next();
                        message = Some(args.value("--message")?);
                    }
                    "--intent-tick" => {
                        args.next();
                        intent_tick = Some(parse_u64_flag(
                            args.value("--intent-tick")?,
                            "--intent-tick",
                        )?);
                    }
                    "--intent-seq" => {
                        args.next();
                        intent_seq =
                            Some(parse_u64_flag(args.value("--intent-seq")?, "--intent-seq")?);
                    }
                    "--with-snapshot" => {
                        args.next();
                        with_snapshot = true;
                    }
                    "-h" | "--help" => return Err(usage()),
                    _ => return Err(format!("unknown chat flag `{flag}`")),
                }
            }
            Command::Chat {
                agent_id: required_flag(agent_id, "--agent-id")?,
                player_id: required_flag(player_id, "--player-id")?,
                private_key_hex: required_flag(private_key_hex, "--private-key-hex")?,
                public_key_hex,
                message: required_flag(message, "--message")?,
                intent_tick,
                intent_seq,
                with_snapshot,
            }
        }
        "gameplay-action" => {
            let mut action_id = None;
            let mut target_agent_id = None;
            let mut player_id = None;
            let mut private_key_hex = None;
            let mut public_key_hex = None;
            let mut with_snapshot = false;
            while let Some(flag) = args.peek() {
                match flag {
                    "--action-id" => {
                        args.next();
                        action_id = Some(args.value("--action-id")?);
                    }
                    "--target-agent-id" => {
                        args.next();
                        target_agent_id = Some(args.value("--target-agent-id")?);
                    }
                    "--player-id" => {
                        args.next();
                        player_id = Some(args.value("--player-id")?);
                    }
                    "--private-key-hex" => {
                        args.next();
                        private_key_hex = Some(args.value("--private-key-hex")?);
                    }
                    "--public-key-hex" => {
                        args.next();
                        public_key_hex = Some(args.value("--public-key-hex")?);
                    }
                    "--with-snapshot" => {
                        args.next();
                        with_snapshot = true;
                    }
                    "-h" | "--help" => return Err(usage()),
                    _ => return Err(format!("unknown gameplay-action flag `{flag}`")),
                }
            }
            Command::GameplayAction {
                action_id: required_flag(action_id, "--action-id")?,
                target_agent_id: required_flag(target_agent_id, "--target-agent-id")?,
                player_id: required_flag(player_id, "--player-id")?,
                private_key_hex: required_flag(private_key_hex, "--private-key-hex")?,
                public_key_hex,
                with_snapshot,
            }
        }
        "prompt-apply" | "prompt-preview" => {
            let preview = subcommand == "prompt-preview";
            let mut agent_id = None;
            let mut player_id = None;
            let mut private_key_hex = None;
            let mut public_key_hex = None;
            let mut expected_version = None;
            let mut updated_by = None;
            let mut system_prompt_override = None;
            let mut short_term_goal_override = None;
            let mut long_term_goal_override = None;
            let mut with_snapshot = false;
            while let Some(flag) = args.peek() {
                match flag {
                    "--agent-id" => {
                        args.next();
                        agent_id = Some(args.value("--agent-id")?);
                    }
                    "--player-id" => {
                        args.next();
                        player_id = Some(args.value("--player-id")?);
                    }
                    "--private-key-hex" => {
                        args.next();
                        private_key_hex = Some(args.value("--private-key-hex")?);
                    }
                    "--public-key-hex" => {
                        args.next();
                        public_key_hex = Some(args.value("--public-key-hex")?);
                    }
                    "--expected-version" => {
                        args.next();
                        expected_version = Some(parse_u64_flag(
                            args.value("--expected-version")?,
                            "--expected-version",
                        )?);
                    }
                    "--updated-by" => {
                        args.next();
                        updated_by = Some(args.value("--updated-by")?);
                    }
                    "--system-prompt" => {
                        args.next();
                        system_prompt_override = Some(Some(args.value("--system-prompt")?));
                    }
                    "--clear-system-prompt" => {
                        args.next();
                        system_prompt_override = Some(None);
                    }
                    "--short-term-goal" => {
                        args.next();
                        short_term_goal_override = Some(Some(args.value("--short-term-goal")?));
                    }
                    "--clear-short-term-goal" => {
                        args.next();
                        short_term_goal_override = Some(None);
                    }
                    "--long-term-goal" => {
                        args.next();
                        long_term_goal_override = Some(Some(args.value("--long-term-goal")?));
                    }
                    "--clear-long-term-goal" => {
                        args.next();
                        long_term_goal_override = Some(None);
                    }
                    "--with-snapshot" => {
                        args.next();
                        with_snapshot = true;
                    }
                    "-h" | "--help" => return Err(usage()),
                    _ => return Err(format!("unknown prompt flag `{flag}`")),
                }
            }
            Command::PromptApply {
                agent_id: required_flag(agent_id, "--agent-id")?,
                player_id: required_flag(player_id, "--player-id")?,
                private_key_hex: required_flag(private_key_hex, "--private-key-hex")?,
                public_key_hex,
                expected_version,
                updated_by,
                system_prompt_override,
                short_term_goal_override,
                long_term_goal_override,
                preview,
                with_snapshot,
            }
        }
        "prompt-rollback" => {
            let mut agent_id = None;
            let mut player_id = None;
            let mut private_key_hex = None;
            let mut public_key_hex = None;
            let mut to_version = None;
            let mut expected_version = None;
            let mut updated_by = None;
            let mut with_snapshot = false;
            while let Some(flag) = args.peek() {
                match flag {
                    "--agent-id" => {
                        args.next();
                        agent_id = Some(args.value("--agent-id")?);
                    }
                    "--player-id" => {
                        args.next();
                        player_id = Some(args.value("--player-id")?);
                    }
                    "--private-key-hex" => {
                        args.next();
                        private_key_hex = Some(args.value("--private-key-hex")?);
                    }
                    "--public-key-hex" => {
                        args.next();
                        public_key_hex = Some(args.value("--public-key-hex")?);
                    }
                    "--to-version" => {
                        args.next();
                        to_version =
                            Some(parse_u64_flag(args.value("--to-version")?, "--to-version")?);
                    }
                    "--expected-version" => {
                        args.next();
                        expected_version = Some(parse_u64_flag(
                            args.value("--expected-version")?,
                            "--expected-version",
                        )?);
                    }
                    "--updated-by" => {
                        args.next();
                        updated_by = Some(args.value("--updated-by")?);
                    }
                    "--with-snapshot" => {
                        args.next();
                        with_snapshot = true;
                    }
                    "-h" | "--help" => return Err(usage()),
                    _ => return Err(format!("unknown prompt-rollback flag `{flag}`")),
                }
            }
            Command::PromptRollback {
                agent_id: required_flag(agent_id, "--agent-id")?,
                player_id: required_flag(player_id, "--player-id")?,
                private_key_hex: required_flag(private_key_hex, "--private-key-hex")?,
                public_key_hex,
                to_version: required_flag(to_version, "--to-version")?,
                expected_version,
                updated_by,
                with_snapshot,
            }
        }
        "reconnect-sync" => {
            let mut player_id = None;
            let mut session_pubkey = None;
            let mut last_known_log_cursor = None;
            let mut expected_reorg_epoch = None;
            let mut with_snapshot = false;
            while let Some(flag) = args.peek() {
                match flag {
                    "--player-id" => {
                        args.next();
                        player_id = Some(args.value("--player-id")?);
                    }
                    "--session-pubkey" => {
                        args.next();
                        session_pubkey = Some(args.value("--session-pubkey")?);
                    }
                    "--last-known-log-cursor" => {
                        args.next();
                        last_known_log_cursor = Some(parse_u64_flag(
                            args.value("--last-known-log-cursor")?,
                            "--last-known-log-cursor",
                        )?);
                    }
                    "--expected-reorg-epoch" => {
                        args.next();
                        expected_reorg_epoch = Some(parse_u64_flag(
                            args.value("--expected-reorg-epoch")?,
                            "--expected-reorg-epoch",
                        )?);
                    }
                    "--with-snapshot" => {
                        args.next();
                        with_snapshot = true;
                    }
                    "-h" | "--help" => return Err(usage()),
                    _ => return Err(format!("unknown reconnect-sync flag `{flag}`")),
                }
            }
            Command::ReconnectSync {
                player_id: required_flag(player_id, "--player-id")?,
                session_pubkey,
                last_known_log_cursor,
                expected_reorg_epoch,
                with_snapshot,
            }
        }
        "rotate-session" => {
            let mut player_id = None;
            let mut old_session_pubkey = None;
            let mut new_session_pubkey = None;
            let mut rotate_reason = None;
            while let Some(flag) = args.peek() {
                match flag {
                    "--player-id" => {
                        args.next();
                        player_id = Some(args.value("--player-id")?);
                    }
                    "--old-session-pubkey" => {
                        args.next();
                        old_session_pubkey = Some(args.value("--old-session-pubkey")?);
                    }
                    "--new-session-pubkey" => {
                        args.next();
                        new_session_pubkey = Some(args.value("--new-session-pubkey")?);
                    }
                    "--rotate-reason" => {
                        args.next();
                        rotate_reason = Some(args.value("--rotate-reason")?);
                    }
                    "-h" | "--help" => return Err(usage()),
                    _ => return Err(format!("unknown rotate-session flag `{flag}`")),
                }
            }
            Command::RotateSession {
                player_id: required_flag(player_id, "--player-id")?,
                old_session_pubkey: required_flag(old_session_pubkey, "--old-session-pubkey")?,
                new_session_pubkey: required_flag(new_session_pubkey, "--new-session-pubkey")?,
                rotate_reason: required_flag(rotate_reason, "--rotate-reason")?,
            }
        }
        "revoke-session" => {
            let mut player_id = None;
            let mut session_pubkey = None;
            let mut revoke_reason = None;
            while let Some(flag) = args.peek() {
                match flag {
                    "--player-id" => {
                        args.next();
                        player_id = Some(args.value("--player-id")?);
                    }
                    "--session-pubkey" => {
                        args.next();
                        session_pubkey = Some(args.value("--session-pubkey")?);
                    }
                    "--revoke-reason" => {
                        args.next();
                        revoke_reason = Some(args.value("--revoke-reason")?);
                    }
                    "-h" | "--help" => return Err(usage()),
                    _ => return Err(format!("unknown revoke-session flag `{flag}`")),
                }
            }
            Command::RevokeSession {
                player_id: required_flag(player_id, "--player-id")?,
                session_pubkey,
                revoke_reason: required_flag(revoke_reason, "--revoke-reason")?,
            }
        }
        _ => return Err(format!("unknown subcommand `{subcommand}`\n\n{}", usage())),
    };

    Ok(CliConfig {
        addr,
        client,
        timeout_ms,
        command,
    })
}

fn usage() -> String {
    format!(
        "Usage: oasis7_pure_api_client [global options] <command> [command options]\n\n\
Global options:\n\
  --addr <host:port>         Live viewer TCP address (default: {DEFAULT_ADDR})\n\
  --client <name>            Client name for hello handshake (default: {DEFAULT_CLIENT})\n\
  --timeout-ms <n>           Wait timeout per command in milliseconds (default: {DEFAULT_TIMEOUT_MS})\n\n\
Commands:\n\
  keygen\n\
    Generate one Ed25519 player keypair for pure API play.\n\n\
  snapshot [--player-gameplay-only]\n\
    Request one live snapshot. With --player-gameplay-only, print snapshot.player_gameplay only.\n\n\
  step [--count <n>] [--request-id <n>] [--events] [--metrics]\n\
    Send live_control.step and collect control ack plus subscribed responses.\n\n\
  play [--events] [--metrics]\n\
  pause [--events] [--metrics]\n\
    Send live_control.play / live_control.pause and drain responses until timeout.\n\n\
  chat --agent-id <id> --player-id <id> --private-key-hex <hex> --message <text>\n\
       [--public-key-hex <hex>] [--intent-tick <n>] [--intent-seq <n>] [--with-snapshot]\n\
    Send one signed agent_chat request.\n\n\
  gameplay-action --action-id <id> --target-agent-id <id> --player-id <id>\n\
       --private-key-hex <hex> [--public-key-hex <hex>] [--with-snapshot]\n\
    Send one signed canonical gameplay_action request.\n\n\
  prompt-apply --agent-id <id> --player-id <id> --private-key-hex <hex>\n\
       [--public-key-hex <hex>] [--expected-version <n>] [--updated-by <name>]\n\
       [--system-prompt <text>|--clear-system-prompt]\n\
       [--short-term-goal <text>|--clear-short-term-goal]\n\
       [--long-term-goal <text>|--clear-long-term-goal] [--with-snapshot]\n\
    Send one signed prompt_control apply request.\n\n\
  prompt-preview <same flags as prompt-apply>\n\
    Send one signed prompt_control preview request.\n\n\
  prompt-rollback --agent-id <id> --player-id <id> --private-key-hex <hex> --to-version <n>\n\
       [--public-key-hex <hex>] [--expected-version <n>] [--updated-by <name>] [--with-snapshot]\n\
    Send one signed prompt_control rollback request.\n\n\
  reconnect-sync --player-id <id> [--session-pubkey <hex>] [--last-known-log-cursor <n>]\n\
       [--expected-reorg-epoch <n>] [--with-snapshot]\n\
    Request authoritative reconnect sync / stage recovery data.\n\n\
  rotate-session --player-id <id> --old-session-pubkey <hex> --new-session-pubkey <hex>\n\
       --rotate-reason <text>\n\
    Rotate the active session key for one player.\n\n\
  revoke-session --player-id <id> [--session-pubkey <hex>] --revoke-reason <text>\n\
    Revoke one player's session key.\n\n\
Examples:\n\
  oasis7_pure_api_client snapshot --player-gameplay-only\n\
  oasis7_pure_api_client step --count 8 --events\n\
  oasis7_pure_api_client chat --agent-id agent-0 --player-id player-1 \\\n\
    --private-key-hex <hex> --message 'build the first stable line' --with-snapshot\n\
  oasis7_pure_api_client gameplay-action --action-id build_factory_smelter_mk1 \\\n\
    --target-agent-id runtime-agent-0 --player-id player-1 --private-key-hex <hex> --with-snapshot\n\
  oasis7_pure_api_client prompt-apply --agent-id agent-0 --player-id player-1 \\\n\
    --private-key-hex <hex> --short-term-goal 'turn iron into output'\n\
  oasis7_pure_api_client reconnect-sync --player-id player-1 --session-pubkey <hex> --with-snapshot"
    )
}

struct ArgCursor {
    args: Vec<String>,
    pos: usize,
}

impl ArgCursor {
    fn from_env() -> Result<Self, String> {
        let args = std::env::args().skip(1).collect::<Vec<_>>();
        if args.is_empty() {
            return Err(usage());
        }
        Ok(Self { args, pos: 0 })
    }

    fn next(&mut self) -> Option<String> {
        let value = self.args.get(self.pos).cloned();
        if value.is_some() {
            self.pos += 1;
        }
        value
    }

    fn peek(&self) -> Option<&str> {
        self.args.get(self.pos).map(String::as_str)
    }

    fn value(&mut self, flag: &str) -> Result<String, String> {
        self.next()
            .ok_or_else(|| format!("missing value for `{flag}`"))
    }

    fn has_more(&self) -> bool {
        self.pos < self.args.len()
    }

    fn remaining(&self) -> &[String] {
        &self.args[self.pos..]
    }
}
