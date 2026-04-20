use super::*;

use parsed_subscription_filters_types::{FilterKind, MatchRule, RuleSet};

pub(crate) mod parsed_subscription_filters_types {
    use super::*;

    #[derive(Debug, Clone, Deserialize)]
    #[serde(deny_unknown_fields)]
    pub(crate) struct SubscriptionFilters {
        #[serde(default)]
        pub(crate) event: Option<RuleSet>,
        #[serde(default)]
        pub(crate) action: Option<RuleSet>,
    }

    #[derive(Debug, Clone, Deserialize)]
    #[serde(untagged)]
    pub(crate) enum RuleSet {
        List(Vec<MatchRule>),
        Group(RuleGroup),
    }

    #[derive(Debug, Clone, Deserialize)]
    #[serde(deny_unknown_fields)]
    pub(crate) struct RuleGroup {
        #[serde(default)]
        pub(crate) all: Vec<MatchRule>,
        #[serde(default)]
        pub(crate) any: Vec<MatchRule>,
    }

    #[derive(Debug, Clone, Deserialize)]
    #[serde(deny_unknown_fields)]
    pub(crate) struct MatchRule {
        pub(crate) path: String,
        #[serde(default)]
        pub(crate) eq: Option<JsonValue>,
        #[serde(default)]
        pub(crate) ne: Option<JsonValue>,
        #[serde(default)]
        pub(crate) gt: Option<f64>,
        #[serde(default)]
        pub(crate) gte: Option<f64>,
        #[serde(default)]
        pub(crate) lt: Option<f64>,
        #[serde(default)]
        pub(crate) lte: Option<f64>,
        #[serde(default)]
        pub(crate) re: Option<String>,
    }

    #[derive(Debug, Clone, Copy)]
    pub(crate) enum FilterKind {
        Event,
        Action,
    }
}

pub fn validate_subscription_stage(
    subscription: &ModuleSubscription,
    module_id: &str,
) -> Result<(), String> {
    let has_events = !subscription.event_kinds.is_empty();
    let has_actions = !subscription.action_kinds.is_empty();
    match subscription.stage {
        Some(ModuleSubscriptionStage::PostEvent) => {
            if has_actions {
                return Err(format!(
                    "module {module_id} subscription post_event cannot include action_kinds"
                ));
            }
            if !has_events {
                return Err(format!(
                    "module {module_id} subscription post_event requires event_kinds"
                ));
            }
        }
        Some(ModuleSubscriptionStage::PreAction) | Some(ModuleSubscriptionStage::PostAction) => {
            if has_events {
                return Err(format!(
                    "module {module_id} subscription action stage cannot include event_kinds"
                ));
            }
            if !has_actions {
                return Err(format!(
                    "module {module_id} subscription action stage requires action_kinds"
                ));
            }
        }
        Some(ModuleSubscriptionStage::Tick) => {
            if has_events || has_actions {
                return Err(format!(
                    "module {module_id} subscription tick stage cannot include event_kinds or action_kinds"
                ));
            }
            if subscription.filters.is_some()
                && !subscription
                    .filters
                    .as_ref()
                    .is_some_and(|value| value.is_null())
            {
                return Err(format!(
                    "module {module_id} subscription tick stage cannot include filters"
                ));
            }
        }
        None => {
            if has_events && has_actions {
                return Err(format!(
                    "module {module_id} subscription cannot mix event_kinds and action_kinds"
                ));
            }
            if !has_events && !has_actions {
                return Err(format!(
                    "module {module_id} subscription requires event_kinds or action_kinds"
                ));
            }
        }
    }
    Ok(())
}

pub fn validate_subscription_filters(
    filters: &Option<JsonValue>,
    module_id: &str,
) -> Result<(), String> {
    let Some(filters_value) = filters else {
        return Ok(());
    };
    if filters_value.is_null() {
        return Ok(());
    }
    let parsed = parsed_subscription_filters(filters_value)
        .map_err(|err| format!("module {module_id} subscription filters invalid: {err}"))?;
    for ruleset in parsed.event.iter().chain(parsed.action.iter()) {
        validate_ruleset(ruleset, module_id)?;
    }
    Ok(())
}

pub(super) fn subscription_match(pattern: &str, value: &str) -> bool {
    if pattern == "*" {
        return true;
    }
    if pattern.ends_with('*') {
        let prefix = &pattern[..pattern.len().saturating_sub(1)];
        return value.starts_with(prefix);
    }
    pattern == value
}

pub(super) fn subscription_filters_match(
    filters: &Option<JsonValue>,
    kind: FilterKind,
    value: &JsonValue,
) -> bool {
    let Some(filters_value) = filters else {
        return true;
    };
    if filters_value.is_null() {
        return true;
    }
    let parsed = match parsed_subscription_filters(filters_value) {
        Ok(parsed) => parsed,
        Err(_) => return false,
    };
    let rules = match kind {
        FilterKind::Event => parsed.event.as_ref(),
        FilterKind::Action => parsed.action.as_ref(),
    };
    let Some(rules) = rules else {
        return true;
    };
    ruleset_matches(rules, value)
}

