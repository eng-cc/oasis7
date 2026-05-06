use crate::geometry::GeoPos;
use serde::{Deserialize, Serialize};

use super::types::{AgentId, LocationId};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
pub enum ModuleVisualAnchor {
    Agent { agent_id: AgentId },
    Location { location_id: LocationId },
    Absolute { pos: GeoPos },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ModuleVisualEntity {
    pub entity_id: String,
    pub module_id: String,
    pub kind: String,
    pub label: Option<String>,
    pub anchor: ModuleVisualAnchor,
}

impl Default for ModuleVisualEntity {
    fn default() -> Self {
        Self {
            entity_id: String::new(),
            module_id: String::new(),
            kind: "artifact".to_string(),
            label: None,
            anchor: ModuleVisualAnchor::Absolute {
                pos: GeoPos::new(0, 0, 0),
            },
        }
    }
}

impl ModuleVisualEntity {
    pub fn sanitized(mut self) -> Self {
        self.entity_id = self.entity_id.trim().to_string();
        self.module_id = self.module_id.trim().to_string();
        self.kind = if self.kind.trim().is_empty() {
            "artifact".to_string()
        } else {
            self.kind.trim().to_string()
        };
        self.label = self
            .label
            .take()
            .map(|label| label.trim().to_string())
            .filter(|label| !label.is_empty());
        self
    }

    pub fn resolved_label(&self) -> String {
        if let Some(label) = self.label.as_deref() {
            if !label.trim().is_empty() {
                return label.to_string();
            }
        }
        if self.kind.trim().is_empty() {
            return self.entity_id.clone();
        }
        format!("{}:{}", self.kind, self.entity_id)
    }
}
