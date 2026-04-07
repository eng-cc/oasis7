use super::*;

pub(super) fn render_text_sections(
    ui: &mut egui::Ui,
    locale: crate::i18n::UiLocale,
    state: &ViewerState,
    selection: &ViewerSelection,
    timeline: &TimelineUiState,
    viewer_3d_config: &Option<Res<Viewer3dConfig>>,
    industry_zoom_level: IndustrySemanticZoomLevel,
    provider_debug_filter: &mut ProviderDebugFilter,
) {
    let focus = if timeline.manual_override || timeline.drag_active {
        Some(timeline.target_tick)
    } else {
        None
    };

    let reference_radiation_area_m2 = viewer_3d_config
        .as_deref()
        .map(|cfg| cfg.physical.reference_radiation_area_m2)
        .unwrap_or(1.0);

    let summary = localize_world_summary_block(
        world_summary(
            state.snapshot.as_ref(),
            state.metrics.as_ref(),
            viewer_3d_config.as_deref().map(|cfg| &cfg.physical),
        ),
        locale,
    );
    let activity = localize_agent_activity_block(
        agent_activity_summary(state.snapshot.as_ref(), &state.events),
        locale,
    );
    let industry_graph = build_industry_graph_view_model(state.snapshot.as_ref(), &state.events);
    let industrial = industrial_ops_summary_with_zoom(
        &industry_graph,
        state.snapshot.as_ref(),
        &state.events,
        industry_zoom_level,
    )
    .map(|text| localize_industrial_ops_block(text, locale));
    let economy = economy_dashboard_summary_with_zoom(&industry_graph, industry_zoom_level)
        .map(|text| localize_economy_dashboard_block(text, locale));
    let ops_navigation = if is_ops_nav_panel_enabled() {
        ops_navigation_alert_summary_with_zoom(&industry_graph, industry_zoom_level)
            .map(|text| localize_ops_navigation_block(text, locale))
    } else {
        None
    };
    let details = localize_details_block(
        selection_details_summary(
            selection,
            state.snapshot.as_ref(),
            &state.events,
            &state.decision_traces,
            reference_radiation_area_m2,
        ),
        locale,
    );
    let events = localize_events_summary_block(events_summary(&state.events, focus), locale);
    let provider_debug = provider_debug_summary(&state.decision_traces, *provider_debug_filter);

    let mut sections: Vec<(&str, String)> = vec![
        (
            if locale.is_zh() {
                "世界摘要"
            } else {
                "World Summary"
            },
            summary,
        ),
        (
            if locale.is_zh() {
                "Agent 活动"
            } else {
                "Agent Activity"
            },
            activity,
        ),
    ];
    if let Some(industrial) = industrial {
        sections.push((
            if locale.is_zh() {
                "工业链路"
            } else {
                "Industrial Ops"
            },
            industrial,
        ));
    }
    if let Some(economy) = economy {
        sections.push((
            if locale.is_zh() {
                "经营看板"
            } else {
                "Economy Dashboard"
            },
            economy,
        ));
    }
    if let Some(ops_navigation) = ops_navigation {
        sections.push((
            if locale.is_zh() {
                "运营导航"
            } else {
                "Ops Navigator"
            },
            ops_navigation,
        ));
    }
    sections.push((
        if locale.is_zh() {
            "选中详情"
        } else {
            "Selection Details"
        },
        details,
    ));
    sections.push((if locale.is_zh() { "事件" } else { "Events" }, events));

    ui.horizontal_wrapped(|ui| {
        ui.small(if locale.is_zh() {
            "Provider 调试筛选"
        } else {
            "Provider Debug Filter"
        });
        ui.selectable_value(
            provider_debug_filter,
            ProviderDebugFilter::All,
            if locale.is_zh() { "全部" } else { "All" },
        );
        ui.selectable_value(
            provider_debug_filter,
            ProviderDebugFilter::LoopbackProviderOnly,
            if locale.is_zh() {
                "仅 local provider"
            } else {
                "local provider Only"
            },
        );
        ui.selectable_value(
            provider_debug_filter,
            ProviderDebugFilter::ErrorsOnly,
            if locale.is_zh() {
                "仅错误"
            } else {
                "Errors Only"
            },
        );
    });

    sections.push((
        if locale.is_zh() {
            "Provider 调试"
        } else {
            "Provider Debug"
        },
        provider_debug,
    ));

    let product_style = is_product_style_enabled();
    let product_style_motion = product_style && is_product_style_motion_enabled();
    for (title, content) in sections {
        render_observe_section_card(
            ui,
            title,
            content.as_str(),
            product_style,
            product_style_motion,
        );
    }

    if let Some(current) = selection.current.as_ref() {
        ui.add(
            egui::Label::new(format!(
                "{} {} {}",
                if locale.is_zh() {
                    "选中类型:"
                } else {
                    "Selection kind:"
                },
                selection_kind_label(current.kind),
                current.id
            ))
            .wrap()
            .selectable(true),
        );
    }

    let ticks = vec![timeline.target_tick, focus.unwrap_or(timeline.target_tick)];
    let shown: Vec<String> = ticks
        .into_iter()
        .take(MAX_TICK_LABELS)
        .map(|tick| tick.to_string())
        .collect();

    ui.add(
        egui::Label::new(format!(
            "{} {}",
            if locale.is_zh() {
                "Tick 标签:"
            } else {
                "Tick labels:"
            },
            shown.join(", ")
        ))
        .wrap()
        .selectable(true),
    );
}
