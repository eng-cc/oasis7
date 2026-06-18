use super::*;
use std::fmt::Display;

impl ClientLauncherApp {
    pub(super) fn render_runtime_overview_panel(
        &mut self,
        ui: &mut egui::Ui,
        game_required_issues: &[ConfigIssue],
        chain_required_issues: &[ConfigIssue],
    ) {
        ui.group(|ui| {
            ui.set_width(ui.available_width());
            ui.label(
                egui::RichText::new(self.tr("系统健康", "System Health"))
                    .strong()
                    .size(15.0),
            );
            let game_url = self.current_game_url();
            let has_saved_profile = self.ux_state.last_successful_config.is_some();
            let tile_width = 116.0;
            ui.horizontal(|ui| {
                ui.allocate_ui_with_layout(
                    egui::vec2(tile_width, 116.0),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        Self::metric_tile(ui, self.tr("P2P 网络", "P2P Network"), |ui| {
                            self.render_health_p2p_body(ui);
                        });
                    },
                );
                ui.allocate_ui_with_layout(
                    egui::vec2(tile_width, 116.0),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        Self::metric_tile(ui, self.tr("可观测性", "Observability"), |ui| {
                            self.render_health_observability_body(ui);
                        });
                    },
                );
            });
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.allocate_ui_with_layout(
                    egui::vec2(tile_width, 116.0),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        Self::metric_tile(ui, self.tr("配置", "Configuration"), |ui| {
                            self.render_health_config_body(
                                ui,
                                game_required_issues,
                                chain_required_issues,
                            );
                        });
                    },
                );
                ui.allocate_ui_with_layout(
                    egui::vec2(tile_width, 116.0),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        Self::metric_tile(ui, self.tr("配置档案", "Profile"), |ui| {
                            ui.small(format!(
                                "{}: {}",
                                self.tr("游戏地址", "Game URL"),
                                Self::compact_middle(game_url.as_str(), 18)
                            ))
                            .on_hover_text(game_url);
                            ui.small(format!(
                                "{}: {}",
                                self.tr("最近配置", "Profile"),
                                if has_saved_profile {
                                    self.tr("已保存", "Saved")
                                } else {
                                    self.tr("无", "None")
                                }
                            ));
                            ui.small(format!(
                                "open={} done={} quick={}",
                                self.ux_state.onboarding_opened_count,
                                self.ux_state.onboarding_completed_count,
                                self.ux_state.quick_action_click_count
                            ));
                        });
                    },
                );
            });
            ui.add_space(4.0);
            egui::Frame::new()
                .fill(egui::Color32::from_rgb(250, 252, 255))
                .stroke(Self::card_stroke())
                .inner_margin(egui::Margin::symmetric(8, 6))
                .show(ui, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        ui.small(self.tr("引导洞察", "Guidance Insights"));
                        ui.separator();
                        ui.small(format!(
                            "{} {}",
                            self.ux_state.onboarding_opened_count,
                            self.tr("会话", "sessions")
                        ));
                        ui.separator();
                        ui.small(format!(
                            "{} {}",
                            self.ux_state.quick_action_click_count,
                            self.tr("快捷操作", "quick actions")
                        ));
                    });
                });
        });
    }

    fn render_health_p2p_body(&self, ui: &mut egui::Ui) {
        if !self.config.chain_enabled {
            Self::health_metric_pair(
                ui,
                self.tr("节点", "Peers"),
                "0",
                Self::ui_idle_color(),
                self.tr("延迟", "Latency"),
                "--",
            );
            Self::render_empty_sparkline(ui);
            Self::health_state_chip(ui, self.tr("已禁用", "Disabled"), Self::ui_idle_color());
            return;
        }

        if let Some(status) = self.chain_p2p_status.as_ref() {
            let applied_mode = status
                .applied_effective_user_mode
                .as_deref()
                .unwrap_or(status.effective_user_mode.as_str());
            let reachable = status
                .detected_reachability
                .as_deref()
                .unwrap_or(self.tr("未探测", "Unknown"));
            Self::health_metric_pair(
                ui,
                self.tr("模式", "Mode"),
                self.chain_user_mode_label(applied_mode),
                Self::ui_ok_color(),
                self.tr("可达", "Reach"),
                reachable,
            );
            ui.small(format!(
                "relay={} probe={}",
                self.p2p_probe_bool_text(status.relay_available),
                self.p2p_probe_bool_text(status.probe_stable)
            ));
        } else {
            Self::health_metric_pair(
                ui,
                self.tr("节点", "Peers"),
                "--",
                Self::ui_idle_color(),
                self.tr("延迟", "Latency"),
                "--",
            );
            Self::render_empty_sparkline(ui);
            Self::health_state_chip(ui, self.tr("等待节点", "Waiting"), Self::ui_idle_color());
        }
    }

    fn render_health_observability_body(&self, ui: &mut egui::Ui) {
        if !self.config.chain_enabled {
            Self::health_metric_pair(
                ui,
                self.tr("指标", "Metrics"),
                "0",
                Self::ui_idle_color(),
                self.tr("告警", "Alerts"),
                "0",
            );
            Self::render_empty_bars(ui);
            Self::health_state_chip(ui, self.tr("已禁用", "Disabled"), Self::ui_idle_color());
            return;
        }

        if let Some(status) = self.chain_observability_status.as_ref() {
            let status_color = self.observability_status_color(status.status.as_str());
            Self::health_metric_pair(
                ui,
                self.tr("Peer", "Peers"),
                status.connected_peer_count,
                status_color,
                self.tr("Heads", "Heads"),
                status.known_peer_heads,
            );
            ui.small(format!(
                "{}: {}",
                self.tr("网络落后高度", "Network Lag"),
                status.network_height_lag
            ));
            Self::health_state_chip(
                ui,
                self.observability_status_text(status.status.as_str()),
                status_color,
            );
        } else {
            Self::health_metric_pair(
                ui,
                self.tr("Peer", "Peers"),
                "--",
                Self::ui_idle_color(),
                self.tr("告警", "Alerts"),
                "--",
            );
            Self::render_empty_bars(ui);
            Self::health_state_chip(ui, self.tr("等待遥测", "Waiting"), Self::ui_idle_color());
        }
    }

    fn render_health_config_body(
        &mut self,
        ui: &mut egui::Ui,
        game_required_issues: &[ConfigIssue],
        chain_required_issues: &[ConfigIssue],
    ) {
        let chain_issue_count = if self.config.chain_enabled {
            chain_required_issues.len()
        } else {
            0
        };
        let has_issue = !game_required_issues.is_empty() || chain_issue_count > 0;
        let issue_count = game_required_issues.len() + chain_issue_count;
        Self::health_metric_pair(
            ui,
            self.tr("问题", "Issues"),
            issue_count,
            if has_issue {
                Self::ui_error_color()
            } else {
                Self::ui_ok_color()
            },
            self.tr("游戏", "Game"),
            game_required_issues.len(),
        );
        if ui
            .small_button(self.tr("高级配置", "Advanced Config"))
            .clicked()
        {
            self.config_window_open = true;
        }
        if has_issue {
            Self::health_state_chip(
                ui,
                self.tr("存在配置问题", "Config Issues"),
                Self::ui_error_color(),
            );
            ui.small(self.tr("查看具体字段。", "Inspect fields."));
        } else {
            Self::health_state_chip(ui, self.tr("配置通过", "Config OK"), Self::ui_ok_color());
        }
    }

    fn health_metric_pair(
        ui: &mut egui::Ui,
        primary_label: &str,
        primary_value: impl Display,
        primary_color: egui::Color32,
        secondary_label: &str,
        secondary_value: impl Display,
    ) {
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label(
                    egui::RichText::new(primary_value.to_string())
                        .strong()
                        .size(16.0)
                        .color(primary_color),
                );
                ui.small(primary_label);
            });
            ui.separator();
            ui.vertical(|ui| {
                ui.label(
                    egui::RichText::new(secondary_value.to_string())
                        .strong()
                        .size(13.0),
                );
                ui.small(secondary_label);
            });
        });
    }

    fn health_state_chip(ui: &mut egui::Ui, text: &str, color: egui::Color32) {
        let fill = if color == Self::ui_ok_color() {
            egui::Color32::from_rgb(229, 246, 235)
        } else if color == Self::ui_error_color() {
            egui::Color32::from_rgb(252, 235, 232)
        } else {
            egui::Color32::from_rgb(240, 244, 248)
        };
        egui::Frame::new()
            .fill(fill)
            .stroke(Self::card_stroke())
            .inner_margin(egui::Margin::symmetric(5, 2))
            .show(ui, |ui| {
                ui.small(egui::RichText::new(text).strong().color(color));
            });
    }

    fn metric_tile(ui: &mut egui::Ui, title: &str, add_body: impl FnOnce(&mut egui::Ui)) {
        egui::Frame::group(ui.style())
            .fill(Self::card_fill())
            .stroke(Self::card_stroke())
            .inner_margin(egui::Margin::same(8))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.label(egui::RichText::new(title).strong().size(11.5));
                add_body(ui);
            });
    }

    fn render_empty_sparkline(ui: &mut egui::Ui) {
        let width = ui.available_width().clamp(72.0, 112.0);
        let (rect, _) = ui.allocate_exact_size(egui::vec2(width, 24.0), egui::Sense::hover());
        let painter = ui.painter();
        let stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(199, 213, 228));
        let points = [
            (0.00, 0.70),
            (0.18, 0.48),
            (0.34, 0.62),
            (0.50, 0.38),
            (0.68, 0.56),
            (0.84, 0.44),
            (1.00, 0.52),
        ];
        for pair in points.windows(2) {
            let a =
                rect.left_top() + egui::vec2(rect.width() * pair[0].0, rect.height() * pair[0].1);
            let b =
                rect.left_top() + egui::vec2(rect.width() * pair[1].0, rect.height() * pair[1].1);
            painter.line_segment([a, b], stroke);
        }
    }

    fn render_empty_bars(ui: &mut egui::Ui) {
        let graph_width = ui.available_width().clamp(72.0, 112.0);
        let (rect, _) = ui.allocate_exact_size(egui::vec2(graph_width, 24.0), egui::Sense::hover());
        let painter = ui.painter();
        let fill = egui::Color32::from_rgb(207, 224, 242);
        for index in 0..14 {
            let width = 4.0;
            let gap = 3.0;
            let x = rect.left() + index as f32 * (width + gap);
            let height = 6.0 + ((index * 7) % 17) as f32;
            let bar = egui::Rect::from_min_size(
                egui::pos2(x, rect.bottom() - height),
                egui::vec2(width, height),
            );
            painter.rect_filled(bar, 1.0, fill);
        }
    }
}
