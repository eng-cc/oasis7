use super::NegotiatedViewerProtocol;

pub const SIGNED_AUTHORITATIVE_ROLLBACK_CAPABILITY: &str = "signed_authoritative_rollback_v1";
pub const GOVERNED_ROLLBACK_REPLAY_CAPABILITY: &str = "governed_rollback_replay_v2";

impl NegotiatedViewerProtocol {
    pub fn v1_without_capabilities() -> Self {
        Self {
            version: 1,
            capabilities: Vec::new(),
        }
    }

    pub fn v2_signed_rollback() -> Self {
        Self {
            version: 2,
            capabilities: vec![GOVERNED_ROLLBACK_REPLAY_CAPABILITY.to_string()],
        }
    }

    pub fn supports_signed_rollback(&self) -> bool {
        self.version >= 2
            && self
                .capabilities
                .iter()
                .any(|value| value == GOVERNED_ROLLBACK_REPLAY_CAPABILITY)
    }
}
