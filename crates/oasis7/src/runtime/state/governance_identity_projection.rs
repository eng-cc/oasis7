use super::GovernanceIdentityProfileState;
use super::projection::GovernanceIdentityProfileOverlay;
use serde::Serialize;
use serde::ser::{SerializeMap, Serializer};
use std::collections::BTreeMap;

pub(super) struct GovernanceIdentityProfileMapProjection<'a> {
    pub(super) profiles: &'a BTreeMap<String, GovernanceIdentityProfileState>,
    pub(super) overlay: &'a GovernanceIdentityProfileOverlay,
}

impl Serialize for GovernanceIdentityProfileMapProjection<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let target_exists = self.profiles.contains_key(&self.overlay.target_agent_id);
        let mut map = serializer.serialize_map(Some(
            self.profiles.len() + usize::from(!target_exists && self.overlay.allow_insert),
        ))?;
        let mut inserted = false;
        for (agent_id, profile) in self.profiles {
            if agent_id == &self.overlay.target_agent_id {
                map.serialize_entry(agent_id, &self.overlay.next_profile)?;
                inserted = true;
            } else if self.overlay.allow_insert
                && !target_exists
                && !inserted
                && self.overlay.target_agent_id.as_str() < agent_id.as_str()
            {
                map.serialize_entry(&self.overlay.target_agent_id, &self.overlay.next_profile)?;
                map.serialize_entry(agent_id, profile)?;
                inserted = true;
            } else {
                map.serialize_entry(agent_id, profile)?;
            }
        }
        if self.overlay.allow_insert && !inserted {
            map.serialize_entry(&self.overlay.target_agent_id, &self.overlay.next_profile)?;
        }
        map.end()
    }
}
