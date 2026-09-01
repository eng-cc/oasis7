use super::*;

fn require_nonempty(value: &str, field: &str) -> Result<(), WorldError> {
    if value.trim().is_empty() {
        return Err(WorldError::ResourceBalanceInvalid {
            reason: format!("{field} cannot be empty"),
        });
    }
    Ok(())
}

fn next_revision(current: Option<u64>, incoming: u64, subject: &str) -> Result<(), WorldError> {
    let expected = current.map_or(1, |revision| revision.saturating_add(1));
    if incoming != expected {
        return Err(WorldError::ResourceBalanceInvalid {
            reason: format!("{subject} authority revision must be {expected}, got {incoming}"),
        });
    }
    Ok(())
}

fn normalize_allowlist(authority: &mut FactorySiteAuthorityV1) -> Result<(), WorldError> {
    require_nonempty(authority.site_id.as_str(), "site_id")?;
    require_nonempty(authority.location_id.as_str(), "location_id")?;
    require_nonempty(authority.owner_agent_id.as_str(), "owner_agent_id")?;
    if authority
        .authorized_agent_ids
        .iter()
        .any(|agent_id| agent_id.trim().is_empty())
    {
        return Err(WorldError::ResourceBalanceInvalid {
            reason: "authorized_agent_ids cannot contain empty ids".to_string(),
        });
    }
    authority.authorized_agent_ids.sort();
    authority.authorized_agent_ids.dedup();
    Ok(())
}

impl WorldState {
    pub(super) fn apply_agent_location_authority_updated(
        &mut self,
        authority: &AgentLocationAuthorityV1,
    ) -> Result<(), WorldError> {
        require_nonempty(authority.agent_id.as_str(), "agent_id")?;
        require_nonempty(authority.location_id.as_str(), "location_id")?;
        if !self.agents.contains_key(authority.agent_id.as_str()) {
            return Err(WorldError::AgentNotFound {
                agent_id: authority.agent_id.clone(),
            });
        }
        let current = self
            .agent_location_authorities
            .get(authority.agent_id.as_str())
            .map(|record| record.authority_revision);
        if let Some(existing) = self
            .agent_location_authorities
            .get(authority.agent_id.as_str())
        {
            if existing == authority {
                return Ok(());
            }
        }
        next_revision(current, authority.authority_revision, "agent location")?;
        self.agent_location_authorities
            .insert(authority.agent_id.clone(), authority.clone());
        Ok(())
    }

    pub(super) fn apply_factory_site_authority_updated(
        &mut self,
        authority: &FactorySiteAuthorityV1,
    ) -> Result<(), WorldError> {
        let mut normalized = authority.clone();
        normalize_allowlist(&mut normalized)?;
        let current = self
            .factory_site_authorities
            .get(normalized.site_id.as_str())
            .map(|record| record.authority_revision);
        if let Some(existing) = self
            .factory_site_authorities
            .get(normalized.site_id.as_str())
        {
            if existing == &normalized {
                return Ok(());
            }
        }
        next_revision(current, normalized.authority_revision, "factory site")?;
        self.factory_site_authorities
            .insert(normalized.site_id.clone(), normalized);
        Ok(())
    }

    pub(super) fn apply_factory_construction_power_profile_updated(
        &mut self,
        profile: &FactoryConstructionPowerProfileV1,
    ) -> Result<(), WorldError> {
        require_nonempty(profile.factory_id.as_str(), "factory_id")?;
        require_nonempty(profile.factory_kind.as_str(), "factory_kind")?;
        if profile.electricity_amount < 0 {
            return Err(WorldError::ResourceBalanceInvalid {
                reason: format!(
                    "construction electricity amount must be >= 0, got {}",
                    profile.electricity_amount
                ),
            });
        }
        if profile
            .source_module_id
            .as_deref()
            .is_some_and(|module_id| module_id.trim().is_empty())
        {
            return Err(WorldError::ResourceBalanceInvalid {
                reason: "construction profile source_module_id cannot be empty".to_string(),
            });
        }
        let current = self
            .factory_construction_power_profiles
            .get(profile.factory_id.as_str())
            .map(|record| record.authority_revision);
        if let Some(existing) = self
            .factory_construction_power_profiles
            .get(profile.factory_id.as_str())
        {
            if existing == profile {
                return Ok(());
            }
        }
        next_revision(
            current,
            profile.authority_revision,
            "construction power profile",
        )?;
        self.factory_construction_power_profiles
            .insert(profile.factory_id.clone(), profile.clone());
        Ok(())
    }
}
