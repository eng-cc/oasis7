use super::super::egui_right_panel_player_micro_loop::{
    build_player_micro_loop_snapshot, format_due_timer_line, PlayerMicroLoopSnapshot,
    PlayerMicroLoopTone, PlayerNoProgressDiagnosis,
};
use super::*;
use std::collections::HashMap;

pub(crate) fn resolve_selected_location_id_for_minimap(
    selection: &ViewerSelection,
    agent_locations: &HashMap<String, String>,
) -> Option<String> {
    let current = selection.current.as_ref()?;
    match current.kind {
        crate::SelectionKind::Location => Some(current.id.clone()),
        crate::SelectionKind::Agent => agent_locations.get(current.id.as_str()).cloned(),
        _ => None,
    }
}

pub(crate) fn build_player_minimap_points(
    raw_points: &[(String, f32, f32)],
    selected_location_id: Option<&str>,
) -> Vec<PlayerMiniMapPoint> {
    if raw_points.is_empty() {
        return Vec::new();
    }

    let mut sorted_points = raw_points.to_vec();
    sorted_points.sort_by(|left, right| left.0.cmp(&right.0));

    let mut min_x = f32::INFINITY;
    let mut max_x = f32::NEG_INFINITY;
    let mut min_z = f32::INFINITY;
    let mut max_z = f32::NEG_INFINITY;
    for (_, x, z) in &sorted_points {
        min_x = min_x.min(*x);
        max_x = max_x.max(*x);
        min_z = min_z.min(*z);
        max_z = max_z.max(*z);
    }

    let span_x = (max_x - min_x).max(1.0);
    let span_z = (max_z - min_z).max(1.0);
    sorted_points
        .into_iter()
        .map(|(id, x, z)| PlayerMiniMapPoint {
            x: ((x - min_x) / span_x).clamp(0.0, 1.0),
            y: (1.0 - (z - min_z) / span_z).clamp(0.0, 1.0),
            selected: selected_location_id == Some(id.as_str()),
        })
        .collect()
}

fn build_player_minimap_snapshot(
    state: &crate::ViewerState,
    selection: &ViewerSelection,
) -> Vec<PlayerMiniMapPoint> {
    let Some(snapshot) = state.snapshot.as_ref() else {
        return Vec::new();
    };
    let agent_locations = snapshot
        .model
        .agents
        .iter()
        .map(|(agent_id, agent)| (agent_id.clone(), agent.location_id.clone()))
        .collect::<HashMap<_, _>>();
    let selected_location_id =
        resolve_selected_location_id_for_minimap(selection, &agent_locations);
    let raw_points = snapshot
        .model
        .locations
        .iter()
        .map(|(location_id, location)| {
            (
                location_id.clone(),
                location.pos.x_cm as f32,
                location.pos.z_cm as f32,
            )
        })
        .collect::<Vec<_>>();
    build_player_minimap_points(&raw_points, selected_location_id.as_deref())
}

