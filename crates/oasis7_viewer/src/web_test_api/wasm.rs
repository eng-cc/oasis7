use super::*;

#[cfg(target_arch = "wasm32")]
pub(super) fn setup_web_test_api(world: &mut World) {
    let Some(window) = web_sys::window() else {
        return;
    };
    if !web_test_api_enabled(&window) {
        return;
    }

    let api = Object::new();

    let run_steps = Closure::wrap(Box::new(move |payload: JsValue| {
        let Some(command) = parse_run_steps_command(&payload) else {
            log_api_warning(
                "web test api: runSteps ignored (payload must be non-empty step string or count)",
            );
            return;
        };
        push_command(command);
    }) as Box<dyn FnMut(JsValue)>);
    let _ = JsReflect::set(
        &api,
        &JsValue::from_str("runSteps"),
        run_steps.as_ref().unchecked_ref(),
    );

    let set_mode = Closure::wrap(Box::new(move |payload: JsValue| {
        let Some(raw_mode) = parse_string_payload(&payload) else {
            log_api_warning("web test api: setMode ignored (mode must be non-empty string)");
            return;
        };
        let Some(mode) = parse_automation_mode(&raw_mode) else {
            log_api_warning("web test api: setMode ignored (invalid mode)");
            return;
        };
        push_command(WebTestApiCommand::EnqueueSteps(vec![
            crate::viewer_automation::ViewerAutomationStep::SetMode(mode),
        ]));
    }) as Box<dyn FnMut(JsValue)>);
    let _ = JsReflect::set(
        &api,
        &JsValue::from_str("setMode"),
        set_mode.as_ref().unchecked_ref(),
    );

    let focus = Closure::wrap(Box::new(move |payload: JsValue| {
        let Some(raw_target) = parse_string_payload(&payload) else {
            log_api_warning("web test api: focus ignored (target must be non-empty string)");
            return;
        };
        let Some(target) = parse_automation_target(&raw_target) else {
            log_api_warning("web test api: focus ignored (invalid target)");
            return;
        };
        push_command(WebTestApiCommand::EnqueueSteps(vec![
            crate::viewer_automation::ViewerAutomationStep::Focus(target),
        ]));
    }) as Box<dyn FnMut(JsValue)>);
    let _ = JsReflect::set(
        &api,
        &JsValue::from_str("focus"),
        focus.as_ref().unchecked_ref(),
    );

    let select = Closure::wrap(Box::new(move |payload: JsValue| {
        let Some(raw_target) = parse_string_payload(&payload) else {
            log_api_warning("web test api: select ignored (target must be non-empty string)");
            return;
        };
        let Some(target) = parse_automation_target(&raw_target) else {
            log_api_warning("web test api: select ignored (invalid target)");
            return;
        };
        push_command(WebTestApiCommand::EnqueueSteps(vec![
            crate::viewer_automation::ViewerAutomationStep::Select(target),
        ]));
    }) as Box<dyn FnMut(JsValue)>);
    let _ = JsReflect::set(
        &api,
        &JsValue::from_str("select"),
        select.as_ref().unchecked_ref(),
    );

    let describe_controls = Closure::wrap(Box::new(move || -> JsValue {
        build_control_catalog_js_value(current_web_test_api_control_profile())
    }) as Box<dyn FnMut() -> JsValue>);
    let _ = JsReflect::set(
        &api,
        &JsValue::from_str("describeControls"),
        describe_controls.as_ref().unchecked_ref(),
    );

    let fill_control_example = Closure::wrap(Box::new(move |action: JsValue| -> JsValue {
        let Some(action) = parse_control_example_action(&action) else {
            log_api_warning("web test api: fillControlExample ignored (invalid action)");
            return JsValue::NULL;
        };
        let profile = current_web_test_api_control_profile();
        let object = Object::new();
        let _ = JsReflect::set(
            &object,
            &JsValue::from_str("action"),
            &JsValue::from_str(action.as_str()),
        );
        let _ = JsReflect::set(
            &object,
            &JsValue::from_str("payload"),
            &control_payload_example(action.as_str()),
        );
        let _ = JsReflect::set(
            &object,
            &JsValue::from_str("supported"),
            &JsValue::from_bool(control_action_supported_for_profile(
                action.as_str(),
                profile,
            )),
        );
        let _ = JsReflect::set(
            &object,
            &JsValue::from_str("controlProfile"),
            &viewer_control_profile_name(profile)
                .map(JsValue::from_str)
                .unwrap_or(JsValue::NULL),
        );
        let _ = JsReflect::set(
            &object,
            &JsValue::from_str("reason"),
            &unsupported_action_reason(action.as_str(), profile, false)
                .map(|value| JsValue::from_str(value.as_str()))
                .unwrap_or(JsValue::NULL),
        );
        let _ = JsReflect::set(
            &object,
            &JsValue::from_str("hint"),
            &unsupported_action_hint(action.as_str(), profile, false)
                .map(|value| JsValue::from_str(value.as_str()))
                .unwrap_or(JsValue::NULL),
        );
        JsValue::from(object)
    }) as Box<dyn FnMut(JsValue) -> JsValue>);
    let _ = JsReflect::set(
        &api,
        &JsValue::from_str("fillControlExample"),
        fill_control_example.as_ref().unchecked_ref(),
    );

    let send_control = Closure::wrap(
        Box::new(move |action: JsValue, payload: JsValue| -> JsValue {
            let Some(raw_action) = parse_string_payload(&action) else {
                let feedback = build_control_feedback(
                    "<empty>".to_string(),
                    false,
                    None,
                    Some("action must be a non-empty string".to_string()),
                    Some(control_action_hint(
                        "unknown",
                        false,
                        current_web_test_api_control_profile(),
                    )),
                    "rejected before enqueue".to_string(),
                    false,
                );
                update_last_control_feedback(feedback.clone());
                log_api_warning(
                    "web test api: sendControl ignored (action must be non-empty string)",
                );
                return build_control_feedback_js_value(&feedback);
            };

            let action = normalize_control_action(raw_action.as_str());
            let control_profile = current_web_test_api_control_profile();
            if !WEB_TEST_API_CONTROL_ACTIONS
                .iter()
                .any(|candidate| *candidate == action.as_str())
            {
                let feedback = build_control_feedback(
                    action.clone(),
                    false,
                    None,
                    Some(format!("unsupported action: {}", action)),
                    Some(control_action_hint("unknown", false, control_profile)),
                    "rejected before enqueue".to_string(),
                    false,
                );
                update_last_control_feedback(feedback.clone());
                let warning =
                    format!("web test api: sendControl ignored (unsupported action: {action})");
                log_api_warning(warning.as_str());
                return build_control_feedback_js_value(&feedback);
            }

            if !control_action_supported_for_profile(action.as_str(), control_profile) {
                let feedback = build_control_feedback(
                    action.clone(),
                    false,
                    None,
                    unsupported_action_reason(action.as_str(), control_profile, false),
                    unsupported_action_hint(action.as_str(), control_profile, false),
                    "rejected before enqueue".to_string(),
                    false,
                );
                update_last_control_feedback(feedback.clone());
                let warning = format!(
                    "web test api: sendControl ignored ({})",
                    feedback
                        .reason
                        .as_deref()
                        .unwrap_or("unsupported control for current profile")
                );
                log_api_warning(warning.as_str());
                return build_control_feedback_js_value(&feedback);
            }

            let Some(control) = parse_control_action(action.as_str(), &payload) else {
                let reason = match action.as_str() {
                    "step" => "step requires numeric payload.count >= 1",
                    "seek" => "seek requires numeric payload.tick >= 0",
                    _ => "invalid payload for control action",
                };
                let feedback = build_control_feedback(
                    action.clone(),
                    false,
                    None,
                    Some(reason.to_string()),
                    Some(control_action_hint(action.as_str(), false, control_profile)),
                    "rejected before enqueue".to_string(),
                    false,
                );
                update_last_control_feedback(feedback.clone());
                let warning = format!("web test api: sendControl ignored ({reason})");
                log_api_warning(warning.as_str());
                return build_control_feedback_js_value(&feedback);
            };
            let parsed_label = parse_control_action_label(&control);
            let mut feedback = build_control_feedback(
                action,
                true,
                Some(parsed_label),
                None,
                Some("queued, check getState().lastControlFeedback for world delta".to_string()),
                "queued control request".to_string(),
                true,
            );
            let request_id = if matches!(control, ViewerControl::Play | ViewerControl::Step { .. })
            {
                Some(feedback.id)
            } else {
                None
            };
            feedback.request_id = request_id;
            let feedback_id = feedback.id;
            update_last_control_feedback(feedback.clone());
            push_command(WebTestApiCommand::SendControl {
                control,
                feedback_id,
                request_id,
            });
            build_control_feedback_js_value(&feedback)
        }) as Box<dyn FnMut(JsValue, JsValue) -> JsValue>,
    );
    let _ = JsReflect::set(
        &api,
        &JsValue::from_str("sendControl"),
        send_control.as_ref().unchecked_ref(),
    );

    let report_fatal_error = Closure::wrap(Box::new(move |message: JsValue, source: JsValue| {
        let message = parse_string_payload(&message)
            .or_else(|| message.as_string())
            .unwrap_or_else(|| "unknown runtime error".to_string());
        let source = parse_string_payload(&source)
            .or_else(|| source.as_string())
            .unwrap_or_else(|| "runtime".to_string());
        record_runtime_fatal_error(source.as_str(), message.as_str());
    }) as Box<dyn FnMut(JsValue, JsValue)>);
    let _ = JsReflect::set(
        &api,
        &JsValue::from_str("reportFatalError"),
        report_fatal_error.as_ref().unchecked_ref(),
    );

    let get_state = Closure::wrap(Box::new(move || -> JsValue {
        WEB_TEST_API_STATE_SNAPSHOT.with(|slot| build_state_js_value(&slot.borrow()))
    }) as Box<dyn FnMut() -> JsValue>);
    let _ = JsReflect::set(
        &api,
        &JsValue::from_str("getState"),
        get_state.as_ref().unchecked_ref(),
    );

    let _ = JsReflect::set(&window, &JsValue::from_str(TEST_API_GLOBAL_NAME), &api);
    let runtime_diag_installer = install_runtime_diagnostic_hooks();

    world.insert_non_send_resource(WebTestApiBindings {
        _api: api,
        _run_steps: run_steps,
        _set_mode: set_mode,
        _focus: focus,
        _select: select,
        _describe_controls: describe_controls,
        _fill_control_example: fill_control_example,
        _send_control: send_control,
        _report_fatal_error: report_fatal_error,
        _get_state: get_state,
        _runtime_diag_installer: runtime_diag_installer,
    });
}

