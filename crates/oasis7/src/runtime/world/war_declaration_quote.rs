use super::super::M5_GAMEPLAY_WAR_MODULE_ID;
use super::World;
use crate::runtime::util::sha256_hex;

const WAR_DECLARATION_QUOTE_MISSING: &str = "war_declaration_quote_missing";
const WAR_MAX_INTENSITY: u32 = 10;
const WAR_DURATION_BASE_TICKS: u64 = 6;
const WAR_DURATION_TICKS_PER_INTENSITY: u64 = 2;
const WAR_SCORE_PER_MEMBER: i64 = 10;
const WAR_SCORE_REPUTATION_DIVISOR: i64 = 10;

/// A non-mutating projection of the core war settlement currently available to
/// the caller. It deliberately contains no module reward guarantee.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WarDeclarationQuote {
    pub actor_alliance_id: String,
    pub target_alliance_id: String,
    pub action_kind: String,
    pub intensity: u32,
    pub settlement_path: String,
    pub minimum_winning_intensity: Option<u32>,
    pub war_duration_ticks: u64,
    pub aggressor_score_estimate: i64,
    pub defender_score_estimate: i64,
    pub likely_winner_before_action: String,
    pub victory_margin_estimate: i64,
    pub conflict_window_blocked_until: u64,
    pub reentry_cooldown_or_active_conflict_blocker: String,
    pub expected_narrative_or_module_reward: String,
    pub settlement_risk: String,
    pub alternative_action: String,
    pub recommended_war_action: String,
    pub why_this_war_is_worth_or_risky: String,
    /// Advisory snapshot metadata; it never reserves resources or a conflict slot.
    pub quoted_at_tick: u64,
    /// Stable fingerprint of the quote inputs read from the authoritative core state.
    pub state_fingerprint: String,
}

impl World {
    /// Derives the core settlement projection without submitting an action,
    /// consuming mobilization resources, or writing an event.
    pub fn war_declaration_quote(
        &self,
        actor_id: &str,
        aggressor_alliance_id: &str,
        defender_alliance_id: &str,
        intensity: u32,
    ) -> Result<WarDeclarationQuote, &'static str> {
        let actor_id = actor_id.trim();
        let aggressor_alliance_id = aggressor_alliance_id.trim();
        let defender_alliance_id = defender_alliance_id.trim();
        if actor_id.is_empty()
            || aggressor_alliance_id.is_empty()
            || defender_alliance_id.is_empty()
            || aggressor_alliance_id == defender_alliance_id
            || intensity == 0
            || intensity > WAR_MAX_INTENSITY
            || !self.state.agents.contains_key(actor_id)
        {
            return Err(WAR_DECLARATION_QUOTE_MISSING);
        }

        // The m5 reducer owns fatigue and exposes no authoritative readback
        // contract. Returning a core score here would misrepresent the active
        // settlement path, so fail closed until that contract exists.
        if self
            .module_registry
            .active
            .contains_key(M5_GAMEPLAY_WAR_MODULE_ID)
        {
            return Err(WAR_DECLARATION_QUOTE_MISSING);
        }

        let Some(aggressor) = self.state.alliances.get(aggressor_alliance_id) else {
            return Err(WAR_DECLARATION_QUOTE_MISSING);
        };
        let Some(defender) = self.state.alliances.get(defender_alliance_id) else {
            return Err(WAR_DECLARATION_QUOTE_MISSING);
        };
        if aggressor.members.len() < 2
            || defender.members.len() < 2
            || !aggressor.members.iter().any(|member| member == actor_id)
            || aggressor
                .members
                .iter()
                .any(|member| defender.members.iter().any(|other| other == member))
        {
            return Err(WAR_DECLARATION_QUOTE_MISSING);
        }