fn render_player_minimap_card(
    context: &egui::Context,
    points: &[PlayerMiniMapPoint],
    locale: crate::i18n::UiLocale,
    now_secs: f64,
) {
    let pulse = ((now_secs * 2.4).sin() * 0.5 + 0.5) as f32;
    egui::Area::new(egui::Id::new("viewer-player-mini-map"))
        .anchor(egui::Align2::RIGHT_BOTTOM, egui::vec2(-14.0, -14.0))
        .movable(false)
        .interactable(false)
        .show(context, |ui| {
            egui::Frame::group(ui.style())
                .fill(egui::Color32::from_rgba_unmultiplied(13, 20, 32, 224))
                .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(56, 96, 146)))
                .corner_radius(egui::CornerRadius::same(10))
                .inner_margin(egui::Margin::same(10))
                .show(ui, |ui| {
                    ui.set_max_width(230.0);
                    ui.small(if locale.is_zh() {
                        "战术小地图"
                    } else {
                        "Tactical Mini-map"
                    });
                    let map_size = egui::vec2(190.0, 110.0);
                    let (rect, _) = ui.allocate_exact_size(map_size, egui::Sense::hover());
                    let painter = ui.painter_at(rect);
                    painter.rect_filled(rect, 6.0, egui::Color32::from_rgb(20, 30, 46));
                    painter.rect_stroke(
                        rect,
                        6.0,
                        egui::Stroke::new(1.0, egui::Color32::from_rgb(48, 72, 108)),
                        egui::StrokeKind::Outside,
                    );
                    painter.line_segment(
                        [
                            egui::pos2(rect.center().x, rect.top()),
                            egui::pos2(rect.center().x, rect.bottom()),
                        ],
                        egui::Stroke::new(1.0, egui::Color32::from_rgb(40, 58, 86)),
                    );
                    painter.line_segment(
                        [
                            egui::pos2(rect.left(), rect.center().y),
                            egui::pos2(rect.right(), rect.center().y),
                        ],
                        egui::Stroke::new(1.0, egui::Color32::from_rgb(40, 58, 86)),
                    );

                    let mut selected_count = 0usize;
                    for point in points {
                        let pos = egui::pos2(
                            rect.left() + point.x * rect.width(),
                            rect.top() + point.y * rect.height(),
                        );
                        if point.selected {
                            selected_count = selected_count.saturating_add(1);
                        }
                        let radius = if point.selected {
                            4.2 + 1.4 * pulse
                        } else {
                            2.8
                        };
                        let color = if point.selected {
                            egui::Color32::from_rgb(244, 196, 96)
                        } else {
                            egui::Color32::from_rgb(92, 150, 218)
                        };
                        painter.circle_filled(pos, radius, color);
                    }

                    if points.is_empty() {
                        ui.small(if locale.is_zh() {
                            "等待位置数据..."
                        } else {
                            "Waiting for location data..."
                        });
                    } else {
                        ui.small(format!(
                            "{} {} | {} {}",
                            if locale.is_zh() {
                                "地点"
                            } else {
                                "Locations"
                            },
                            points.len(),
                            if locale.is_zh() { "选中" } else { "Selected" },
                            selected_count
                        ));
                    }
                });
        });
}

fn player_micro_loop_tone_color(tone: PlayerMicroLoopTone) -> egui::Color32 {
    match tone {
        PlayerMicroLoopTone::Positive => egui::Color32::from_rgb(92, 188, 126),
        PlayerMicroLoopTone::Warning => egui::Color32::from_rgb(230, 148, 96),
        PlayerMicroLoopTone::Info => egui::Color32::from_rgb(116, 174, 236),
    }
}

fn render_player_micro_loop_summary(
    ui: &mut egui::Ui,
    snapshot: &PlayerMicroLoopSnapshot,
    locale: crate::i18n::UiLocale,
) {
    let tone = player_micro_loop_tone_color(snapshot.action_status.tone);
    egui::Frame::group(ui.style())
        .fill(egui::Color32::from_rgba_unmultiplied(26, 34, 48, 144))
        .stroke(egui::Stroke::new(1.0, tone))
        .corner_radius(egui::CornerRadius::same(7))
        .inner_margin(egui::Margin::same(7))
        .show(ui, |ui| {
            ui.small(if locale.is_zh() {
                "微循环反馈"
            } else {
                "Micro-loop Feedback"
            });
            ui.small(egui::RichText::new(snapshot.action_status.headline.as_str()).color(tone));
            ui.small(snapshot.action_status.detail.as_str());
            if let Some(pending_eta_ticks) = snapshot.action_status.pending_eta_ticks {
                ui.small(if locale.is_zh() {
                    format!("动作 ETA: 约 {} tick", pending_eta_ticks)
                } else {
                    format!("Action ETA: about {} ticks", pending_eta_ticks)
                });
            }
            if snapshot.due_timers.is_empty() {
                ui.small(if locale.is_zh() {
                    "关键计时器：暂无激活项"
                } else {
                    "Key timers: none active"
                });
            } else {
                ui.small(if locale.is_zh() {
                    "关键计时器（战争/治理/危机/合约）"
                } else {
                    "Key timers (war/governance/crisis/contract)"
                });
                for timer in snapshot.due_timers.iter().take(4) {
                    ui.small(
                        egui::RichText::new(format_due_timer_line(timer, locale)).color(
                            if timer.overdue_ticks > 0 {
                                egui::Color32::from_rgb(238, 168, 108)
                            } else {
                                egui::Color32::from_rgb(186, 206, 238)
                            },
                        ),
                    );
                }
            }
        });
}

