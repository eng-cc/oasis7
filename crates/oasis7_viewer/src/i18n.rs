use bevy::prelude::*;

use super::{SelectionKind, ViewerCameraMode, ViewerControl, ViewerExperienceMode};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum UiLocale {
    ZhCn,
    EnUs,
}

#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct UiI18n {
    pub locale: UiLocale,
}

impl Default for UiI18n {
    fn default() -> Self {
        Self {
            locale: UiLocale::ZhCn,
        }
    }
}

impl UiLocale {
    pub(super) fn toggled(self) -> Self {
        match self {
            UiLocale::ZhCn => UiLocale::EnUs,
            UiLocale::EnUs => UiLocale::ZhCn,
        }
    }

    pub(super) fn is_zh(self) -> bool {
        matches!(self, UiLocale::ZhCn)
    }
}

pub(super) fn locale_or_default(i18n: Option<&UiI18n>) -> UiLocale {
    i18n.map(|value| value.locale).unwrap_or(UiLocale::EnUs)
}

pub(super) fn top_panel_toggle_label(collapsed: bool, locale: UiLocale) -> &'static str {
    match (locale, collapsed) {
        (UiLocale::ZhCn, false) => "隐藏顶部",
        (UiLocale::ZhCn, true) => "显示顶部",
        (UiLocale::EnUs, false) => "Hide Top",
        (UiLocale::EnUs, true) => "Show Top",
    }
}

pub(super) fn top_controls_label(locale: UiLocale) -> &'static str {
    if locale.is_zh() {
        "顶部控制区"
    } else {
        "Top Controls"
    }
}

pub(super) fn language_toggle_label(locale: UiLocale) -> &'static str {
    if locale.is_zh() {
        "语言：中文"
    } else {
        "Language: English"
    }
}

pub(super) fn camera_mode_section_label(locale: UiLocale) -> &'static str {
    if locale.is_zh() {
        "视角"
    } else {
        "View"
    }
}

pub(super) fn camera_mode_button_label(mode: ViewerCameraMode, locale: UiLocale) -> &'static str {
    match (mode, locale) {
        (ViewerCameraMode::TwoD, UiLocale::ZhCn) => "2D",
        (ViewerCameraMode::TwoD, UiLocale::EnUs) => "2D",
        (ViewerCameraMode::ThreeD, UiLocale::ZhCn) => "3D",
        (ViewerCameraMode::ThreeD, UiLocale::EnUs) => "3D",
    }
}

pub(super) fn copyable_panel_toggle_label(visible: bool, locale: UiLocale) -> &'static str {
    match (locale, visible) {
        (UiLocale::ZhCn, true) => "隐藏明细",
        (UiLocale::ZhCn, false) => "显示明细",
        (UiLocale::EnUs, true) => "Hide Details",
        (UiLocale::EnUs, false) => "Show Details",
    }
}

pub(super) fn right_panel_toggle_label(visible: bool, locale: UiLocale) -> &'static str {
    match (locale, visible) {
        (UiLocale::ZhCn, true) => "隐藏面板",
        (UiLocale::ZhCn, false) => "显示面板",
        (UiLocale::EnUs, true) => "Hide Panel",
        (UiLocale::EnUs, false) => "Show Panel",
    }
}

pub(super) fn experience_mode_label(mode: ViewerExperienceMode, locale: UiLocale) -> &'static str {
    match (mode, locale) {
        (ViewerExperienceMode::Player, UiLocale::ZhCn) => "玩家模式",
        (ViewerExperienceMode::Player, UiLocale::EnUs) => "Player Mode",
        (ViewerExperienceMode::Director, UiLocale::ZhCn) => "导演模式",
        (ViewerExperienceMode::Director, UiLocale::EnUs) => "Director Mode",
    }
}

pub(super) fn panel_entry_hint_label(mode: ViewerExperienceMode, locale: UiLocale) -> &'static str {
    match (mode, locale) {
        (ViewerExperienceMode::Player, UiLocale::ZhCn) => {
            "世界优先视图已启用。打开面板可查看任务、事件与更多控制。"
        }
        (ViewerExperienceMode::Player, UiLocale::EnUs) => {
            "World-first view is active. Open the panel for tasks, events, and controls."
        }
        (ViewerExperienceMode::Director, UiLocale::ZhCn) => {
            "可打开右侧面板查看完整运行状态与调试模块。"
        }
        (ViewerExperienceMode::Director, UiLocale::EnUs) => {
            "Open the side panel for full runtime status and debug modules."
        }
    }
}