        let aggressor_reputation = self.alliance_reputation_total(aggressor_alliance_id);
        let defender_reputation = self.alliance_reputation_total(defender_alliance_id);
        let aggressor_base_score = core_score(aggressor.members.len(), aggressor_reputation);
        let defender_score = core_score(defender.members.len(), defender_reputation);
        let aggressor_score = aggressor_base_score.saturating_add(i64::from(intensity));
        let minimum_winning_intensity = (1..=WAR_MAX_INTENSITY).find(|candidate| {
            aggressor_base_score.saturating_add(i64::from(*candidate)) >= defender_score
        });
        let victory_margin_estimate = aggressor_score.saturating_sub(defender_score);
        let aggressor_wins = victory_margin_estimate >= 0;
        let active_wars = self
            .state
            .wars
            .values()
            .filter(|war| {
                war.active
                    && (war.aggressor_alliance_id == aggressor_alliance_id
                        || war.defender_alliance_id == aggressor_alliance_id
                        || war.aggressor_alliance_id == defender_alliance_id
                        || war.defender_alliance_id == defender_alliance_id)
            })
            .collect::<Vec<_>>();
        let war_duration_ticks = war_duration_ticks(intensity);
        // Submit-time admission rejects while *any* related war is active. The
        // advertised wait boundary is therefore the latest actual core
        // lifecycle due tick among those wars, not merely the first map entry.
        let conflict_window_blocked_until = active_wars
            .iter()
            .map(|war| {
                war.declared_at
                    .saturating_add(war.max_duration_ticks.max(1))
            })
            .max()
            .unwrap_or_else(|| self.state.time.saturating_add(war_duration_ticks));
        let active_blocker = (!active_wars.is_empty()).then(|| {
            let active_ids = active_wars
                .iter()
                .map(|war| war.war_id.as_str())
                .collect::<Vec<_>>()
                .join(",");
            format!("active war {active_ids} blocks either alliance until tick {conflict_window_blocked_until}")
        });
        let (alternative_action, recommended_war_action, why_this_war_is_worth_or_risky) =
            if let Some(blocker) = &active_blocker {
                (
                    "wait".to_string(),
                    "wait".to_string(),
                    format!("{blocker}; wait for the active conflict to conclude."),
                )
            } else if let Some(minimum) = minimum_winning_intensity {
                if intensity < minimum {
                    (
                        "recruit".to_string(),
                        "recruit".to_string(),
                        format!(
                            "current intensity {intensity} is below the minimum winning intensity {minimum}; recruit or increase intensity."
                        ),
                    )
                } else {
                    (
                        "negotiate".to_string(),
                        "declare_war".to_string(),
                        format!(
                            "intensity {intensity} meets the minimum winning intensity {minimum} on the core settlement path."
                        ),
                    )
                }
            } else {
                (
                    "recruit".to_string(),
                    "recruit".to_string(),
                    "no winning intensity is reachable within the authoritative maximum; recruit before declaring war.".to_string(),
                )
            };
        let settlement_risk = if aggressor_wins {
            "core settlement still applies participant resource and reputation changes; optional module rewards are not guaranteed.".to_string()
        } else {
            "loss is projected on the core settlement path; losing members can lose resources and reputation.".to_string()
        };
        let initiator = self
            .state
            .agents
            .get(actor_id)
            .expect("actor existence was checked before quote projection");
        let state_fingerprint = quote_fingerprint(
            self.state.time,
            actor_id,
            aggressor_alliance_id,
            defender_alliance_id,
            intensity,
            &aggressor.members,
            &defender.members,
            aggressor_reputation,
            defender_reputation,
            initiator
                .state
                .resources
                .get(crate::simulator::ResourceKind::Electricity),
            initiator
                .state
                .resources
                .get(crate::simulator::ResourceKind::Data),
            &active_wars,
        );

        Ok(WarDeclarationQuote {
            actor_alliance_id: aggressor_alliance_id.to_string(),
            target_alliance_id: defender_alliance_id.to_string(),
            action_kind: "declare_war".to_string(),
            intensity,
            settlement_path: "core_fallback".to_string(),
            minimum_winning_intensity,
            war_duration_ticks,
            aggressor_score_estimate: aggressor_score,
            defender_score_estimate: defender_score,
            likely_winner_before_action: if aggressor_wins {
                aggressor_alliance_id.to_string()
            } else {
                defender_alliance_id.to_string()
            },
            victory_margin_estimate,
            conflict_window_blocked_until,
            reentry_cooldown_or_active_conflict_blocker: active_blocker.unwrap_or_else(|| {
                "no active war blocks either alliance at this quoted tick".to_string()
            }),
            expected_narrative_or_module_reward:
                "Core guarantees a recorded winner, loser, and war-history outcome; no module reward is guaranteed by this quote.".to_string(),
            settlement_risk,
            alternative_action,
            recommended_war_action,
            why_this_war_is_worth_or_risky,
            quoted_at_tick: self.state.time,
            state_fingerprint,
        })
    }
}

fn core_score(member_count: usize, reputation_total: i64) -> i64 {
    i64::try_from(member_count)
        .unwrap_or(i64::MAX)
        .saturating_mul(WAR_SCORE_PER_MEMBER)
        .saturating_add(reputation_total.saturating_div(WAR_SCORE_REPUTATION_DIVISOR))
}

fn war_duration_ticks(intensity: u32) -> u64 {
    WAR_DURATION_BASE_TICKS
        .saturating_add(u64::from(intensity).saturating_mul(WAR_DURATION_TICKS_PER_INTENSITY))
}

fn quote_fingerprint(
    time: u64,
    actor_id: &str,
    aggressor_alliance_id: &str,
    defender_alliance_id: &str,
    intensity: u32,
    aggressor_members: &[String],
    defender_members: &[String],
    aggressor_reputation: i64,
    defender_reputation: i64,
    initiator_electricity: i64,
    initiator_data: i64,
    active_wars: &[&crate::runtime::WarState],
) -> String {
    let mut aggressor_members = aggressor_members.to_vec();
    aggressor_members.sort();
    let mut defender_members = defender_members.to_vec();
    defender_members.sort();
    let active_wars = active_wars
        .iter()
        .map(|war| {
            format!(
                "{}:{}:{}:{}:{}:{}",
                war.war_id,
                war.aggressor_alliance_id,
                war.defender_alliance_id,
                war.declared_at,
                war.max_duration_ticks,
                war.active
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    sha256_hex(
        format!(
            "war-declaration-quote-v1|core_fallback:no_active_m5_gameplay_war_core|constants:{WAR_MAX_INTENSITY},{WAR_DURATION_BASE_TICKS},{WAR_DURATION_TICKS_PER_INTENSITY},{WAR_SCORE_PER_MEMBER},{WAR_SCORE_REPUTATION_DIVISOR}|{time}|{actor_id}|{aggressor_alliance_id}|{defender_alliance_id}|{intensity}|{}|{}|{aggressor_reputation}|{defender_reputation}|{initiator_electricity}|{initiator_data}|{active_wars}",
            aggressor_members.join(","),
            defender_members.join(","),
        )
        .as_bytes(),
    )
}
