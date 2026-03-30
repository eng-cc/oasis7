use super::*;

pub(crate) fn ensure_profile_field_whitelist<T: serde::Serialize>(
    profile: &T,
    allowed_fields: &[&str],
    profile_label: &str,
) -> Result<(), String> {
    let value = serde_json::to_value(profile)
        .map_err(|err| format!("{profile_label} whitelist encode failed: {err}"))?;
    let serde_json::Value::Object(fields) = value else {
        return Err(format!(
            "{profile_label} whitelist rejected: payload is not an object"
        ));
    };
    let allowed: BTreeSet<&str> = allowed_fields.iter().copied().collect();
    for key in fields.keys() {
        if !allowed.contains(key.as_str()) {
            return Err(format!(
                "{profile_label} whitelist rejected: field `{key}` is not allowed"
            ));
        }
    }
    Ok(())
}

pub(super) fn merge_recipe_consume_with_maintenance_sink(
    world: &World,
    consume: &[MaterialStack],
    produce: &[MaterialStack],
) -> Vec<MaterialStack> {
    let mut merged: BTreeMap<String, i64> = BTreeMap::new();
    for stack in consume {
        let entry = merged.entry(stack.kind.clone()).or_insert(0);
        *entry = entry.saturating_add(stack.amount);
    }
    for stack in produce {
        if stack.amount <= 0 {
            continue;
        }
        let Some(profile) = world.product_profile(stack.kind.as_str()) else {
            continue;
        };
        for sink in &profile.maintenance_sink {
            if sink.amount <= 0 {
                continue;
            }
            let required_amount = sink.amount.saturating_mul(stack.amount);
            if required_amount <= 0 {
                continue;
            }
            let entry = merged.entry(sink.kind.clone()).or_insert(0);
            *entry = entry.saturating_add(required_amount);
        }
    }
    merged
        .into_iter()
        .map(|(kind, amount)| MaterialStack::new(kind, amount))
        .collect()
}

fn infer_bottleneck_tags(consume: &[MaterialStack]) -> Vec<String> {
    let tags: BTreeSet<String> = consume
        .iter()
        .filter_map(|stack| {
            let normalized = stack.kind.to_ascii_lowercase();
            BOTTLENECK_TAG_KINDS
                .iter()
                .find(|kind| normalized == **kind)
                .map(|kind| (*kind).to_string())
        })
        .collect();
    tags.into_iter().collect()
}

pub(super) fn compute_local_scarcity_delay_ticks(
    world: &World,
    preferred_consume_ledger: &MaterialLedgerId,
    consume_ledger: &MaterialLedgerId,
    consume: &[MaterialStack],
    bottleneck_tags: &[String],
) -> u32 {
    if *preferred_consume_ledger == MaterialLedgerId::world()
        || *consume_ledger != MaterialLedgerId::world()
        || bottleneck_tags.is_empty()
    {
        return 0;
    }

    let bottleneck_set: BTreeSet<&str> = bottleneck_tags.iter().map(String::as_str).collect();
    let mut requested_total: i128 = 0;
    let mut deficit_total: i128 = 0;
    for stack in consume {
        if stack.amount <= 0 {
            continue;
        }
        let normalized_kind = stack.kind.to_ascii_lowercase();
        if !bottleneck_set.contains(normalized_kind.as_str()) {
            continue;
        }
        let requested = stack.amount as i128;
        let available = world
            .ledger_material_balance(preferred_consume_ledger, stack.kind.as_str())
            .max(0) as i128;
        requested_total = requested_total.saturating_add(requested);
        deficit_total = deficit_total.saturating_add(requested.saturating_sub(available).max(0));
    }

    if requested_total <= 0 || deficit_total <= 0 {
        return 0;
    }
    let deficit_ratio_bps = deficit_total
        .saturating_mul(10_000)
        .saturating_div(requested_total);
    if deficit_ratio_bps >= 7_000 {
        2
    } else {
        1
    }
}

fn governance_tax_bps_for_material_quotes(world: &World) -> u16 {
    world
        .state
        .gameplay_policy
        .electricity_tax_bps
        .saturating_add(world.state.gameplay_policy.data_tax_bps)
        .min(10_000)
}