fn render_player_control_result_strip(
    ui: &mut egui::Ui,
    feedback: &WebTestApiControlFeedbackSnapshot,
    locale: crate::i18n::UiLocale,
    pulse: f32,
) {
    let stage_color = player_control_stage_color(feedback.stage.as_str());
    let stroke_alpha = (172.0 + 58.0 * pulse).round() as u8;
    let fill_alpha = if feedback.stage == "blocked" { 62 } else { 44 };
    egui::Frame::group(ui.style())
        .fill(egui::Color32::from_rgba_unmultiplied(
            stage_color.r(),
            stage_color.g(),
            stage_color.b(),
            fill_alpha,
        ))
        .stroke(egui::Stroke::new(
            1.1,
            egui::Color32::from_rgba_unmultiplied(
                stage_color.r(),
                stage_color.g(),
                stage_color.b(),
                stroke_alpha,
            ),
        ))
        .corner_radius(egui::CornerRadius::same(7))
        .inner_margin(egui::Margin::same(7))
        .show(ui, |ui| {
            ui.small(if locale.is_zh() {
                "最新指令"
            } else {
                "Latest Order"
            });
            ui.small(
                egui::RichText::new(player_control_result_summary(feedback, locale))
                    .color(stage_color)
                    .strong(),
            );
            ui.small(format!(
                "{} · {}",
                feedback.action,
                player_control_stage_label(feedback.stage.as_str(), locale)
            ));
            if !feedback.effect.is_empty() {
                ui.small(feedback.effect.as_str());
            }
            ui.small(if locale.is_zh() {
                format!(
                    "增量: tick +{} · event +{} · trace +{}",
                    feedback.delta_logical_time,
                    feedback.delta_event_seq,
                    feedback.delta_trace_count
                )
            } else {
                format!(
                    "Delta: tick +{} · event +{} · trace +{}",
                    feedback.delta_logical_time,
                    feedback.delta_event_seq,
                    feedback.delta_trace_count
                )
            });
        });
}

pub(crate) fn player_control_result_summary(
    feedback: &WebTestApiControlFeedbackSnapshot,
    locale: crate::i18n::UiLocale,
) -> String {
    match (feedback.stage.as_str(), locale.is_zh()) {
        ("received", true) => "指令已接收，世界正在应用。".to_string(),
        ("received", false) => "Order received and queued into the world.".to_string(),
        ("executing", true) => "指令执行中，继续观察前台反馈。".to_string(),
        ("executing", false) => "Order executing now; watch the world-facing feedback.".to_string(),
        ("blocked", true) => "指令遇到阻塞，先处理当前代价。".to_string(),
        ("blocked", false) => "Order hit a blocker; resolve the current cost first.".to_string(),
        ("completed_no_progress", true) => "指令已结束，但还没有形成有效推进。".to_string(),
        ("completed_no_progress", false) => {
            "Order completed, but it did not create useful forward progress.".to_string()
        }
        ("completed_advanced" | "applied", true) => "指令已推进世界状态。".to_string(),
        ("completed_advanced" | "applied", false) => "Order advanced the world state.".to_string(),
        (_, true) => format!(
            "指令状态：{}",
            player_control_stage_label(feedback.stage.as_str(), locale)
        ),
        (_, false) => format!(
            "Order status: {}",
            player_control_stage_label(feedback.stage.as_str(), locale)
        ),
    }
}

fn player_identity_line(
    state: &crate::ViewerState,
    progress: PlayerGuideProgressSnapshot,
    locale: crate::i18n::UiLocale,
) -> String {
    if let Some(claimer_agent_id) = state
        .snapshot
        .as_ref()
        .and_then(|snapshot| snapshot.player_gameplay.as_ref())
        .and_then(|gameplay| gameplay.agent_claim.as_ref())
        .map(|claim| super::super::truncate_observe_text(&claim.claimer_agent_id, 18))
    {
        return if locale.is_zh() {
            format!("你正在负责 {} 的首条产线。", claimer_agent_id)
        } else {
            format!("You are directing {claimer_agent_id}'s first industrial line.")
        };
    }

    if progress.explore_ready {
        if locale.is_zh() {
            "你当前是首条工业线的负责人。".to_string()
        } else {
            "You are the lead for the first industrial line.".to_string()
        }
    } else if locale.is_zh() {
        "你当前在建立首局行动闭环。".to_string()
    } else {
        "You are still establishing the first action loop.".to_string()
    }
}

