use bevy::ecs::hierarchy::ChildSpawnerCommands;
use bevy::prelude::*;
use oasis7::simulator::{RejectReason, WorldEventKind};

use crate::i18n::{locale_or_default, UiI18n};
use crate::ui_locale_text::{diagnosis_waiting, localize_diagnosis_text};

use super::*;

#[derive(Resource)]
pub(super) struct DiagnosisState {
    pub text: String,
}

impl Default for DiagnosisState {
    fn default() -> Self {
        Self {
            text: diagnosis_waiting(crate::i18n::UiLocale::EnUs).to_string(),
        }
    }
}

#[derive(Component)]
pub(super) struct DiagnosisText;

#[allow(dead_code)]
pub(super) fn spawn_diagnosis_panel(
    parent: &mut ChildSpawnerCommands,
    font: Handle<Font>,
    locale: crate::i18n::UiLocale,
) {
    parent
        .spawn((
            Node {
                width: Val::Percent(100.0),
                row_gap: Val::Px(4.0),
                padding: UiRect::all(Val::Px(8.0)),
                border: UiRect::all(Val::Px(1.0)),
                flex_direction: FlexDirection::Column,
                ..default()
            },
            BackgroundColor(Color::srgb(0.16, 0.14, 0.1)),
            BorderColor::all(Color::srgb(0.32, 0.27, 0.18)),
        ))
        .with_children(|root| {
            root.spawn((
                Text::new(diagnosis_waiting(locale)),
                TextFont {
                    font,
                    font_size: 10.0,
                    ..default()
                },
                TextColor(Color::srgb(0.9, 0.84, 0.68)),
                DiagnosisText,
            ));
        });
}

pub(super) fn update_diagnosis_panel(
    state: Res<ViewerState>,
    selection: Res<ViewerSelection>,
    i18n: Option<Res<UiI18n>>,
    mut diagnosis_state: ResMut<DiagnosisState>,
    mut query: Query<&mut Text, With<DiagnosisText>>,
) {
    let locale_changed = i18n
        .as_ref()
        .map(|value| value.is_changed())
        .unwrap_or(false);
    if !state.is_changed() && !selection.is_changed() && !locale_changed {
        return;
    }

    let locale = locale_or_default(i18n.as_deref());

    let text = build_diagnosis_text(&state, &selection);
    let localized = localize_diagnosis_text(text, locale);
    diagnosis_state.text = localized.clone();

    if let Ok(mut diagnosis_text) = query.single_mut() {
        diagnosis_text.0 = localized;
    }
}

fn build_diagnosis_text(state: &ViewerState, selection: &ViewerSelection) -> String {
    if let ConnectionStatus::Error(message) = &state.status {
        return format!(
            "Diagnosis: viewer disconnected ({message}). Conclusion: data stream unavailable, check live server/network."
        );
    }

    if let Some(trace) = state
        .decision_traces
        .iter()
        .rev()
        .find(|trace| trace.llm_error.is_some() || trace.parse_error.is_some())
    {
        if let Some(err) = &trace.llm_error {
            return format!(
                "Diagnosis: t{} agent {} LLM call failed ({err}). Conclusion: decision degraded, check model endpoint/config.",
                trace.time, trace.agent_id
            );
        }
        if let Some(err) = &trace.parse_error {
            return format!(
                "Diagnosis: t{} agent {} decision parse failed ({err}). Conclusion: model output format mismatch.",
                trace.time, trace.agent_id
            );
        }
    }

    if let Some(event) = state.events.iter().rev().find(|event| {
        matches!(
            event.kind,
            WorldEventKind::ActionRejected { .. }
                | WorldEventKind::Power(_)
                | WorldEventKind::ChunkGenerated { .. }
        )
    }) {
        if let WorldEventKind::ActionRejected { reason } = &event.kind {
            return format!(
                "Diagnosis: t{} action rejected ({:?}). Conclusion: {}.",
                event.time,
                reason,
                reject_reason_conclusion(reason)
            );
        }
    }

    if state.snapshot.is_none() {
        return "Diagnosis: no snapshot yet. Conclusion: wait for first world snapshot."
            .to_string();
    }

    if let Some(selected) = selection.current.as_ref() {
        return format!(
            "Diagnosis: no blocking issue detected. Conclusion: simulation healthy, focus on selected {} {}.",
            selection_kind_name(selected.kind),
            selected.id
        );
    }

    "Diagnosis: no blocking issue detected. Conclusion: simulation healthy.".to_string()
}

