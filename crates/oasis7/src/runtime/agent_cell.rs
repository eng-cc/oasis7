//! Agent cell representation - wraps agent state with mailbox and activity tracking.

use crate::models::AgentState;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

use super::events::DomainEvent;
use super::types::WorldTime;

/// The authoritative coarse-grained activity state exposed for an agent.
///
/// A missing activity (`AgentCell::activity == None`) is distinct from an
/// explicitly idle agent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentActivityStatus {
    Idle,
    Executing,
    Blocked,
    Waiting,
    Unavailable,
}

/// Versioned activity metadata persisted alongside an agent cell.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentActivityV1 {
    pub status: AgentActivityStatus,
    pub updated_at: WorldTime,
}

impl AgentActivityV1 {
    pub fn idle(updated_at: WorldTime) -> Self {
        Self {
            status: AgentActivityStatus::Idle,
            updated_at,
        }
    }
}

/// A cell that holds an agent's state along with its mailbox and activity tracking.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentCell {
    pub state: AgentState,
    pub mailbox: VecDeque<DomainEvent>,
    pub last_active: WorldTime,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub activity: Option<AgentActivityV1>,
}

impl AgentCell {
    pub fn new(state: AgentState, now: WorldTime) -> Self {
        Self {
            state,
            mailbox: VecDeque::new(),
            last_active: now,
            activity: None,
        }
    }
}
