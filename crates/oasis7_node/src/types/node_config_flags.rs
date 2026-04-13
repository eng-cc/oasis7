use super::*;

impl NodeConfig {
    pub fn with_auto_attest_all_validators(mut self, enabled: bool) -> Self {
        self.auto_attest_all_validators = enabled;
        self
    }

    pub fn with_allow_local_proposals(mut self, enabled: bool) -> Self {
        self.allow_local_proposals = enabled;
        self
    }

    pub fn with_require_execution_on_commit(mut self, enabled: bool) -> Self {
        self.require_execution_on_commit = enabled;
        self
    }

    pub fn with_require_peer_execution_hashes(mut self, enabled: bool) -> Self {
        self.require_peer_execution_hashes = enabled;
        self
    }
}
