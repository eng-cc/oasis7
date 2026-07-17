use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct GovernanceRegistryAuditReport {
    pub(crate) world_dir: String,
    pub(crate) finality: GovernanceSlotAuditRow,
    pub(crate) controllers: Vec<GovernanceSlotAuditRow>,
    pub(crate) rollback_authorities: Vec<RollbackAuthorityAuditRow>,
    pub(crate) rollback_blockers: Vec<String>,
    pub(crate) overall_single_failure_tolerance_pass: bool,
    pub(crate) manifest_match_pass: Option<bool>,
    pub(crate) overall_status: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct RollbackAuthorityAuditRow {
    pub(crate) slot_id: String,
    pub(crate) role: String,
    pub(crate) configured: bool,
    pub(crate) active: bool,
    pub(crate) threshold: u16,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) manifest_match: Option<bool>,
    pub(crate) status: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct GovernanceSlotAuditRow {
    pub(crate) slot_id: String,
    pub(crate) threshold: u16,
    pub(crate) signer_count: usize,
    pub(crate) tolerated_failures: usize,
    pub(crate) single_failure_tolerant: bool,
    pub(crate) threshold_matches_expectation: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) manifest_match: Option<bool>,
    pub(crate) status: String,
}

pub(crate) fn audit_row(
    slot_id: &str,
    threshold: u16,
    signer_count: usize,
    expected_threshold: u16,
    manifest_match: Option<bool>,
) -> GovernanceSlotAuditRow {
    let tolerated_failures = signer_count.saturating_sub(usize::from(threshold));
    let single_failure_tolerant = tolerated_failures >= 1;
    let threshold_matches_expectation = threshold == expected_threshold;
    let status = if !threshold_matches_expectation {
        "threshold_mismatch".to_string()
    } else if !single_failure_tolerant {
        "single_failure_blocks_slot".to_string()
    } else {
        "single_failure_tolerant".to_string()
    };
    GovernanceSlotAuditRow {
        slot_id: slot_id.to_string(),
        threshold,
        signer_count,
        tolerated_failures,
        single_failure_tolerant,
        threshold_matches_expectation,
        manifest_match,
        status,
    }
}