pub(super) fn panel_toggle_shortcut_hint(locale: UiLocale) -> &'static str {
    if locale.is_zh() {
        "快捷键：Tab"
    } else {
        "Shortcut: Tab"
    }
}

pub(super) fn module_switches_title(locale: UiLocale) -> &'static str {
    if locale.is_zh() {
        "模块开关"
    } else {
        "Modules"
    }
}

pub(super) fn module_toggle_label(module_key: &str, visible: bool, locale: UiLocale) -> String {
    let (zh, en) = match module_key {
        "controls" => ("控制", "Controls"),
        "overview" => ("总览", "Overview"),
        "chat" => ("对话", "Chat"),
        "overlay" => ("覆盖层", "Overlay"),
        "diagnosis" => ("诊断", "Diagnosis"),
        "event_link" => ("事件联动", "Event Link"),
        "timeline" => ("时间轴", "Timeline"),
        "details" => ("明细", "Details"),
        _ => ("模块", "Module"),
    };

    let state = if locale.is_zh() {
        if visible {
            "开"
        } else {
            "关"
        }
    } else if visible {
        "on"
    } else {
        "off"
    };

    if locale.is_zh() {
        format!("{zh}:{state}")
    } else {
        format!("{en}:{state}")
    }
}

pub(super) fn control_button_label(control: &ViewerControl, locale: UiLocale) -> &'static str {
    match control {
        ViewerControl::Play => {
            if locale.is_zh() {
                "播放"
            } else {
                "Play"
            }
        }
        ViewerControl::Pause => {
            if locale.is_zh() {
                "暂停"
            } else {
                "Pause"
            }
        }
        ViewerControl::Step { .. } => {
            if locale.is_zh() {
                "单步"
            } else {
                "Step"
            }
        }
        ViewerControl::Seek { .. } => {
            if locale.is_zh() {
                "跳转 0"
            } else {
                "Seek 0"
            }
        }
    }
}

pub(super) fn play_pause_toggle_label(playing: bool, locale: UiLocale) -> &'static str {
    if playing {
        control_button_label(&ViewerControl::Pause, locale)
    } else {
        control_button_label(&ViewerControl::Play, locale)
    }
}

pub(super) fn advanced_debug_toggle_label(expanded: bool, locale: UiLocale) -> String {
    if locale.is_zh() {
        format!("高级调试:{}", if expanded { "开" } else { "关" })
    } else {
        format!("Advanced Debug:{}", if expanded { "on" } else { "off" })
    }
}

pub(super) fn step_button_label(locale: UiLocale, pending: bool) -> &'static str {
    if locale.is_zh() {
        if pending {
            "单步 ..."
        } else {
            "单步"
        }
    } else if pending {
        "Step ..."
    } else {
        "Step"
    }
}

#[allow(dead_code)]
pub(super) fn selection_kind_label(kind: SelectionKind, locale: UiLocale) -> &'static str {
    match kind {
        SelectionKind::Agent => {
            if locale.is_zh() {
                "agent"
            } else {
                "agent"
            }
        }
        SelectionKind::Location => {
            if locale.is_zh() {
                "地点"
            } else {
                "location"
            }
        }
        SelectionKind::Fragment => {
            if locale.is_zh() {
                "碎片"
            } else {
                "fragment"
            }
        }
        SelectionKind::Asset => {
            if locale.is_zh() {
                "资产"
            } else {
                "asset"
            }
        }
        SelectionKind::PowerPlant => {
            if locale.is_zh() {
                "电厂"
            } else {
                "power_plant"
            }
        }
        SelectionKind::Chunk => {
            if locale.is_zh() {
                "分块"
            } else {
                "chunk"
            }
        }
    }
}

