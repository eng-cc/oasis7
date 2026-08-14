use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use eframe::egui;

use super::{LaunchConfig, UiLanguage};

const DEFAULT_CONFIG_PATH: &str = "config.toml";

#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct LlmSettingsDraft {
    api_key: String,
    base_url: String,
    model: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum LlmSettingsStatus {
    None,
    Success(String),
    Failed(String),
}

#[derive(Debug)]
pub(crate) struct LlmSettingsPanel {
    config_path: PathBuf,
    draft: LlmSettingsDraft,
    status: LlmSettingsStatus,
    open: bool,
}

impl LlmSettingsPanel {
    pub(crate) fn new(path: impl Into<PathBuf>) -> Self {
        let mut panel = Self {
            config_path: path.into(),
            draft: LlmSettingsDraft::default(),
            status: LlmSettingsStatus::None,
            open: false,
        };
        panel.reload_from_file();
        panel
    }

    pub(crate) fn default_path() -> &'static str {
        DEFAULT_CONFIG_PATH
    }

    pub(crate) fn open(&mut self) {
        self.open = true;
        self.reload_from_file();
    }

    pub(crate) fn show(
        &mut self,
        ctx: &egui::Context,
        language: UiLanguage,
        config: &mut LaunchConfig,
    ) {
        if !self.open {
            return;
        }

        let mut open = self.open;
        let mut save_clicked = false;
        let mut reload_clicked = false;

        egui::Window::new(tr(language, "设置中心", "Settings Center"))
            .open(&mut open)
            .resizable(true)
            .default_size(egui::vec2(860.0, 620.0))
            .show(ctx, |ui| {
                modal_card(ui, |ui| {
                    ui.label(
                        egui::RichText::new(tr(language, "设置中心", "Settings Center"))
                            .strong()
                            .size(18.0),
                    );
                    ui.small(tr(
                        language,
                        "管理启动器、链运行时和 LLM 连接配置。",
                        "Manage launcher, chain runtime, and LLM connection settings.",
                    ));
                });
                ui.add_space(8.0);

                egui::ScrollArea::vertical()
                    .max_height(500.0)
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        modal_card(ui, |ui| {
                            ui.collapsing(tr(language, "游戏与显示", "Game & Viewer"), |ui| {
                                ui.horizontal_wrapped(|ui| {
                                    ui.label(tr(language, "场景", "Scenario"));
                                    ui.text_edit_singleline(&mut config.scenario);
                                    ui.label(tr(language, "实时服务绑定", "Live Bind"));
                                    ui.text_edit_singleline(&mut config.live_bind);
                                });
                                ui.horizontal_wrapped(|ui| {
                                    ui.label(tr(language, "WebSocket 绑定", "Web Bind"));
                                    ui.text_edit_singleline(&mut config.web_bind);
                                    ui.label(tr(language, "游戏页面主机", "Viewer Host"));
                                    ui.text_edit_singleline(&mut config.viewer_host);
                                });
                                ui.horizontal_wrapped(|ui| {
                                    ui.label(tr(language, "游戏页面端口", "Viewer Port"));
                                    ui.text_edit_singleline(&mut config.viewer_port);
                                    ui.label(tr(
                                        language,
                                        "前端静态资源目录",
                                        "Viewer Static Directory",
                                    ));
                                    ui.text_edit_singleline(&mut config.viewer_static_dir);
                                });
                                ui.horizontal_wrapped(|ui| {
                                    ui.checkbox(
                                        &mut config.llm_enabled,
                                        tr(language, "启用 LLM", "Enable LLM"),
                                    );
                                    ui.checkbox(
                                        &mut config.auto_open_browser,
                                        tr(
                                            language,
                                            "自动打开浏览器",
                                            "Open Browser Automatically",
                                        ),
                                    );
                                });
                                ui.horizontal_wrapped(|ui| {
                                    ui.label(tr(language, "启动器二进制路径", "Launcher Binary"));
                                    ui.text_edit_singleline(&mut config.launcher_bin);
                                });
                            });
                        });
                        ui.add_space(8.0);

                        modal_card(ui, |ui| {
                            ui.collapsing(
                                tr(language, "区块链运行时", "Blockchain Runtime"),
                                |ui| {
                                    render_chain_settings(ui, language, config);
                                },
                            );
                        });
                        ui.add_space(8.0);

                        modal_card(ui, |ui| {
                            ui.label(
                                egui::RichText::new(tr(language, "LLM 连接配置", "LLM Connection"))
                                    .strong()
                                    .size(14.0),
                            );
                            ui.small(format!(
                                "{}: {}",
                                tr(language, "配置文件", "Config File"),
                                self.config_path.display()
                            ));
                            ui.add_space(6.0);

                            ui.horizontal_wrapped(|ui| {
                                ui.label("API Key");
                                ui.add(
                                    egui::TextEdit::singleline(&mut self.draft.api_key)
                                        .desired_width((ui.available_width() - 8.0).max(220.0))
                                        .password(true),
                                );
                            });
                            ui.horizontal_wrapped(|ui| {
                                ui.label("Base URL");
                                ui.add(
                                    egui::TextEdit::singleline(&mut self.draft.base_url)
                                        .desired_width((ui.available_width() - 8.0).max(220.0)),
                                );
                            });
                            ui.horizontal_wrapped(|ui| {
                                ui.label("Model");
                                ui.add(
                                    egui::TextEdit::singleline(&mut self.draft.model)
                                        .desired_width((ui.available_width() - 8.0).max(220.0)),
                                );
                            });
                        });
                    });

                ui.add_space(8.0);
                ui.horizontal_wrapped(|ui| {
                    if ui
                        .button(tr(language, "保存到 config.toml", "Save to config.toml"))
                        .clicked()
                    {
                        save_clicked = true;
                    }
                    if ui
                        .button(tr(language, "从文件重载", "Reload from File"))
                        .clicked()
                    {
                        reload_clicked = true;
                    }
                });

                match &self.status {
                    LlmSettingsStatus::Success(message) => {
                        ui.small(
                            egui::RichText::new(message.as_str())
                                .color(egui::Color32::from_rgb(62, 152, 92)),
                        );
                    }
                    LlmSettingsStatus::Failed(message) => {
                        ui.small(
                            egui::RichText::new(message.as_str())
                                .color(egui::Color32::from_rgb(196, 84, 84)),
                        );
                    }
                    LlmSettingsStatus::None => {}
                }
            });

        self.open = open;
        if save_clicked {
            self.save_to_file(language);
        }
        if reload_clicked {
            self.reload_from_file();
            self.status = LlmSettingsStatus::Success(
                tr(
                    language,
                    "已从文件重载 LLM 设置",
                    "Reloaded LLM settings from file",
                )
                .to_string(),
            );
        }
    }

    fn reload_from_file(&mut self) {
        match load_llm_settings_from_config(self.config_path.as_path()) {
            Ok(settings) => {
                self.draft = settings;
            }
            Err(err) => {
                self.status = LlmSettingsStatus::Failed(err);
            }
        }
    }

    fn save_to_file(&mut self, language: UiLanguage) {
        match save_llm_settings_to_config(self.config_path.as_path(), &self.draft) {
            Ok(()) => {
                self.status = LlmSettingsStatus::Success(
                    tr(language, "LLM 配置已保存", "LLM settings saved").to_string(),
                );
            }
            Err(err) => {
                self.status = LlmSettingsStatus::Failed(format!(
                    "{}: {err}",
                    tr(language, "LLM 配置保存失败", "Failed to save LLM settings")
                ));
            }
        }
    }
}

