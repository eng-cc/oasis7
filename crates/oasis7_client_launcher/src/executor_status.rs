use super::*;
impl ClientLauncherApp {
    pub(super) fn render_executor_status(&self, ui: &mut egui::Ui) {
        let zh = self.ui_language == UiLanguage::ZhCn;
        let fallback = oasis7_launcher_ui::AgentExecutorStatus::default();
        let status = self.executor_status.as_ref().unwrap_or(&fallback);
        let color = match status.phase.as_str() {
            "running" => Self::ui_ok_color(),
            "blocked" => Self::ui_error_color(),
            _ => Self::ui_idle_color(),
        };
        ui.horizontal_wrapped(|ui| {
            ui.label(
                egui::RichText::new(format!(
                    "{} · {}",
                    self.tr("工业任务执行器", "Industry task executor"),
                    status.phase_label(zh)
                ))
                .strong()
                .color(color),
            );
            let location = if status.phase == "not_selected" {
                self.tr(
                    "本节点未启用此执行 profile",
                    "This execution profile is not selected on this node",
                )
            } else {
                match status.execution_location.as_deref() {
                    Some("node_process") => self.tr("节点进程内执行", "Runs in node process"),
                    _ => self.tr("执行位置未知", "Execution location unknown"),
                }
            };
            ui.small(location);
            if status.profile.as_deref() == Some("native_task_industry_v1") {
                ui.small(self.tr("原生产业任务", "Native industry task"));
            }
            if let Some(model) = &status.model {
                ui.small(format!(
                    "{}: {}",
                    self.tr("模型", "Model"),
                    Self::compact_middle(model, 64)
                ));
            }
        });
        if status.phase != "not_selected" {
            let number = |value: Option<u32>| value.map_or_else(|| "—".into(), |v| v.to_string());
            ui.horizontal_wrapped(|ui| {
                ui.small(format!(
                    "{}: {} / {}",
                    self.tr("已预留模型调用", "Reserved model calls"),
                    number(status.reserved_model_calls),
                    number(status.max_model_calls)
                ));
                ui.small(format!(
                    "{}: {}",
                    self.tr("已提交轮次", "Committed turns"),
                    number(status.committed_turns)
                ));
                if matches!(status.phase.as_str(), "blocked" | "stopped") {
                    ui.small(status.reason_label(zh));
                }
            });
        }
    }
}
