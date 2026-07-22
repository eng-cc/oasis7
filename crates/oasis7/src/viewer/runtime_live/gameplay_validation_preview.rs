use crate::runtime::{IndustryStage, WorldState};
use crate::simulator::persist::ProductValidationUnlockPreview;

fn stage_label(stage: IndustryStage) -> &'static str {
    match stage {
        IndustryStage::Bootstrap => "bootstrap",
        IndustryStage::ScaleOut => "scale_out",
        IndustryStage::Governance => "governance",
    }
}

fn stage_rank(stage: &str) -> Option<u8> {
    match stage {
        "bootstrap" => Some(0),
        "scale_out" => Some(1),
        "governance" => Some(2),
        _ => None,
    }
}

pub(super) fn product_validation_unlock_preview(
    state: &WorldState,
) -> Option<ProductValidationUnlockPreview> {
    let validation = state.latest_product_validation.as_ref()?;
    let current_stage = stage_label(state.industry_progress.stage).to_string();
    let Some(profile) = state.product_profiles.get(validation.product_id.as_str()) else {
        return Some(ProductValidationUnlockPreview {
            product_id: validation.product_id.clone(),
            role_tag: "unknown".to_string(),
            tradable: validation.tradable,
            required_stage: "unknown".to_string(),
            current_stage,
            stage_status: "unknown".to_string(),
            value_summary: "Validated product has no governed role profile.".to_string(),
            next_step_hint: "Inspect product use before relying on this validation.".to_string(),
        });
    };
    let required_stage = if profile.unlock_stage.trim().is_empty() {
        "unknown".to_string()
    } else {
        profile.unlock_stage.clone()
    };
    let stage_status = match (
        stage_rank(current_stage.as_str()),
        stage_rank(required_stage.as_str()),
    ) {
        (_, None) | (None, Some(_)) => "unknown",
        (Some(current), Some(required)) if current >= required => "available",
        (Some(_), Some(_)) => "denied",
    };
    let role_tag = if profile.role_tag.trim().is_empty() {
        "unknown".to_string()
    } else {
        profile.role_tag.clone()
    };
    let (value_summary, next_step_hint) = match stage_status {
        "available" => (
            format!(
                "Validated {role_tag} product; {}.",
                if validation.tradable {
                    "trading enabled"
                } else {
                    "trading disabled"
                }
            ),
            format!(
                "Use this product in its {role_tag} role; validation unlocks no new capability."
            ),
        ),
        "denied" => (
            format!("Validated {role_tag} product remains gated by stage {required_stage}."),
            format!(
                "Advance industry from {current_stage} to {required_stage}; validation unlocks no new capability."
            ),
        ),
        _ => (
            format!("Validated {role_tag} product has an unknown stage requirement."),
            "Inspect the governed product profile before relying on this validation.".to_string(),
        ),
    };
    Some(ProductValidationUnlockPreview {
        product_id: validation.product_id.clone(),
        role_tag,
        tradable: validation.tradable,
        required_stage,
        current_stage,
        stage_status: stage_status.to_string(),
        value_summary,
        next_step_hint,
    })
}