fn tr(language: UiLanguage, zh: &'static str, en: &'static str) -> &'static str {
    match language {
        UiLanguage::ZhCn => zh,
        UiLanguage::EnUs => en,
    }
}

fn modal_card(ui: &mut egui::Ui, add_contents: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::new()
        .fill(egui::Color32::from_rgb(255, 255, 255))
        .stroke(egui::Stroke::new(
            1.0,
            egui::Color32::from_rgb(224, 229, 236),
        ))
        .inner_margin(egui::Margin::symmetric(10, 8))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            add_contents(ui);
        });
}

fn render_chain_settings(ui: &mut egui::Ui, language: UiLanguage, config: &mut LaunchConfig) {
    ui.horizontal_wrapped(|ui| {
        ui.checkbox(
            &mut config.chain_enabled,
            tr(language, "启用链运行时", "Enable Chain Runtime"),
        );
        ui.label(tr(language, "链状态服务绑定", "Chain Status Bind"));
        ui.text_edit_singleline(&mut config.chain_status_bind);
    });
    ui.horizontal_wrapped(|ui| {
        ui.label(tr(language, "链节点 ID", "Chain Node ID"));
        ui.text_edit_singleline(&mut config.chain_node_id);
        ui.label(tr(language, "链世界 ID", "Chain World ID"));
        ui.text_edit_singleline(&mut config.chain_world_id);
    });
    ui.horizontal_wrapped(|ui| {
        ui.label(tr(language, "链节点角色", "Chain Role"));
        ui.text_edit_singleline(&mut config.chain_node_role);
        ui.label(tr(
            language,
            "链轮询间隔毫秒",
            "Chain Worker Poll/Fallback Milliseconds",
        ));
        ui.text_edit_singleline(&mut config.chain_node_tick_ms);
    });
    ui.horizontal_wrapped(|ui| {
        ui.label(tr(
            language,
            "PoS 槽时长毫秒",
            "PoS Slot Duration Milliseconds",
        ));
        ui.text_edit_singleline(&mut config.chain_pos_slot_duration_ms);
        ui.label(tr(language, "PoS 每槽 Tick 数", "PoS Ticks Per Slot"));
        ui.text_edit_singleline(&mut config.chain_pos_ticks_per_slot);
    });
    ui.horizontal_wrapped(|ui| {
        ui.label(tr(
            language,
            "PoS 提案 Tick 相位",
            "PoS Proposal Tick Phase",
        ));
        ui.text_edit_singleline(&mut config.chain_pos_proposal_tick_phase);
        ui.label(tr(language, "PoS 过旧槽滞后上限", "PoS Max Past Slot Lag"));
        ui.text_edit_singleline(&mut config.chain_pos_max_past_slot_lag);
    });
    ui.horizontal_wrapped(|ui| {
        ui.checkbox(
            &mut config.chain_pos_adaptive_tick_scheduler_enabled,
            tr(
                language,
                "启用 PoS 自适应 Tick 调度",
                "Enable PoS Adaptive Tick Scheduler",
            ),
        );
        ui.label(tr(
            language,
            "PoS 槽时钟起点 Unix 毫秒（可留空）",
            "PoS Slot Clock Genesis Unix Ms (optional)",
        ));
        ui.text_edit_singleline(&mut config.chain_pos_slot_clock_genesis_unix_ms);
    });
    ui.horizontal_wrapped(|ui| {
        ui.label(tr(language, "链验证者", "Chain Validators"));
        ui.text_edit_singleline(&mut config.chain_node_validators);
    });
    ui.horizontal_wrapped(|ui| {
        ui.label(tr(language, "链运行时二进制路径", "Chain Runtime Binary"));
        ui.text_edit_singleline(&mut config.chain_runtime_bin);
    });
}