pub(super) fn build_material_market_quotes(
    world: &World,
    preferred_consume_ledger: &MaterialLedgerId,
    consume: &[MaterialStack],
) -> Vec<MaterialMarketQuote> {
    let mut requested_by_kind: BTreeMap<String, i64> = BTreeMap::new();
    for stack in consume {
        if stack.amount <= 0 {
            continue;
        }
        let entry = requested_by_kind.entry(stack.kind.clone()).or_insert(0);
        *entry = entry.saturating_add(stack.amount);
    }
    if requested_by_kind.is_empty() {
        return Vec::new();
    }

    let governance_tax_bps = governance_tax_bps_for_material_quotes(world);
    let mut quotes = Vec::with_capacity(requested_by_kind.len());
    for (kind, requested_amount) in requested_by_kind {
        let transit_loss_bps = material_transit_loss_bps_for_kind(world, kind.as_str());
        let local_available_amount = world
            .ledger_material_balance(preferred_consume_ledger, kind.as_str())
            .max(0);
        let world_available_amount = world
            .ledger_material_balance(&MaterialLedgerId::world(), kind.as_str())
            .max(0);
        let local_deficit_amount = requested_amount
            .saturating_sub(local_available_amount)
            .max(0);
        let deficit_ratio_bps = if requested_amount > 0 {
            ((local_deficit_amount as i128)
                .saturating_mul(10_000)
                .saturating_div(requested_amount as i128)) as i64
        } else {
            0
        };
        let effective_cost_index_ppm = 1_000_000_i64
            .saturating_add(deficit_ratio_bps.saturating_mul(100))
            .saturating_add(transit_loss_bps.saturating_mul(100))
            .saturating_add(i64::from(governance_tax_bps).saturating_mul(100));
        quotes.push(MaterialMarketQuote {
            kind,
            requested_amount,
            local_available_amount,
            world_available_amount,
            local_deficit_amount,
            transit_loss_bps,
            governance_tax_bps,
            effective_cost_index_ppm,
        });
    }
    quotes
}

pub(super) fn recipe_stage_gate_allowed(
    current_stage: crate::runtime::IndustryStage,
    stage_gate: &str,
) -> bool {
    let normalized = stage_gate.trim();
    if normalized.is_empty() {
        return true;
    }
    let Some(required_stage) = parse_industry_stage(normalized) else {
        return true;
    };
    current_stage >= required_stage
}

pub(super) fn product_unlock_stage_allowed(
    current_stage: crate::runtime::IndustryStage,
    unlock_stage: &str,
) -> bool {
    recipe_stage_gate_allowed(current_stage, unlock_stage)
}

fn parse_industry_stage(raw: &str) -> Option<crate::runtime::IndustryStage> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "bootstrap" => Some(crate::runtime::IndustryStage::Bootstrap),
        "scale_out" | "scaleout" | "scale-out" => Some(crate::runtime::IndustryStage::ScaleOut),
        "governance" => Some(crate::runtime::IndustryStage::Governance),
        _ => None,
    }
}

pub(super) fn industry_stage_label(stage: crate::runtime::IndustryStage) -> &'static str {
    match stage {
        crate::runtime::IndustryStage::Bootstrap => "bootstrap",
        crate::runtime::IndustryStage::ScaleOut => "scale_out",
        crate::runtime::IndustryStage::Governance => "governance",
    }
}

pub(super) fn recipe_preferred_tags_compatible(
    preferred_tags: &[String],
    factory_tags: &[String],
) -> bool {
    if preferred_tags.is_empty() {
        return true;
    }
    let normalized_factory: BTreeSet<String> = factory_tags
        .iter()
        .map(|tag| tag.trim().to_ascii_lowercase())
        .filter(|tag| !tag.is_empty())
        .collect();
    preferred_tags.iter().any(|tag| {
        let normalized = tag.trim().to_ascii_lowercase();
        !normalized.is_empty() && normalized_factory.contains(normalized.as_str())
    })
}

pub(super) fn resolve_recipe_bottleneck_tags(
    recipe_profile: Option<&crate::runtime::RecipeProfileV1>,
    consume: &[MaterialStack],
) -> Vec<String> {
    let from_profile: BTreeSet<String> = recipe_profile
        .map(|profile| {
            profile
                .bottleneck_tags
                .iter()
                .map(|tag| tag.trim().to_ascii_lowercase())
                .filter(|tag| !tag.is_empty())
                .collect()
        })
        .unwrap_or_default();
    if !from_profile.is_empty() {
        return from_profile.into_iter().collect();
    }
    infer_bottleneck_tags(consume)
}

pub(super) fn material_transit_loss_bps_for_kind(world: &World, kind: &str) -> i64 {
    let base = MATERIAL_TRANSFER_LOSS_PER_KM_BPS.max(0);
    let factor = world
        .material_profile(kind)
        .map(|profile| match profile.transport_loss_class {
            crate::runtime::MaterialTransportLossClass::Low => 1_i64,
            crate::runtime::MaterialTransportLossClass::Medium => 2_i64,
            crate::runtime::MaterialTransportLossClass::High => 4_i64,
        })
        .unwrap_or(1);
    base.saturating_mul(factor)
}
