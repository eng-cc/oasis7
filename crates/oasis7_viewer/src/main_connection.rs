use super::*;

pub(super) fn reconnect_backoff_secs(attempt: u32) -> f64 {
    let exponential = 2_f64.powi(attempt.saturating_sub(1).min(4) as i32);
    (RECONNECT_BACKOFF_BASE_SECS * exponential).min(RECONNECT_BACKOFF_MAX_SECS)
}

pub(super) fn reconnectable_error_signature(message: &str) -> Option<String> {
    let normalized = message.trim().to_ascii_lowercase();
    if normalized.is_empty()
        || normalized == "offline mode"
        || normalized.starts_with("agent chat error:")
    {
        return None;
    }

    if normalized.contains("websocket") {
        return Some("websocket".to_string());
    }
    if normalized.contains("connection refused") {
        return Some("connection_refused".to_string());
    }
    if normalized.contains("timed out") {
        return Some("timed_out".to_string());
    }
    if normalized.contains("connection reset") {
        return Some("connection_reset".to_string());
    }
    if normalized.contains("broken pipe") {
        return Some("broken_pipe".to_string());
    }
    if normalized.contains("disconnected") {
        return Some("disconnected".to_string());
    }
    if normalized.contains("viewer receiver poisoned") {
        return Some("receiver_poisoned".to_string());
    }

    None
}

pub(super) fn websocket_close_code(message: &str) -> Option<u16> {
    let marker = "code=";
    let start = message.find(marker)? + marker.len();
    let digits = message[start..]
        .chars()
        .take_while(|ch| ch.is_ascii_digit())
        .collect::<String>();
    if digits.is_empty() {
        return None;
    }
    digits.parse::<u16>().ok()
}

pub(super) fn friendly_connection_error(message: &str) -> String {
    let trimmed = message.trim();
    if trimmed.is_empty() {
        return "connection error".to_string();
    }

    let lowered = trimmed.to_ascii_lowercase();
    if lowered == "offline mode" || lowered.starts_with("agent chat error:") {
        return trimmed.to_string();
    }
    if lowered.starts_with("websocket closed:") {
        if let Some(code) = websocket_close_code(trimmed) {
            return format!("connection closed (code {code}), retrying...");
        }
        return "connection closed, retrying...".to_string();
    }
    if lowered.contains("connection refused") || lowered.contains("err_connection_refused") {
        return "viewer server unreachable, retrying...".to_string();
    }
    if lowered.contains("timed out") {
        return "connection timed out, retrying...".to_string();
    }
    if lowered.contains("connection reset") || lowered.contains("broken pipe") {
        return "connection interrupted, retrying...".to_string();
    }
    if lowered.contains("disconnected") {
        return "viewer disconnected, retrying...".to_string();
    }
    if lowered.contains("websocket error") {
        return "network error, retrying...".to_string();
    }
    if lowered.contains("viewer receiver poisoned") {
        return "viewer channel unavailable, retrying...".to_string();
    }

    trimmed.to_string()
}

pub(super) fn viewer_client_from_addr(addr: String) -> ViewerClient {
    let (tx, rx) = spawn_viewer_client(addr);
    #[cfg(not(target_arch = "wasm32"))]
    {
        ViewerClient {
            tx,
            rx: Mutex::new(rx),
        }
    }
    #[cfg(target_arch = "wasm32")]
    {
        ViewerClient { tx, rx }
    }
}

pub(super) fn setup_connection(mut commands: Commands, config: Res<ViewerConfig>) {
    commands.insert_resource(viewer_client_from_addr(config.addr.clone()));
    commands.insert_resource(ViewerState::default());
    commands.insert_resource(ViewerControlProfileState::default());
}

pub(super) fn setup_startup_state(
    commands: Commands,
    config: Res<OfflineConfig>,
    viewer: Res<ViewerConfig>,
) {
    if config.offline {
        setup_offline_state(commands);
    } else {
        setup_connection(commands, viewer);
    }
}