fn render_player_next_step_card(
    ui: &mut egui::Ui,
    next_step: &str,
    locale: crate::i18n::UiLocale,
    tone: egui::Color32,
) {
    egui::Frame::group(ui.style())
        .fill(egui::Color32::from_rgba_unmultiplied(
            tone.r(),
            tone.g(),
            tone.b(),
            24,
        ))
        .stroke(egui::Stroke::new(1.0, tone))
        .corner_radius(egui::CornerRadius::same(7))
        .inner_margin(egui::Margin::same(7))
        .show(ui, |ui| {
            ui.small(if locale.is_zh() {
                "下一步"
            } else {
                "Immediate Next Step"
            });
            ui.small(
                egui::RichText::new(next_step)
                    .color(egui::Color32::from_rgb(212, 228, 246))
                    .strong(),
            );
        });
}

pub(crate) fn render_player_mission_hud(
    context: &egui::Context,
    state: &crate::ViewerState,
    selection: &ViewerSelection,
    client: Option<&crate::ViewerClient>,
    control_feedback: Option<&WebTestApiControlFeedbackSnapshot>,
    control_profile: Option<&crate::ViewerControlProfileState>,
    layout_state: &mut RightPanelLayoutState,
    module_visibility: &mut crate::right_panel_module_visibility::RightPanelModuleVisibilityState,
    onboarding_visible: bool,
    step: PlayerGuideStep,
    progress: PlayerGuideProgressSnapshot,
    stuck_hint: Option<&str>,
    stuck_diagnosis: Option<&PlayerNoProgressDiagnosis>,
    locale: crate::i18n::UiLocale,
    now_secs: f64,
) {
    let snapshot = build_player_mission_loop_snapshot(step, progress, locale);
    let remaining_hint = build_player_mission_remaining_hint(step, progress, state, locale);
    let reward = build_player_reward_feedback_snapshot(progress, locale);
    let post_onboarding = progress
        .explore_ready
        .then(|| build_player_post_onboarding_snapshot(state, control_feedback, locale));
    let tone = post_onboarding
        .as_ref()
        .map(|snapshot| player_post_onboarding_status_color(snapshot.status))
        .unwrap_or_else(|| player_goal_color(step));
    let reward_tone = if let Some(post_onboarding) = post_onboarding.as_ref() {
        player_post_onboarding_status_color(post_onboarding.status)
    } else if reward.complete {
        egui::Color32::from_rgb(54, 166, 96)
    } else {
        egui::Color32::from_rgb(74, 126, 184)
    };
    let compact_mode = player_mission_hud_compact_mode(layout_state.panel_hidden);
    let mission_anchor_y = player_mission_hud_anchor_y(
        layout_state.panel_hidden,
        onboarding_visible,
        stuck_hint.is_some(),
    );
    let pulse = ((now_secs * 1.8).sin() * 0.5 + 0.5) as f32;
    let mut action_clicked = false;
    let mut command_clicked = false;
    let (mut recover_play_clicked, mut recover_step_clicked) = (false, false);
    let micro_loop_snapshot = build_player_micro_loop_snapshot(state, locale);
    let control_feedback_needs_recovery = control_feedback.as_ref().is_some_and(|feedback| {
        player_control_stage_shows_recovery_actions(feedback.stage.as_str())
    });
    let identity_line = player_identity_line(state, progress, locale);
    let show_secondary_signals = post_onboarding.is_none() || !compact_mode;
    egui::Area::new(egui::Id::new("viewer-player-mission-hud"))
        .anchor(egui::Align2::LEFT_TOP, egui::vec2(14.0, mission_anchor_y))
        .movable(false)
        .interactable(true)
        .show(context, |ui| {
            egui::Frame::group(ui.style())
                .fill(egui::Color32::from_rgba_unmultiplied(14, 22, 34, 230))
                .stroke(egui::Stroke::new(
                    1.0 + 0.45 * pulse,
                    egui::Color32::from_rgba_unmultiplied(
                        tone.r(),
                        tone.g(),
                        tone.b(),
                        (150.0 + 86.0 * pulse).round() as u8,
                    ),
                ))
                .corner_radius(egui::CornerRadius::same(10))
                .inner_margin(egui::Margin::same(10))
                .show(ui, |ui| {
                    ui.set_max_width(if compact_mode { 280.0 } else { 320.0 });
                    if let Some(post_onboarding) = post_onboarding.as_ref() {
                        ui.small(if locale.is_zh() {
                            "玩家身份"
                        } else {
                            "Player Role"
                        });
                        ui.small(
                            egui::RichText::new(identity_line.as_str())
                                .color(egui::Color32::from_rgb(222, 198, 120))
                                .strong(),
                        );
                        ui.small(
                            egui::RichText::new(player_post_onboarding_status_label(
                                post_onboarding.status,
                                locale,
                            ))
                            .color(tone)
                            .strong(),
                        );
                        ui.small(if locale.is_zh() {
                            "当前主目标"
                        } else {
                            "Primary Goal"
                        });
                        ui.strong(post_onboarding.title);
                        ui.label(post_onboarding.objective.as_str());
                    } else {
                        ui.small(egui::RichText::new(snapshot.title).color(tone).strong());
                        ui.small(if locale.is_zh() {
                            "玩家身份"
                        } else {
                            "Player Role"
                        });
                        ui.small(
                            egui::RichText::new(identity_line.as_str())
                                .color(egui::Color32::from_rgb(222, 198, 120))
                                .strong(),
                        );
                        ui.small(if locale.is_zh() {
                            "主目标"
                        } else {
                            "Main Goal"
                        });
                        ui.strong(snapshot.objective);
                        ui.small(snapshot.completion_condition);
                        ui.small(snapshot.eta);
                        ui.small(
                            egui::RichText::new(remaining_hint.as_str())
                                .color(egui::Color32::from_rgb(186, 206, 238)),
                        );
                    }
                    if let Some(feedback) = control_feedback.as_ref() {
                        render_player_control_result_strip(ui, feedback, locale, pulse);
                    }
                    if let Some(post_onboarding) = post_onboarding.as_ref() {
                        if let Some(blocker_detail) = post_onboarding.blocker_detail.as_ref() {
                            egui::Frame::group(ui.style())
                                .fill(egui::Color32::from_rgba_unmultiplied(92, 48, 28, 132))
                                .stroke(egui::Stroke::new(1.0, reward_tone))
                                .corner_radius(egui::CornerRadius::same(6))
                                .inner_margin(egui::Margin::same(6))
                                .show(ui, |ui| {
                                    ui.small(if locale.is_zh() {
                                        "当前阻塞"
                                    } else {
                                        "Current Blocker"
                                    });
                                    ui.small(
                                        egui::RichText::new(blocker_detail.as_str())
                                            .color(egui::Color32::from_rgb(248, 214, 186)),
                                    );
                                });
                        }
                        render_player_next_step_card(
                            ui,
                            post_onboarding.next_step.as_str(),
                            locale,
                            reward_tone,
                        );
                        ui.small(
                            egui::RichText::new(post_onboarding.progress_detail.as_str())
                                .color(egui::Color32::from_rgb(186, 206, 238)),
                        );
                        if let Some(branch_hint) = post_onboarding.branch_hint.as_ref() {
                            egui::Frame::group(ui.style())
                                .fill(egui::Color32::from_rgba_unmultiplied(
                                    reward_tone.r(),
                                    reward_tone.g(),
                                    reward_tone.b(),
                                    28,
                                ))
                                .stroke(egui::Stroke::new(1.0, reward_tone))
                                .corner_radius(egui::CornerRadius::same(8))
                                .inner_margin(egui::Margin::same(8))
                                .show(ui, |ui| {
                                    ui.small(if locale.is_zh() {
                                        "下一批方向"
                                    } else {
                                        "Next Branches"
                                    });
                                    ui.strong(branch_hint.as_str());
                                });
                        }
                    } else {
                        render_player_micro_loop_summary(ui, &micro_loop_snapshot, locale);
                        egui::CollapsingHeader::new(if locale.is_zh() {
                            "展开短目标"
                        } else {
                            "Expand short goals"
                        })
                        .default_open(false)
                        .show(ui, |ui| {
                            for goal in snapshot.short_goals {
                                let marker = if goal.complete { "✓" } else { "□" };
                                let color = if goal.complete {
                                    tone
                                } else {
                                    egui::Color32::from_gray(182)
                                };
                                ui.small(
                                    egui::RichText::new(format!("{marker} {}", goal.label))
                                        .color(color),
                                );
                            }
                        });
                        if !compact_mode {
                            ui.small(player_goal_detail(step, locale));
                        }
                    }
                    if post_onboarding.is_some() && show_secondary_signals {
                        egui::CollapsingHeader::new(if locale.is_zh() {
                            "次级世界线索"
                        } else {
                            "Secondary World Cues"
                        })
                        .default_open(false)
                        .show(ui, |ui| {
                            render_player_micro_loop_summary(ui, &micro_loop_snapshot, locale);
                        });
                    }
                    if let Some(stuck_hint) = stuck_hint {
                        egui::Frame::group(ui.style())
                            .fill(egui::Color32::from_rgba_unmultiplied(84, 42, 28, 132))
                            .stroke(egui::Stroke::new(
                                1.0,
                                egui::Color32::from_rgb(224, 146, 92),
                            ))
                            .corner_radius(egui::CornerRadius::same(6))
                            .inner_margin(egui::Margin::same(6))
                            .show(ui, |ui| {
                                ui.small(
                                    egui::RichText::new(stuck_hint)
                                        .color(egui::Color32::from_rgb(248, 210, 180)),
                                );
                                if let Some(diagnosis) = stuck_diagnosis {
                                    ui.small(
                                        egui::RichText::new(if locale.is_zh() {
                                            format!("原因：{}", diagnosis.reason)
                                        } else {
                                            format!("Cause: {}", diagnosis.reason)
                                        })
                                        .color(egui::Color32::from_rgb(244, 188, 152)),
                                    );
                                    ui.small(
                                        egui::RichText::new(if locale.is_zh() {
                                            format!("建议：{}", diagnosis.suggestion)
                                        } else {
                                            format!("Next: {}", diagnosis.suggestion)
                                        })
                                        .color(egui::Color32::from_rgb(204, 226, 244)),
                                    );
                                }
                                if client.is_some() && !control_feedback_needs_recovery {
                                    ui.horizontal_wrapped(|ui| {
                                        recover_play_clicked = ui
                                            .button(if locale.is_zh() {
                                                "恢复：step x1"
                                            } else {
                                                "Recover: step x1"
                                            })
                                            .clicked();
                                        recover_step_clicked = ui
                                            .button(if locale.is_zh() {
                                                "恢复：step x8"
                                            } else {
                                                "Recover: step x8"
                                            })
                                            .clicked();
                                    });
                                }
                            });
                    }
                    if let Some(feedback) = control_feedback.as_ref() {
                        let stage_color = player_control_stage_color(feedback.stage.as_str());
                        let show_detail_card =
                            player_control_stage_shows_recovery_actions(feedback.stage.as_str())
                                || feedback.reason.is_some()
                                || feedback.hint.is_some();
                        if show_detail_card {
                            egui::Frame::group(ui.style())
                                .fill(egui::Color32::from_rgba_unmultiplied(28, 36, 52, 156))
                                .stroke(egui::Stroke::new(1.0, stage_color))
                                .corner_radius(egui::CornerRadius::same(6))
                                .inner_margin(egui::Margin::same(6))
                                .show(ui, |ui| {
                                    ui.small(if locale.is_zh() {
                                        "反馈细节"
                                    } else {
                                        "Feedback Details"
                                    });
                                    if let Some(reason) = feedback.reason.as_ref() {
                                        ui.small(
                                            egui::RichText::new(reason.as_str())
                                                .color(egui::Color32::from_rgb(226, 164, 136)),
                                        );
                                    }
                                    if let Some(hint) = feedback.hint.as_ref() {
                                        ui.small(
                                            egui::RichText::new(hint.as_str())
                                                .color(egui::Color32::from_rgb(186, 206, 238)),
                                        );
                                    }
                                    if player_control_stage_shows_recovery_actions(
                                        feedback.stage.as_str(),
                                    ) && client.is_some()
                                    {
                                        ui.horizontal_wrapped(|ui| {
                                            recover_play_clicked = ui
                                                .button(if locale.is_zh() {
                                                    "恢复：step x1"
                                                } else {
                                                    "Recover: step x1"
                                                })
                                                .clicked();
                                            recover_step_clicked = ui
                                                .button(if locale.is_zh() {
                                                    "重试：step x8"
                                                } else {
                                                    "Retry: step x8"
                                                })
                                                .clicked();
                                        });
                                    }
                                });
                        }
                    }
                    let progress_ratio = post_onboarding
                        .as_ref()
                        .map(|snapshot| snapshot.progress_percent as f32 / 100.0)
                        .unwrap_or_else(|| (snapshot.completed_steps as f32 / 4.0).clamp(0.0, 1.0));
                    ui.add(
                        egui::ProgressBar::new(progress_ratio)
                            .desired_width(280.0)
                            .text(format!(
                                "{} {}",
                                if locale.is_zh() {
                                    if post_onboarding.is_some() {
                                        "阶段进度"
                                    } else {
                                        "任务进度"
                                    }
                                } else if post_onboarding.is_some() {
                                    "Stage Progress"
                                } else {
                                    "Mission Progress"
                                },
                                if let Some(post_onboarding) = post_onboarding.as_ref() {
                                    format!("{}%", post_onboarding.progress_percent)
                                } else {
                                    format!("{}/4", snapshot.completed_steps)
                                }
                            )),
                    );
                    if compact_mode && post_onboarding.is_none() {
                        ui.small(egui::RichText::new(reward.badge).color(reward_tone));
                    } else {
                        egui::Frame::group(ui.style())
                            .fill(egui::Color32::from_rgba_unmultiplied(
                                reward_tone.r(),
                                reward_tone.g(),
                                reward_tone.b(),
                                if reward.complete { 54 } else { 34 },
                            ))
                            .stroke(egui::Stroke::new(1.0, reward_tone))
                            .corner_radius(egui::CornerRadius::same(8))
                            .inner_margin(egui::Margin::same(8))
                            .show(ui, |ui| {
                                ui.small(egui::RichText::new(reward.badge).color(reward_tone));
                                ui.strong(reward.title);
                                ui.small(reward.detail.as_str());
                            });
                    }
                    ui.horizontal_wrapped(|ui| {
                        action_clicked = ui
                            .button(
                                post_onboarding
                                    .as_ref()
                                    .map(|snapshot| snapshot.action_label)
                                    .unwrap_or(snapshot.action_label),
                            )
                            .clicked();
                        if player_mission_hud_show_command_action(layout_state.panel_hidden) {
                            command_clicked = ui
                                .button(if locale.is_zh() {
                                    "直接指挥 Agent"
                                } else {
                                    "Command Agent"
                                })
                                .clicked();
                        }
                    });
                });
        });

    if action_clicked && snapshot.action_opens_panel {
        layout_state.panel_hidden = false;
    }
    if action_clicked && post_onboarding.is_some() {
        apply_player_layout_preset(layout_state, module_visibility, PlayerLayoutPreset::Command);
        if let Some(client) = client {
            let _ = crate::dispatch_viewer_control(
                client,
                control_profile,
                oasis7::viewer::ViewerControl::Step { count: 1 },
                None,
            );
        }
    } else if action_clicked && step == PlayerGuideStep::ExploreAction {
        apply_player_layout_preset(layout_state, module_visibility, PlayerLayoutPreset::Command);
        if let Some(client) = client {
            let _ = crate::dispatch_viewer_control(
                client,
                control_profile,
                oasis7::viewer::ViewerControl::Step { count: 1 },
                None,
            );
        }
    }
    if command_clicked {
        apply_player_layout_preset(layout_state, module_visibility, PlayerLayoutPreset::Command);
    }
    if let Some(client) = client {
        if recover_play_clicked {
            let _ = crate::dispatch_viewer_control(
                client,
                control_profile,
                oasis7::viewer::ViewerControl::Step { count: 1 },
                None,
            );
        }
        if recover_step_clicked {
            let _ = crate::dispatch_viewer_control(
                client,
                control_profile,
                oasis7::viewer::ViewerControl::Step { count: 8 },
                None,
            );
        }
    }

    if player_mission_hud_show_minimap(layout_state.panel_hidden) {
        let points = build_player_minimap_snapshot(state, selection);
        render_player_minimap_card(context, &points, locale, now_secs);
    }
}

pub(crate) fn player_mission_hud_compact_mode(panel_hidden: bool) -> bool {
    !panel_hidden
}

pub(crate) fn player_mission_hud_anchor_y(
    panel_hidden: bool,
    onboarding_visible: bool,
    stuck_hint_visible: bool,
) -> f32 {
    if player_mission_hud_compact_mode(panel_hidden) {
        96.0
    } else if onboarding_visible {
        if stuck_hint_visible {
            298.0
        } else {
            214.0
        }
    } else {
        136.0
    }
}

pub(crate) fn player_mission_hud_show_command_action(panel_hidden: bool) -> bool {
    panel_hidden
}

pub(crate) fn player_mission_hud_show_minimap(panel_hidden: bool) -> bool {
    panel_hidden
}

pub(crate) fn player_mission_hud_minimap_reserved_bottom(panel_hidden: bool) -> f32 {
    if player_mission_hud_show_minimap(panel_hidden) {
        188.0
    } else {
        0.0
    }
}