fn reject_reason_conclusion(reason: &RejectReason) -> String {
    match reason {
        RejectReason::InsufficientResource {
            requested,
            available,
            ..
        } => format!(
            "resource shortage (requested {}, available {})",
            requested, available
        ),
        RejectReason::AgentNotAtLocation { .. } | RejectReason::AgentsNotCoLocated { .. } => {
            "location constraints not satisfied".to_string()
        }
        RejectReason::ThermalOverload { heat, capacity } => {
            format!("thermal overload (heat {}, capacity {})", heat, capacity)
        }
        RejectReason::AgentShutdown { .. } => "agent is shutdown".to_string(),
        RejectReason::PowerTransferDistanceExceeded { .. } => {
            "power transfer distance exceeded".to_string()
        }
        RejectReason::PowerTransferLossExceedsAmount { .. } => {
            "power transfer loss exceeds amount".to_string()
        }
        _ => "action preconditions not satisfied".to_string(),
    }
}

fn selection_kind_name(kind: SelectionKind) -> &'static str {
    match kind {
        SelectionKind::Agent => "agent",
        SelectionKind::Location => "location",
        SelectionKind::Fragment => "fragment",
        SelectionKind::Asset => "asset",
        SelectionKind::PowerPlant => "power_plant",
        SelectionKind::Chunk => "chunk",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oasis7::simulator::{
        AgentDecision, AgentDecisionTrace, LlmDecisionDiagnostics, WorldEvent,
    };

    #[test]
    fn diagnosis_reports_connection_error_directly() {
        let state = ViewerState {
            status: ConnectionStatus::Error("disconnected".to_string()),
            snapshot: None,
            events: Vec::new(),
            decision_traces: Vec::new(),
            metrics: None,
        };

        let text = build_diagnosis_text(&state, &ViewerSelection::default());
        assert!(text.contains("viewer disconnected"));
        assert!(text.contains("data stream unavailable"));
    }

    #[test]
    fn diagnosis_reports_llm_error_directly() {
        let state = ViewerState {
            status: ConnectionStatus::Connected,
            snapshot: None,
            events: Vec::new(),
            decision_traces: vec![AgentDecisionTrace {
                agent_id: "agent-1".to_string(),
                time: 7,
                decision: AgentDecision::Wait,
                llm_input: None,
                llm_output: None,
                llm_error: Some("timeout".to_string()),
                parse_error: None,
                llm_diagnostics: Some(LlmDecisionDiagnostics::default()),
                llm_effect_intents: Vec::new(),
                llm_effect_receipts: Vec::new(),
                llm_step_trace: Vec::new(),
                llm_prompt_section_trace: Vec::new(),
                llm_chat_messages: Vec::new(),
            }],
            metrics: None,
        };

        let text = build_diagnosis_text(&state, &ViewerSelection::default());
        assert!(text.contains("LLM call failed"));
        assert!(text.contains("decision degraded"));
    }

    #[test]
    fn diagnosis_reports_reject_reason_conclusion() {
        let state = ViewerState {
            status: ConnectionStatus::Connected,
            snapshot: None,
            events: vec![WorldEvent {
                id: 3,
                time: 9,
                kind: WorldEventKind::ActionRejected {
                    reason: RejectReason::InsufficientResource {
                        owner: oasis7::simulator::ResourceOwner::Agent {
                            agent_id: "agent-1".to_string(),
                        },
                        kind: oasis7::simulator::ResourceKind::Electricity,
                        requested: 20,
                        available: 5,
                    },
                },
                runtime_event: None,
            }],
            decision_traces: Vec::new(),
            metrics: None,
        };

        let text = build_diagnosis_text(&state, &ViewerSelection::default());
        assert!(text.contains("action rejected"));
        assert!(text.contains("resource shortage"));
    }
}