pub(super) fn setup_offline_state(mut commands: Commands) {
    commands.insert_resource(ViewerState {
        status: ConnectionStatus::Error("offline mode".to_string()),
        ..ViewerState::default()
    });
    commands.insert_resource(ViewerControlProfileState::default());
}

use std::sync::atomic::{AtomicU64, Ordering};

static VIEWER_CONTROL_REQUEST_ID: AtomicU64 = AtomicU64::new(1);

fn default_request_id_for_control(control: &ViewerControl, request_id: Option<u64>) -> Option<u64> {
    request_id.or_else(|| {
        matches!(control, ViewerControl::Play | ViewerControl::Step { .. })
            .then(|| VIEWER_CONTROL_REQUEST_ID.fetch_add(1, Ordering::Relaxed))
    })
}

fn control_request_for_profile(
    profile: Option<ViewerControlProfile>,
    control: ViewerControl,
    request_id: Option<u64>,
) -> Option<ViewerRequest> {
    let request_id = default_request_id_for_control(&control, request_id);
    match profile {
        Some(ViewerControlProfile::Playback) => Some(ViewerRequest::PlaybackControl {
            mode: oasis7::viewer::PlaybackControl::from(control),
            request_id,
        }),
        Some(ViewerControlProfile::Live) => {
            let mode = oasis7::viewer::LiveControl::try_from(control).ok()?;
            Some(ViewerRequest::LiveControl { mode, request_id })
        }
        None => Some(ViewerRequest::Control {
            mode: control,
            request_id,
        }),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum ViewerControlDispatchResult {
    Sent,
    UnsupportedForProfile {
        profile: ViewerControlProfile,
        control: ViewerControl,
    },
    ClientChannelSendFailed,
}

pub(super) fn viewer_control_profile(
    profile_state: Option<&ViewerControlProfileState>,
) -> Option<ViewerControlProfile> {
    profile_state.and_then(|state| state.profile)
}

#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
pub(super) fn viewer_control_profile_name(
    profile: Option<ViewerControlProfile>,
) -> Option<&'static str> {
    match profile {
        Some(ViewerControlProfile::Playback) => Some("playback"),
        Some(ViewerControlProfile::Live) => Some("live"),
        None => None,
    }
}

pub(super) fn viewer_control_supported_for_profile(
    profile: Option<ViewerControlProfile>,
    control: &ViewerControl,
) -> bool {
    !matches!(
        (profile, control),
        (Some(ViewerControlProfile::Live), ViewerControl::Seek { .. })
    )
}

pub(super) fn viewer_control_supported(
    profile_state: Option<&ViewerControlProfileState>,
    control: &ViewerControl,
) -> bool {
    viewer_control_supported_for_profile(viewer_control_profile(profile_state), control)
}

pub(super) fn viewer_seek_supported(profile_state: Option<&ViewerControlProfileState>) -> bool {
    viewer_control_supported(profile_state, &ViewerControl::Seek { tick: 0 })
}

pub(super) fn dispatch_viewer_control(
    client: &ViewerClient,
    profile_state: Option<&ViewerControlProfileState>,
    control: ViewerControl,
    request_id: Option<u64>,
) -> ViewerControlDispatchResult {
    let profile = viewer_control_profile(profile_state);
    if !viewer_control_supported_for_profile(profile, &control) {
        return ViewerControlDispatchResult::UnsupportedForProfile {
            profile: profile.expect("live profile required for unsupported control"),
            control,
        };
    }
    let Some(request) = control_request_for_profile(profile, control, request_id) else {
        return ViewerControlDispatchResult::ClientChannelSendFailed;
    };
    if client.tx.send(request).is_ok() {
        ViewerControlDispatchResult::Sent
    } else {
        ViewerControlDispatchResult::ClientChannelSendFailed
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub(super) fn spawn_viewer_client(
    addr: String,
) -> (Sender<ViewerRequest>, Receiver<ViewerResponse>) {
    let (tx_out, rx_out) = mpsc::channel::<ViewerRequest>();
    let (tx_in, rx_in) = mpsc::channel::<ViewerResponse>();

    thread::spawn(move || match TcpStream::connect(&addr) {
        Ok(stream) => {
            if let Err(err) = run_connection(stream, rx_out, tx_in.clone()) {
                let _ = tx_in.send(ViewerResponse::Error { message: err });
            }
        }
        Err(err) => {
            let _ = tx_in.send(ViewerResponse::Error {
                message: err.to_string(),
            });
        }
    });

    (tx_out, rx_in)
}

#[cfg(target_arch = "wasm32")]
pub(super) fn spawn_viewer_client(addr: String) -> (WasmViewerRequestTx, WasmViewerResponseRx) {
    wasm_reset_ws_queues();
    let tx_out = WasmViewerRequestTx;
    let tx_in = WasmViewerResponseTx;
    let rx_in = WasmViewerResponseRx;

    let ws_url = normalize_ws_addr(&addr);
    let socket = match WebSocket::new(&ws_url) {
        Ok(socket) => socket,
        Err(err) => {
            let _ = tx_in.send(ViewerResponse::Error {
                message: format!("websocket open failed: {err:?}"),
            });
            return (tx_out, rx_in);
        }
    };

    let open_socket = socket.clone();
    let open_tx = tx_in.clone();
    let on_open = Closure::wrap(Box::new(move |_event: Event| {
        send_request_ws(
            &open_socket,
            &ViewerRequest::Hello {
                client: "bevy_viewer_web".to_string(),
                version: VIEWER_PROTOCOL_VERSION,
            },
            &open_tx,
        );
        send_request_ws(
            &open_socket,
            &ViewerRequest::Subscribe {
                streams: vec![
                    ViewerStream::Snapshot,
                    ViewerStream::Events,
                    ViewerStream::Metrics,
                ],
                event_kinds: Vec::new(),
            },
            &open_tx,
        );
        send_request_ws(&open_socket, &ViewerRequest::RequestSnapshot, &open_tx);
    }) as Box<dyn FnMut(_)>);
    socket.set_onopen(Some(on_open.as_ref().unchecked_ref()));

    let message_tx = tx_in.clone();
    let on_message = Closure::wrap(Box::new(move |event: MessageEvent| {
        let Some(text) = event.data().as_string() else {
            let _ = message_tx.send(ViewerResponse::Error {
                message: "websocket message decode failed: non-text payload".to_string(),
            });
            return;
        };
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return;
        }
        match serde_json::from_str::<ViewerResponse>(trimmed) {
            Ok(response) => {
                let _ = message_tx.send(response);
            }
            Err(err) => {
                let _ = message_tx.send(ViewerResponse::Error {
                    message: format!("decode error: {err}"),
                });
            }
        }
    }) as Box<dyn FnMut(_)>);
    socket.set_onmessage(Some(on_message.as_ref().unchecked_ref()));

    let error_tx = tx_in.clone();
    let on_error = Closure::wrap(Box::new(move |event: Event| {
        let detail = event
            .dyn_ref::<ErrorEvent>()
            .map(|error| error.message())
            .filter(|message| !message.trim().is_empty())
            .unwrap_or_else(|| "network error".to_string());
        let _ = error_tx.send(ViewerResponse::Error {
            message: format!("websocket error: {detail}"),
        });
    }) as Box<dyn FnMut(_)>);
    socket.set_onerror(Some(on_error.as_ref().unchecked_ref()));

    let close_tx = tx_in.clone();
    let on_close = Closure::wrap(Box::new(move |event: CloseEvent| {
        let _ = close_tx.send(ViewerResponse::Error {
            message: format!(
                "websocket closed: code={} reason={}",
                event.code(),
                event.reason()
            ),
        });
    }) as Box<dyn FnMut(_)>);
    socket.set_onclose(Some(on_close.as_ref().unchecked_ref()));

    let sender_socket = socket.clone();
    let sender_tx = tx_in.clone();
    let sender_loop = Interval::new(16, move || {
        while let Ok(request) = wasm_try_recv_request() {
            send_request_ws(&sender_socket, &request, &sender_tx);
        }
    });

    WASM_WS_RUNTIME.with(|runtime| {
        *runtime.borrow_mut() = Some(WasmWsRuntime {
            _socket: socket,
            _sender_loop: sender_loop,
            _on_open: on_open,
            _on_message: on_message,
            _on_error: on_error,
            _on_close: on_close,
        });
    });

    (tx_out, rx_in)
}

#[cfg(target_arch = "wasm32")]
pub(super) fn send_request_ws(
    socket: &WebSocket,
    request: &ViewerRequest,
    tx_in: &WasmViewerResponseTx,
) {
    match serde_json::to_string(request) {
        Ok(payload) => {
            if let Err(err) = socket.send_with_str(&payload) {
                let _ = tx_in.send(ViewerResponse::Error {
                    message: format!("websocket send failed: {err:?}"),
                });
            }
        }
        Err(err) => {
            let _ = tx_in.send(ViewerResponse::Error {
                message: format!("request encode failed: {err}"),
            });
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub(super) fn run_connection(
    stream: TcpStream,
    rx_out: Receiver<ViewerRequest>,
    tx_in: Sender<ViewerResponse>,
) -> Result<(), String> {
    stream.set_nodelay(true).map_err(|err| err.to_string())?;
    let reader_stream = stream.try_clone().map_err(|err| err.to_string())?;
    let mut writer = std::io::BufWriter::new(stream);

    send_request(
        &mut writer,
        &ViewerRequest::Hello {
            client: "bevy_viewer".to_string(),
            version: VIEWER_PROTOCOL_VERSION,
        },
    )?;
    send_request(
        &mut writer,
        &ViewerRequest::Subscribe {
            streams: vec![
                ViewerStream::Snapshot,
                ViewerStream::Events,
                ViewerStream::Metrics,
            ],
            event_kinds: Vec::new(),
        },
    )?;
    send_request(&mut writer, &ViewerRequest::RequestSnapshot)?;

    let reader_tx = tx_in.clone();
    thread::spawn(move || read_responses(reader_stream, reader_tx));

    for request in rx_out {
        send_request(&mut writer, &request)?;
    }

    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
pub(super) fn read_responses(stream: TcpStream, tx_in: Sender<ViewerResponse>) {
    let mut reader = std::io::BufReader::new(stream);
    let mut line = String::new();
    loop {
        line.clear();
        match reader.read_line(&mut line) {
            Ok(0) => break,
            Ok(_) => {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }
                match serde_json::from_str::<ViewerResponse>(trimmed) {
                    Ok(response) => {
                        let _ = tx_in.send(response);
                    }
                    Err(err) => {
                        let _ = tx_in.send(ViewerResponse::Error {
                            message: format!("decode error: {err}"),
                        });
                    }
                }
            }
            Err(err) => {
                let _ = tx_in.send(ViewerResponse::Error {
                    message: err.to_string(),
                });
                break;
            }
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub(super) fn send_request(
    writer: &mut std::io::BufWriter<TcpStream>,
    request: &ViewerRequest,
) -> Result<(), String> {
    serde_json::to_writer(&mut *writer, request).map_err(|err| err.to_string())?;
    writer.write_all(b"\n").map_err(|err| err.to_string())?;
    writer.flush().map_err(|err| err.to_string())?;
    Ok(())
}

pub(super) fn poll_viewer_messages(
    mut state: ResMut<ViewerState>,
    mut control_profile: Option<ResMut<ViewerControlProfileState>>,
    config: Res<ViewerConfig>,
    client: Option<Res<ViewerClient>>,
) {
    let Some(client) = client else {
        return;
    };
    #[cfg(not(target_arch = "wasm32"))]
    let receiver = match client.rx.lock() {
        Ok(receiver) => receiver,
        Err(_) => {
            state.status =
                ConnectionStatus::Error(friendly_connection_error("viewer receiver poisoned"));
            return;
        }
    };
    #[cfg(target_arch = "wasm32")]
    let receiver = &client.rx;

    loop {
        match receiver.try_recv() {
            Ok(message) => match message {
                ViewerResponse::HelloAck {
                    control_profile: profile,
                    ..
                } => {
                    state.status = ConnectionStatus::Connected;
                    if let Some(control_profile) = control_profile.as_deref_mut() {
                        control_profile.profile = Some(profile);
                    }
                }
                ViewerResponse::Snapshot { snapshot } => {
                    state.snapshot = Some(snapshot);
                }
                ViewerResponse::Event { event } => {
                    push_event_with_window(&mut state.events, event, config.event_window);
                }
                ViewerResponse::AuthoritativeBatch { .. } => {}
                ViewerResponse::DecisionTrace { trace } => {
                    state.decision_traces.push(trace);
                    if state.decision_traces.len() > config.max_events {
                        let overflow = state.decision_traces.len() - config.max_events;
                        state.decision_traces.drain(0..overflow);
                    }
                }
                ViewerResponse::Metrics { metrics, .. } => {
                    state.metrics = Some(metrics);
                }
                ViewerResponse::ControlCompletionAck { ack } => {
                    crate::web_test_api::record_control_completion_ack(ack);
                }
                ViewerResponse::Error { message } => {
                    state.status = ConnectionStatus::Error(friendly_connection_error(&message));
                    if let Some(control_profile) = control_profile.as_deref_mut() {
                        control_profile.profile = None;
                    }
                }
                ViewerResponse::PromptControlAck { .. } => {}
                ViewerResponse::PromptControlError { .. } => {}
                ViewerResponse::AgentChatAck { .. } => {}
                ViewerResponse::GameplayActionAck { .. } => {}
                ViewerResponse::GameplayActionError { .. } => {}
                ViewerResponse::AuthoritativeChallengeAck { .. } => {}
                ViewerResponse::AuthoritativeChallengeError { .. } => {}
                ViewerResponse::AuthoritativeRecoveryAck { .. } => {}
                ViewerResponse::AuthoritativeRecoveryError { .. } => {}
                ViewerResponse::AgentChatError { .. } => {}
            },
            Err(mpsc::TryRecvError::Empty) => break,
            Err(mpsc::TryRecvError::Disconnected) => {
                if !matches!(state.status, ConnectionStatus::Error(_)) {
                    state.status =
                        ConnectionStatus::Error(friendly_connection_error("disconnected"));
                }
                if let Some(control_profile) = control_profile.as_deref_mut() {
                    control_profile.profile = None;
                }
                break;
            }
        }
    }
}

pub(super) fn attempt_viewer_reconnect(
    mut commands: Commands,
    config: Res<ViewerConfig>,
    offline: Option<Res<OfflineConfig>>,
    time: Option<Res<Time>>,
    state: Option<ResMut<ViewerState>>,
    mut control_profile: Option<ResMut<ViewerControlProfileState>>,
    mut reconnect: Local<ViewerReconnectRuntime>,
) {
    if offline.as_deref().is_some_and(|cfg| cfg.offline) {
        reconnect.reset();
        return;
    }

    let Some(mut state) = state else {
        reconnect.reset();
        return;
    };

    let ConnectionStatus::Error(message) = &state.status else {
        reconnect.reset();
        return;
    };

    let Some(signature) = reconnectable_error_signature(message) else {
        reconnect.reset();
        return;
    };

    let now = time
        .as_deref()
        .map(Time::elapsed_secs_f64)
        .unwrap_or_default();
    let is_new_error = reconnect.last_error_signature.as_deref() != Some(signature.as_str());
    if is_new_error {
        reconnect.attempt = 0;
        reconnect.next_retry_at_secs = Some(now);
        reconnect.last_error_signature = Some(signature);
    }

    let should_retry = reconnect
        .next_retry_at_secs
        .map(|next| now >= next)
        .unwrap_or(true);
    if !should_retry {
        return;
    }

    commands.insert_resource(viewer_client_from_addr(config.addr.clone()));
    state.status = ConnectionStatus::Connecting;
    if let Some(control_profile) = control_profile.as_deref_mut() {
        control_profile.profile = None;
    }
    reconnect.attempt = reconnect.attempt.saturating_add(1);
    reconnect.next_retry_at_secs = Some(now + reconnect_backoff_secs(reconnect.attempt));
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    #[test]
    fn control_request_defaults_to_compat_pre_hello_profile() {
        let request = control_request_for_profile(None, ViewerControl::Play, None)
            .expect("control request should be produced");
        match request {
            ViewerRequest::Control {
                mode: ViewerControl::Play,
                request_id,
            } => assert!(
                request_id.is_some(),
                "play should receive a default request_id"
            ),
            other => panic!("unexpected request: {other:?}"),
        }
    }

    #[test]
    fn control_request_maps_to_playback_channel_when_profile_is_playback() {
        let request = control_request_for_profile(
            Some(ViewerControlProfile::Playback),
            ViewerControl::Seek { tick: 42 },
            Some(42),
        )
        .expect("control request should be produced");
        assert_eq!(
            request,
            ViewerRequest::PlaybackControl {
                mode: oasis7::viewer::PlaybackControl::Seek { tick: 42 },
                request_id: Some(42),
            }
        );
    }

    #[test]
    fn control_request_maps_to_live_channel_without_seek() {
        let request = control_request_for_profile(
            Some(ViewerControlProfile::Live),
            ViewerControl::Step { count: 3 },
            Some(8),
        )
        .expect("control request should be produced");
        assert_eq!(
            request,
            ViewerRequest::LiveControl {
                mode: oasis7::viewer::LiveControl::Step { count: 3 },
                request_id: Some(8),
            }
        );
    }

    #[test]
    fn control_request_rejects_seek_in_live_profile() {
        let request = control_request_for_profile(
            Some(ViewerControlProfile::Live),
            ViewerControl::Seek { tick: 9 },
            Some(9),
        );
        assert_eq!(request, None);
    }

    #[test]
    fn dispatch_viewer_control_reports_live_seek_as_unsupported() {
        let (tx, _rx) = mpsc::channel();
        let (_response_tx, response_rx) = mpsc::channel();
        let client = ViewerClient {
            tx,
            rx: Mutex::new(response_rx),
        };
        let profile = ViewerControlProfileState {
            profile: Some(ViewerControlProfile::Live),
        };

        let result = dispatch_viewer_control(
            &client,
            Some(&profile),
            ViewerControl::Seek { tick: 9 },
            Some(9),
        );

        assert_eq!(
            result,
            ViewerControlDispatchResult::UnsupportedForProfile {
                profile: ViewerControlProfile::Live,
                control: ViewerControl::Seek { tick: 9 },
            }
        );
    }

    #[test]
    fn dispatch_viewer_control_sends_playback_seek_to_profile_channel() {
        let (tx, rx) = mpsc::channel();
        let (_response_tx, response_rx) = mpsc::channel();
        let client = ViewerClient {
            tx,
            rx: Mutex::new(response_rx),
        };
        let profile = ViewerControlProfileState {
            profile: Some(ViewerControlProfile::Playback),
        };

        let result = dispatch_viewer_control(
            &client,
            Some(&profile),
            ViewerControl::Seek { tick: 42 },
            Some(7),
        );

        assert_eq!(result, ViewerControlDispatchResult::Sent);
        assert_eq!(
            rx.recv().expect("request should be sent"),
            ViewerRequest::PlaybackControl {
                mode: oasis7::viewer::PlaybackControl::Seek { tick: 42 },
                request_id: Some(7),
            }
        );
    }

    #[test]
    fn viewer_seek_supported_blocks_live_profile_only() {
        assert!(viewer_seek_supported(None));
        assert!(viewer_seek_supported(Some(&ViewerControlProfileState {
            profile: Some(ViewerControlProfile::Playback),
        })));
        assert!(!viewer_seek_supported(Some(&ViewerControlProfileState {
            profile: Some(ViewerControlProfile::Live),
        })));
    }
}
