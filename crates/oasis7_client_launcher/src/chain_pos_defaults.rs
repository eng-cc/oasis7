use std::sync::OnceLock;

const CHAIN_POS_DEFAULTS_ENV: &str = include_str!("../../../config/chain-pos-defaults.env");

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ChainPosTimingDefaults {
    pub(crate) slot_duration_ms: u64,
    pub(crate) ticks_per_slot: u64,
    pub(crate) proposal_tick_phase: u64,
    pub(crate) max_past_slot_lag: u64,
}

static DEFAULTS: OnceLock<ChainPosTimingDefaults> = OnceLock::new();

pub(crate) fn defaults() -> &'static ChainPosTimingDefaults {
    DEFAULTS.get_or_init(|| {
        let slot_duration_ms = parse_required_u64("POS_SLOT_DURATION_MS");
        let ticks_per_slot = parse_required_u64("POS_TICKS_PER_SLOT");
        let proposal_tick_phase = parse_required_u64("POS_PROPOSAL_TICK_PHASE");
        let max_past_slot_lag = parse_required_u64("POS_MAX_PAST_SLOT_LAG");

        assert!(
            slot_duration_ms > 0,
            "POS_SLOT_DURATION_MS must be positive"
        );
        assert!(ticks_per_slot > 0, "POS_TICKS_PER_SLOT must be positive");
        assert!(
            proposal_tick_phase < ticks_per_slot,
            "POS_PROPOSAL_TICK_PHASE must be less than POS_TICKS_PER_SLOT"
        );

        ChainPosTimingDefaults {
            slot_duration_ms,
            ticks_per_slot,
            proposal_tick_phase,
            max_past_slot_lag,
        }
    })
}

fn parse_required_u64(key: &str) -> u64 {
    let raw = CHAIN_POS_DEFAULTS_ENV
        .lines()
        .filter_map(parse_env_line)
        .find_map(|(line_key, line_value)| (line_key == key).then_some(line_value))
        .unwrap_or_else(|| panic!("config/chain-pos-defaults.env must define {key}"));
    raw.parse::<u64>()
        .unwrap_or_else(|_| panic!("{key} must be a non-negative integer, got `{raw}`"))
}

fn parse_env_line(line: &str) -> Option<(&str, &str)> {
    let line = line.trim();
    if line.is_empty() || line.starts_with('#') {
        return None;
    }
    let (key, value) = line.split_once('=')?;
    Some((key.trim(), value.trim()))
}

#[cfg(test)]
mod tests {
    use super::defaults;

    #[test]
    fn launcher_pos_defaults_match_the_repository_profile() {
        let defaults = defaults();
        assert_eq!(defaults.slot_duration_ms, 8_000);
        assert_eq!(defaults.ticks_per_slot, 10);
        assert_eq!(defaults.proposal_tick_phase, 9);
        assert_eq!(defaults.max_past_slot_lag, 256);
    }
}