#[cfg(not(target_arch = "wasm32"))]
pub(super) fn setup_web_test_api(_world: &mut World) {}

#[cfg(target_arch = "wasm32")]
pub(super) fn consume_web_test_api_commands(
    mut automation_state: ResMut<ViewerAutomationState>,
    _state: Option<Res<ViewerState>>,
    client: Option<Res<ViewerClient>>,
    control_profile: Option<Res<ViewerControlProfileState>>,
) {
    let mut commands = Vec::new();
    WEB_TEST_API_COMMAND_QUEUE.with(|queue| {
        let mut queue = queue.borrow_mut();
        while let Some(command) = queue.pop_front() {
            commands.push(command);
        }
    });

    for command in commands {
        match command {
            WebTestApiCommand::EnqueueSteps(steps) => {
                enqueue_runtime_steps(&mut automation_state, steps);
            }
            WebTestApiCommand::SendControl {
                control,
                feedback_id,
                request_id,
            } => {
                let Some(client) = client.as_deref() else {
                    mutate_last_control_feedback(feedback_id, |feedback| {
                        feedback.accepted = false;
                        feedback.enqueued = false;
                        feedback.stage = CONTROL_STAGE_BLOCKED.to_string();
                        feedback.reason = Some("viewer client is not available".to_string());
                        feedback.hint = Some("reconnect then retry sendControl".to_string());
                        feedback.effect = "dropped before dispatch".to_string();
                        feedback.awaiting_effect = false;
                    });
                    continue;
                };
                let dispatch_result = dispatch_viewer_control(
                    client,
                    control_profile.as_deref(),
                    control,
                    request_id,
                );
                mutate_last_control_feedback(feedback_id, |feedback| match dispatch_result {
                    ViewerControlDispatchResult::Sent => {
                        feedback.stage = CONTROL_STAGE_EXECUTING.to_string();
                        feedback.enqueued = true;
                        feedback.hint =
                            Some("dispatch accepted, waiting for world delta".to_string());
                    }
                    ViewerControlDispatchResult::UnsupportedForProfile {
                        profile,
                        ref control,
                    } => {
                        feedback.accepted = false;
                        feedback.enqueued = false;
                        feedback.stage = CONTROL_STAGE_BLOCKED.to_string();
                        feedback.reason = unsupported_control_reason(profile, control, false);
                        feedback.hint = unsupported_control_hint(profile, control, false);
                        feedback.effect = "dropped before dispatch".to_string();
                        feedback.awaiting_effect = false;
                    }
                    ViewerControlDispatchResult::ClientChannelSendFailed => {
                        feedback.accepted = false;
                        feedback.enqueued = false;
                        feedback.stage = CONTROL_STAGE_BLOCKED.to_string();
                        feedback.reason = Some("viewer client channel send failed".to_string());
                        feedback.hint = Some("retry control after reconnect".to_string());
                        feedback.effect = "dropped before dispatch".to_string();
                        feedback.awaiting_effect = false;
                    }
                });
            }
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub(super) fn consume_web_test_api_commands(
    _automation_state: ResMut<ViewerAutomationState>,
    _state: Option<Res<ViewerState>>,
    _client: Option<Res<ViewerClient>>,
    _control_profile: Option<Res<ViewerControlProfileState>>,
) {
}

#[cfg(target_arch = "wasm32")]
pub(super) fn publish_web_test_api_state(
    state: Res<ViewerState>,
    selection: Res<ViewerSelection>,
    camera_mode: Res<ViewerCameraMode>,
    cameras: Query<(&OrbitCamera, &Projection), With<Viewer3dCamera>>,
    _client: Option<Res<ViewerClient>>,
    control_profile: Option<Res<ViewerControlProfileState>>,
) {
    WEB_TEST_API_STATE_SNAPSHOT.with(|slot| {
        let mut snapshot = slot.borrow_mut();
        let runtime_fatal_error = current_runtime_fatal_error();
        snapshot.connection_status = if runtime_fatal_error.is_some() {
            "error"
        } else {
            match &state.status {
                ConnectionStatus::Connecting => "connecting",
                ConnectionStatus::Connected => "connected",
                ConnectionStatus::Error(_) => "error",
            }
        };
        snapshot.control_profile = control_profile.as_deref().and_then(|state| state.profile);
        let snapshot_tick = state
            .snapshot
            .as_ref()
            .map(|snapshot| snapshot.time)
            .unwrap_or(0);
        let metrics_tick = state
            .metrics
            .as_ref()
            .map(|metrics| metrics.total_ticks)
            .unwrap_or(0);
        snapshot.logical_time = snapshot_tick.max(metrics_tick);
        snapshot.event_seq = state
            .events
            .iter()
            .map(|event| event.id)
            .max()
            .unwrap_or(snapshot.event_seq);
        snapshot.event_count = state.events.len();
        snapshot.trace_count = state.decision_traces.len();
        snapshot.selected_kind = selection
            .current
            .as_ref()
            .map(|info| match info.kind {
                SelectionKind::Agent => "agent",
                SelectionKind::Location => "location",
                SelectionKind::Fragment => "fragment",
                SelectionKind::Asset => "asset",
                SelectionKind::PowerPlant => "power_plant",
                SelectionKind::Chunk => "chunk",
            })
            .map(str::to_string);
        snapshot.selected_id = selection.current.as_ref().map(|info| info.id.clone());

        let next_error = runtime_fatal_error.or_else(|| match &state.status {
            ConnectionStatus::Error(message) => Some(message.clone()),
            _ => None,
        });
        if snapshot.last_error.as_deref() != next_error.as_deref() {
            if next_error.is_some() {
                snapshot.error_count = snapshot.error_count.saturating_add(1);
            }
        }
        snapshot.last_error = next_error;

        snapshot.camera_mode = match *camera_mode {
            ViewerCameraMode::TwoD => "2d",
            ViewerCameraMode::ThreeD => "3d",
        };

        if let Ok((orbit, projection)) = cameras.single() {
            snapshot.camera_radius = orbit.radius as f64;
            snapshot.camera_ortho_scale = match projection {
                Projection::Orthographic(ortho) => ortho.scale as f64,
                _ => 0.0,
            };
        } else {
            snapshot.camera_radius = 0.0;
            snapshot.camera_ortho_scale = 0.0;
        }

        let connection_ready = matches!(state.status, ConnectionStatus::Connected);
        let control_profile = snapshot.control_profile;
        let latest_logical_time = snapshot.logical_time;
        let latest_event_seq = snapshot.event_seq;
        let latest_trace_count = snapshot.trace_count;
        if let Some(feedback) = snapshot.last_control_feedback.as_mut() {
            if feedback.awaiting_effect {
                let mut completion_ack_applied = false;
                if let Some(request_id) = feedback.request_id {
                    if let Some(ack) = take_control_completion_ack(request_id) {
                        feedback.delta_logical_time = ack.delta_logical_time;
                        feedback.delta_event_seq = ack.delta_event_seq;
                        feedback.delta_trace_count =
                            latest_trace_count.saturating_sub(feedback.baseline_trace_count);
                        feedback.no_progress_frames = 0;
                        match ack.status {
                            ControlCompletionStatus::Advanced => {
                                feedback.stage = CONTROL_STAGE_COMPLETED_ADVANCED.to_string();
                                feedback.reason = None;
                                feedback.hint = Some(
                                    "completion ack received: step advanced world".to_string(),
                                );
                                feedback.effect = format!(
                                    "completion ack: logicalTime +{}, eventSeq +{}",
                                    ack.delta_logical_time, ack.delta_event_seq
                                );
                            }
                            ControlCompletionStatus::TimeoutNoProgress => {
                                feedback.stage = CONTROL_STAGE_COMPLETED_NO_PROGRESS.to_string();
                                feedback.reason =
                                    Some("Cause: completion ack timeout_no_progress".to_string());
                                feedback.hint = Some(
                                    "Next: keep play running, then retry step after sync"
                                        .to_string(),
                                );
                                feedback.effect =
                                    "completion ack: timeout without observed progress"
                                        .to_string();
                            }
                            ControlCompletionStatus::Blocked => {
                                feedback.stage = CONTROL_STAGE_BLOCKED.to_string();
                                feedback.reason = ack.error_message.clone().or_else(|| {
                                    ack.error_code
                                        .as_ref()
                                        .map(|code| format!("Cause: completion ack blocked ({code})"))
                                });
                                feedback.hint = Some(
                                    "Next: restore active LLM access, then retry step/play"
                                        .to_string(),
                                );
                                feedback.effect = format!(
                                    "completion ack: blocked before runtime advance completed (logicalTime +{}, eventSeq +{})",
                                    ack.delta_logical_time, ack.delta_event_seq
                                );
                            }
                        }
                        feedback.awaiting_effect = false;
                        completion_ack_applied = true;
                    }
                }

                if !completion_ack_applied {
                    let delta_logical_time =
                        latest_logical_time.saturating_sub(feedback.baseline_logical_time);
                    let delta_event_seq =
                        latest_event_seq.saturating_sub(feedback.baseline_event_seq);
                    let delta_trace_count =
                        latest_trace_count.saturating_sub(feedback.baseline_trace_count);
                    feedback.delta_logical_time = delta_logical_time;
                    feedback.delta_event_seq = delta_event_seq;
                    feedback.delta_trace_count = delta_trace_count;
                    let world_advanced = delta_logical_time > 0 || delta_event_seq > 0;
                    let trace_advanced = delta_trace_count > 0;
                    if world_advanced {
                        feedback.stage = CONTROL_STAGE_COMPLETED_ADVANCED.to_string();
                        feedback.no_progress_frames = 0;
                        feedback.effect = format!(
                            "world advanced: logicalTime +{delta_logical_time}, eventSeq +{delta_event_seq}"
                        );
                        feedback.hint =
                            Some("input was accepted and world state advanced".to_string());
                        feedback.awaiting_effect = false;
                    } else if trace_advanced {
                        feedback.stage = CONTROL_STAGE_EXECUTING.to_string();
                        feedback.no_progress_frames = 0;
                        feedback.effect = format!(
                            "decision trace advanced: +{delta_trace_count}, waiting for world delta"
                        );
                        feedback.hint = Some(
                            "model is still processing, keep waiting for tick/event advancement"
                                .to_string(),
                        );
                    } else if connection_ready {
                        let stall_threshold = control_feedback_no_progress_threshold(feedback);
                        let hint_secs = stall_hint_secs(stall_threshold);
                        feedback.no_progress_frames = feedback.no_progress_frames.saturating_add(1);
                        if feedback.no_progress_frames >= stall_threshold {
                            feedback.stage = CONTROL_STAGE_COMPLETED_NO_PROGRESS.to_string();
                            feedback.reason = Some(format!(
                                "Cause: no world delta observed for >= {:.1}s ({} frames)",
                                hint_secs, stall_threshold
                            ));
                            feedback.hint = Some(if feedback.action == "step" {
                                "Next: click Recover: play, then retry step".to_string()
                            } else if feedback.action == "play" {
                                "Next: keep play and wait for sync, or retry play after reconnect"
                                    .to_string()
                            } else {
                                control_action_hint(
                                    feedback.action.as_str(),
                                    false,
                                    control_profile,
                                )
                            });
                            feedback.effect = "accepted without observed progress".to_string();
                            feedback.awaiting_effect = false;
                        } else {
                            feedback.stage = CONTROL_STAGE_EXECUTING.to_string();
                            feedback.effect = "queued, waiting for next world delta".to_string();
                        }
                    } else {
                        feedback.stage = CONTROL_STAGE_RECEIVED.to_string();
                        feedback.effect = "queued, waiting for world connection".to_string();
                    }
                }
            }
        }
    });
}

#[cfg(not(target_arch = "wasm32"))]
pub(super) fn publish_web_test_api_state(
    _state: Res<ViewerState>,
    _selection: Res<ViewerSelection>,
    _camera_mode: Res<ViewerCameraMode>,
    _cameras: Query<(&OrbitCamera, &Projection), With<Viewer3dCamera>>,
    _client: Option<Res<ViewerClient>>,
    _control_profile: Option<Res<ViewerControlProfileState>>,
) {
}
