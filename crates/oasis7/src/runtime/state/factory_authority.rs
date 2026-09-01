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

fn require_active_location_anchor(
    anchors: &BTreeMap<String, LocationAnchorV1>,
    location_id: &str,
    now: WorldTime,
) -> Result<(), WorldError> {
    let Some(anchor) = anchors.get(location_id) else {
        return Err(WorldError::ResourceBalanceInvalid {
            reason: format!("location anchor unknown: {location_id}"),
        });
    };
    if anchor.location_id != location_id || !anchor.active || anchor.authority_revision == 0 {
        return Err(WorldError::ResourceBalanceInvalid {
            reason: format!(
                "location anchor inactive_or_stale: location_id={} revision={} active={}",
                location_id, anchor.authority_revision, anchor.active
            ),
        });
    }
    if anchor.effective_at > now {
        return Err(WorldError::ResourceBalanceInvalid {
            reason: format!(
                "location anchor not yet effective: location_id={} effective_at={} now={}",
                location_id, anchor.effective_at, now
            ),
        });
    }
    Ok(())
}

impl WorldState {
    pub(crate) fn active_location_anchor_revision(
        &self,
        location_id: &str,
        now: WorldTime,
    ) -> Result<u64, WorldError> {
        require_active_location_anchor(&self.location_anchors, location_id, now)?;
        Ok(self
            .location_anchors
            .get(location_id)
            .expect("active location anchor was validated")
            .authority_revision)
    }

    pub(super) fn apply_location_anchor_updated(
        &mut self,
        anchor: &LocationAnchorV1,
    ) -> Result<(), WorldError> {
        require_nonempty(anchor.location_id.as_str(), "location_id")?;
        let current = self
            .location_anchors
            .get(anchor.location_id.as_str())
            .map(|record| record.authority_revision);
        if let Some(existing) = self.location_anchors.get(anchor.location_id.as_str()) {
            if existing == anchor {
                return Ok(());
            }
        }
        next_revision(current, anchor.authority_revision, "location anchor")?;
        self.location_anchors
            .insert(anchor.location_id.clone(), anchor.clone());
        Ok(())
    }

    pub(super) fn apply_agent_location_authority_updated(
        &mut self,
        authority: &AgentLocationAuthorityV1,
        now: WorldTime,
    ) -> Result<(), WorldError> {
        require_nonempty(authority.agent_id.as_str(), "agent_id")?;
        require_nonempty(authority.location_id.as_str(), "location_id")?;
        if !self.agents.contains_key(authority.agent_id.as_str()) {
            return Err(WorldError::AgentNotFound {
                agent_id: authority.agent_id.clone(),
            });
        }
        require_active_location_anchor(
            &self.location_anchors,
            authority.location_id.as_str(),
            now,
        )?;
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
        now: WorldTime,
    ) -> Result<(), WorldError> {
        let mut normalized = authority.clone();
        normalize_allowlist(&mut normalized)?;
        require_active_location_anchor(
            &self.location_anchors,
            normalized.location_id.as_str(),
            now,
        )?;
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
