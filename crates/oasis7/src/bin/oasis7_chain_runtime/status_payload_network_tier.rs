use serde::Serialize;

#[derive(Debug, Serialize)]
pub(crate) struct ChainNetworkTierStatus {
    pub(crate) source_path: String,
    pub(crate) schema_version: String,
    pub(crate) tier: String,
    pub(crate) status: String,
    pub(crate) network_id: String,
    pub(crate) chain_id: String,
    pub(crate) bootstrap_peer_count: usize,
    pub(crate) governance_mode: String,
    pub(crate) validator_admission: String,
    pub(crate) target_validator_count: u64,
    pub(crate) allow_observer_nodes: bool,
    pub(crate) token_symbol: String,
    pub(crate) faucet_mode: String,
    pub(crate) reset_policy: String,
    pub(crate) value_semantics: String,
    pub(crate) rpc_ref: String,
    pub(crate) explorer_ref: String,
    pub(crate) faucet_ref: Option<String>,
    pub(crate) required_gates: Vec<String>,
    pub(crate) allowed_claims: Vec<String>,
    pub(crate) denied_claims: Vec<String>,
}