pub(super) fn prepared_subscription_filters_match(
    filters: Option<&PreparedSubscriptionFilters>,
    kind: FilterKind,
    value: &JsonValue,
) -> bool {
    let Some(filters) = filters else {
        return true;
    };
    let rules = match kind {
        FilterKind::Event => filters.event.as_ref(),
        FilterKind::Action => filters.action.as_ref(),
    };
    let Some(rules) = rules else {
        return true;
    };
    prepared_ruleset_matches(rules, value)
}

pub(super) fn ruleset_matches(ruleset: &RuleSet, value: &JsonValue) -> bool {
    match ruleset {
        RuleSet::List(rules) => rules.iter().all(|rule| match_rule(rule, value)),
        RuleSet::Group(group) => {
            let all_ok = group.all.iter().all(|rule| match_rule(rule, value));
            if !all_ok {
                return false;
            }
            if group.any.is_empty() {
                return true;
            }
            group.any.iter().any(|rule| match_rule(rule, value))
        }
    }
}

fn prepared_ruleset_matches(ruleset: &PreparedRuleSet, value: &JsonValue) -> bool {
    match ruleset {
        PreparedRuleSet::List(rules) => rules.iter().all(|rule| prepared_match_rule(rule, value)),
        PreparedRuleSet::Group(group) => {
            let all_ok = group
                .all
                .iter()
                .all(|rule| prepared_match_rule(rule, value));
            if !all_ok {
                return false;
            }
            if group.any.is_empty() {
                return true;
            }
            group
                .any
                .iter()
                .any(|rule| prepared_match_rule(rule, value))
        }
    }
}

fn match_rule(rule: &MatchRule, value: &JsonValue) -> bool {
    let Some(current) = value.pointer(&rule.path) else {
        return false;
    };
    if let Some(expected) = &rule.eq {
        return current == expected;
    }
    if let Some(expected) = &rule.ne {
        return current != expected;
    }
    if let Some(pattern) = &rule.re {
        let Some(text) = current.as_str() else {
            return false;
        };
        return cached_regex(pattern)
            .map(|re| re.is_match(text))
            .unwrap_or(false);
    }
    if let Some(threshold) = rule.gt {
        return compare_number(current, |value| value > threshold);
    }
    if let Some(threshold) = rule.gte {
        return compare_number(current, |value| value >= threshold);
    }
    if let Some(threshold) = rule.lt {
        return compare_number(current, |value| value < threshold);
    }
    if let Some(threshold) = rule.lte {
        return compare_number(current, |value| value <= threshold);
    }
    false
}

fn prepared_match_rule(rule: &PreparedMatchRule, value: &JsonValue) -> bool {
    let Some(current) = value.pointer(&rule.path) else {
        return false;
    };
    match &rule.operator {
        PreparedMatchOperator::Eq(expected) => current == expected,
        PreparedMatchOperator::Ne(expected) => current != expected,
        PreparedMatchOperator::Gt(threshold) => compare_number(current, |value| value > *threshold),
        PreparedMatchOperator::Gte(threshold) => {
            compare_number(current, |value| value >= *threshold)
        }
        PreparedMatchOperator::Lt(threshold) => compare_number(current, |value| value < *threshold),
        PreparedMatchOperator::Lte(threshold) => {
            compare_number(current, |value| value <= *threshold)
        }
        PreparedMatchOperator::Re(regex) => current
            .as_str()
            .map(|text| regex.is_match(text))
            .unwrap_or(false),
    }
}

pub(super) fn compare_number<F>(value: &JsonValue, predicate: F) -> bool
where
    F: Fn(f64) -> bool,
{
    value.as_f64().map(predicate).unwrap_or(false)
}

pub(super) fn validate_ruleset(ruleset: &RuleSet, module_id: &str) -> Result<(), String> {
    match ruleset {
        RuleSet::List(rules) => {
            for rule in rules {
                validate_rule(rule, module_id)?;
            }
        }
        RuleSet::Group(group) => {
            for rule in group.all.iter().chain(group.any.iter()) {
                validate_rule(rule, module_id)?;
            }
        }
    }
    Ok(())
}

pub(super) fn validate_rule(rule: &MatchRule, module_id: &str) -> Result<(), String> {
    if !rule.path.is_empty() && !rule.path.starts_with('/') {
        return Err(format!(
            "module {module_id} subscription filter path must start with '/': {}",
            rule.path
        ));
    }

    let mut operators = 0usize;
    operators += usize::from(rule.eq.is_some());
    operators += usize::from(rule.ne.is_some());
    operators += usize::from(rule.gt.is_some());
    operators += usize::from(rule.gte.is_some());
    operators += usize::from(rule.lt.is_some());
    operators += usize::from(rule.lte.is_some());
    operators += usize::from(rule.re.is_some());
    if operators != 1 {
        return Err(format!(
            "module {module_id} subscription filter must specify exactly one operator"
        ));
    }

    for candidate in [rule.gt, rule.gte, rule.lt, rule.lte] {
        if let Some(number) = candidate {
            if !number.is_finite() {
                return Err(format!(
                    "module {module_id} subscription filter numeric value must be finite"
                ));
            }
        }
    }

    if let Some(pattern) = &rule.re {
        if regex::Regex::new(pattern).is_err() {
            return Err(format!(
                "module {module_id} subscription filter regex invalid"
            ));
        }
    }

    Ok(())
}