pub(super) fn on_off_label(enabled: bool, locale: UiLocale) -> &'static str {
    if locale.is_zh() {
        if enabled {
            "开"
        } else {
            "关"
        }
    } else if enabled {
        "on"
    } else {
        "off"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_locale_is_chinese() {
        assert_eq!(UiI18n::default().locale, UiLocale::ZhCn);
    }

    #[test]
    fn locale_toggle_round_trip() {
        assert_eq!(UiLocale::ZhCn.toggled(), UiLocale::EnUs);
        assert_eq!(UiLocale::EnUs.toggled(), UiLocale::ZhCn);
    }

    #[test]
    fn copyable_toggle_label_is_localized() {
        assert_eq!(
            copyable_panel_toggle_label(true, UiLocale::ZhCn),
            "隐藏明细"
        );
        assert_eq!(
            copyable_panel_toggle_label(false, UiLocale::ZhCn),
            "显示明细"
        );
        assert_eq!(
            copyable_panel_toggle_label(true, UiLocale::EnUs),
            "Hide Details"
        );
        assert_eq!(
            copyable_panel_toggle_label(false, UiLocale::EnUs),
            "Show Details"
        );
        assert_eq!(right_panel_toggle_label(true, UiLocale::ZhCn), "隐藏面板");
        assert_eq!(right_panel_toggle_label(false, UiLocale::ZhCn), "显示面板");
        assert_eq!(right_panel_toggle_label(true, UiLocale::EnUs), "Hide Panel");
        assert_eq!(
            right_panel_toggle_label(false, UiLocale::EnUs),
            "Show Panel"
        );
        assert_eq!(
            experience_mode_label(ViewerExperienceMode::Player, UiLocale::ZhCn),
            "玩家模式"
        );
        assert_eq!(
            experience_mode_label(ViewerExperienceMode::Director, UiLocale::EnUs),
            "Director Mode"
        );
        assert_eq!(
            panel_entry_hint_label(ViewerExperienceMode::Player, UiLocale::EnUs),
            "World-first view is active. Open the panel for tasks, events, and controls."
        );
        assert_eq!(panel_toggle_shortcut_hint(UiLocale::ZhCn), "快捷键：Tab");
    }

    #[test]
    fn camera_mode_labels_are_stable() {
        assert_eq!(camera_mode_section_label(UiLocale::ZhCn), "视角");
        assert_eq!(camera_mode_section_label(UiLocale::EnUs), "View");
        assert_eq!(
            camera_mode_button_label(ViewerCameraMode::TwoD, UiLocale::ZhCn),
            "2D"
        );
        assert_eq!(
            camera_mode_button_label(ViewerCameraMode::ThreeD, UiLocale::EnUs),
            "3D"
        );
    }

    #[test]
    fn module_toggle_label_is_localized_and_stateful() {
        assert_eq!(module_switches_title(UiLocale::ZhCn), "模块开关");
        assert_eq!(module_switches_title(UiLocale::EnUs), "Modules");
        assert_eq!(
            module_toggle_label("controls", true, UiLocale::ZhCn),
            "控制:开"
        );
        assert_eq!(
            module_toggle_label("timeline", false, UiLocale::EnUs),
            "Timeline:off"
        );
    }

    #[test]
    fn play_pause_and_advanced_debug_labels_are_stateful() {
        assert_eq!(play_pause_toggle_label(false, UiLocale::ZhCn), "播放");
        assert_eq!(play_pause_toggle_label(true, UiLocale::ZhCn), "暂停");
        assert_eq!(play_pause_toggle_label(false, UiLocale::EnUs), "Play");
        assert_eq!(play_pause_toggle_label(true, UiLocale::EnUs), "Pause");

        assert_eq!(
            advanced_debug_toggle_label(false, UiLocale::ZhCn),
            "高级调试:关"
        );
        assert_eq!(
            advanced_debug_toggle_label(true, UiLocale::ZhCn),
            "高级调试:开"
        );
        assert_eq!(
            advanced_debug_toggle_label(false, UiLocale::EnUs),
            "Advanced Debug:off"
        );
        assert_eq!(
            advanced_debug_toggle_label(true, UiLocale::EnUs),
            "Advanced Debug:on"
        );
    }
}