fn load_llm_settings_from_config(path: &Path) -> Result<LlmSettingsDraft, String> {
    let raw = match fs::read_to_string(path) {
        Ok(content) => content,
        Err(err) if err.kind() == ErrorKind::NotFound => return Ok(LlmSettingsDraft::default()),
        Err(err) => {
            return Err(format!(
                "read config `{}` failed: {err}",
                path.to_string_lossy()
            ));
        }
    };
    if raw.trim().is_empty() {
        return Ok(LlmSettingsDraft::default());
    }

    let table: toml::value::Table = toml::from_str(raw.as_str()).map_err(|err| {
        format!(
            "parse config `{}` as TOML failed: {err}",
            path.to_string_lossy()
        )
    })?;
    let llm_table = table
        .get("llm")
        .and_then(toml::Value::as_table)
        .cloned()
        .unwrap_or_default();

    Ok(LlmSettingsDraft {
        api_key: string_like_value(llm_table.get("api_key")),
        base_url: string_like_value(llm_table.get("base_url")),
        model: string_like_value(llm_table.get("model")),
    })
}

fn save_llm_settings_to_config(path: &Path, draft: &LlmSettingsDraft) -> Result<(), String> {
    let mut root = load_or_init_root_table(path)?;

    let llm_entry = root
        .entry("llm".to_string())
        .or_insert_with(|| toml::Value::Table(toml::value::Table::new()));
    if !llm_entry.is_table() {
        *llm_entry = toml::Value::Table(toml::value::Table::new());
    }
    let llm_table = llm_entry
        .as_table_mut()
        .ok_or_else(|| "llm entry must be table".to_string())?;

    set_string_or_remove(llm_table, "api_key", draft.api_key.as_str());
    set_string_or_remove(llm_table, "base_url", draft.base_url.as_str());
    set_string_or_remove(llm_table, "model", draft.model.as_str());

    let rendered = toml::to_string_pretty(&toml::Value::Table(root))
        .map_err(|err| format!("render config TOML failed: {err}"))?;
    fs::write(path, rendered)
        .map_err(|err| format!("write config `{}` failed: {err}", path.to_string_lossy()))?;

    Ok(())
}

fn load_or_init_root_table(path: &Path) -> Result<toml::value::Table, String> {
    let raw = match fs::read_to_string(path) {
        Ok(content) => content,
        Err(err) if err.kind() == ErrorKind::NotFound => return Ok(toml::value::Table::new()),
        Err(err) => {
            return Err(format!(
                "read config `{}` failed: {err}",
                path.to_string_lossy()
            ));
        }
    };
    if raw.trim().is_empty() {
        return Ok(toml::value::Table::new());
    }

    toml::from_str(raw.as_str()).map_err(|err| {
        format!(
            "parse config `{}` as TOML failed: {err}",
            path.to_string_lossy()
        )
    })
}

fn set_string_or_remove(table: &mut toml::value::Table, key: &str, value: &str) {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        table.remove(key);
        return;
    }
    table.insert(key.to_string(), toml::Value::String(trimmed.to_string()));
}

fn string_like_value(value: Option<&toml::Value>) -> String {
    match value {
        Some(toml::Value::String(raw)) => raw.trim().to_string(),
        Some(toml::Value::Integer(raw)) => raw.to_string(),
        Some(toml::Value::Float(raw)) => raw.to_string(),
        Some(toml::Value::Boolean(raw)) => raw.to_string(),
        _ => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::{LlmSettingsDraft, load_llm_settings_from_config, save_llm_settings_to_config};
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn load_llm_settings_reads_lowercase_toml_fields() {
        let path = temp_config_path("load_llm_settings_reads_lowercase_toml_fields");
        fs::write(
            &path,
            r#"
[llm]
api_key = "token-a"
base_url = "https://example.test/v1"
model = "model-x"
"#,
        )
        .expect("write config");

        let settings = load_llm_settings_from_config(path.as_path()).expect("load settings");
        assert_eq!(settings.api_key, "token-a");
        assert_eq!(settings.base_url, "https://example.test/v1");
        assert_eq!(settings.model, "model-x");
    }

    #[test]
    fn save_llm_settings_updates_llm_table_without_touching_other_tables() {
        let path =
            temp_config_path("save_llm_settings_updates_llm_table_without_touching_other_tables");
        fs::write(
            &path,
            r#"
[node]
private_key = "abc"
public_key = "def"

[llm]
timeout_ms = 8000
"#,
        )
        .expect("write config");

        let draft = LlmSettingsDraft {
            api_key: "token-b".to_string(),
            base_url: "https://endpoint.test/v3".to_string(),
            model: "model-y".to_string(),
        };
        save_llm_settings_to_config(path.as_path(), &draft).expect("save settings");

        let raw = fs::read_to_string(&path).expect("read config");
        let value: toml::Value = toml::from_str(raw.as_str()).expect("parse toml");
        let llm = value
            .get("llm")
            .and_then(toml::Value::as_table)
            .expect("llm table");
        assert_eq!(
            llm.get("api_key").and_then(toml::Value::as_str),
            Some("token-b")
        );
        assert_eq!(
            llm.get("base_url").and_then(toml::Value::as_str),
            Some("https://endpoint.test/v3")
        );
        assert_eq!(
            llm.get("model").and_then(toml::Value::as_str),
            Some("model-y")
        );
        assert_eq!(
            llm.get("timeout_ms").and_then(toml::Value::as_integer),
            Some(8000)
        );

        let node = value
            .get("node")
            .and_then(toml::Value::as_table)
            .expect("node table");
        assert_eq!(
            node.get("private_key").and_then(toml::Value::as_str),
            Some("abc")
        );
    }

    #[test]
    fn save_llm_settings_removes_empty_keys() {
        let path = temp_config_path("save_llm_settings_removes_empty_keys");
        fs::write(
            &path,
            r#"
[llm]
api_key = "token-a"
base_url = "https://example.test/v1"
model = "model-a"
"#,
        )
        .expect("write config");

        let draft = LlmSettingsDraft {
            api_key: "".to_string(),
            base_url: "   ".to_string(),
            model: "".to_string(),
        };
        save_llm_settings_to_config(path.as_path(), &draft).expect("save settings");

        let raw = fs::read_to_string(&path).expect("read config");
        let value: toml::Value = toml::from_str(raw.as_str()).expect("parse toml");
        let llm = value
            .get("llm")
            .and_then(toml::Value::as_table)
            .expect("llm table");
        assert!(!llm.contains_key("api_key"));
        assert!(!llm.contains_key("base_url"));
        assert!(!llm.contains_key("model"));
    }

    #[test]
    fn load_llm_settings_reports_invalid_toml() {
        let path = temp_config_path("load_llm_settings_reports_invalid_toml");
        fs::write(&path, "[llm").expect("write invalid config");
        let err = load_llm_settings_from_config(path.as_path()).expect_err("should fail");
        assert!(err.contains("parse config"));
    }

    fn temp_config_path(test_name: &str) -> PathBuf {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("oasis7_launcher_llm_settings_{now}"));
        fs::create_dir_all(&dir).expect("create temp dir");
        dir.join(format!("{test_name}.toml"))
    }
}
