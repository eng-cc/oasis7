use oasis7_wasm_abi::{ModuleSubscription, ModuleSubscriptionStage};
use serde::Deserialize;
use serde_json::Value as JsonValue;
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex, MutexGuard, OnceLock};

const MAX_CACHE_ENTRIES: usize = 1024;

struct BoundedCache<V> {
    capacity: usize,
    entries: HashMap<String, Arc<V>>,
    insertion_order: VecDeque<String>,
}

impl<V> BoundedCache<V> {
    fn new(capacity: usize) -> Self {
        Self {
            capacity,
            entries: HashMap::new(),
            insertion_order: VecDeque::new(),
        }
    }

    fn get_cloned(&self, key: &str) -> Option<Arc<V>> {
        self.entries.get(key).cloned()
    }

    fn insert(&mut self, key: String, value: Arc<V>) {
        if self.capacity == 0 {
            self.entries.clear();
            self.insertion_order.clear();
            return;
        }
        if self.entries.contains_key(&key) {
            self.entries.insert(key, value);
            return;
        }
        while self.entries.len() >= self.capacity {
            if let Some(oldest_key) = self.insertion_order.pop_front() {
                self.entries.remove(&oldest_key);
            } else {
                break;
            }
        }
        self.insertion_order.push_back(key.clone());
        self.entries.insert(key, value);
    }
}

type ParsedFilterCache = Mutex<BoundedCache<SubscriptionFilters>>;
type RegexCache = Mutex<BoundedCache<regex::Regex>>;

fn lock_recover<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    match mutex.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

fn parsed_filter_cache() -> &'static ParsedFilterCache {
    static CACHE: OnceLock<ParsedFilterCache> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(BoundedCache::new(MAX_CACHE_ENTRIES)))
}

fn regex_cache() -> &'static RegexCache {
    static CACHE: OnceLock<RegexCache> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(BoundedCache::new(MAX_CACHE_ENTRIES)))
}

fn parsed_subscription_filters(
    filters_value: &JsonValue,
) -> Result<Arc<SubscriptionFilters>, serde_json::Error> {
    let key = filters_value.to_string();
    if let Some(cached) = lock_recover(parsed_filter_cache()).get_cloned(&key) {
        return Ok(cached);
    }
    let parsed: SubscriptionFilters = serde_json::from_value(filters_value.clone())?;
    let parsed = Arc::new(parsed);
    lock_recover(parsed_filter_cache()).insert(key, parsed.clone());
    Ok(parsed)
}

fn cached_regex(pattern: &str) -> Option<Arc<regex::Regex>> {
    if let Some(cached) = lock_recover(regex_cache()).get_cloned(pattern) {
        return Some(cached);
    }
    let compiled = Arc::new(regex::Regex::new(pattern).ok()?);
    lock_recover(regex_cache()).insert(pattern.to_string(), compiled.clone());
    Some(compiled)
}

pub fn module_subscribes_to_event(
    subscriptions: &[ModuleSubscription],
    event_kind: &str,
    event_value: &JsonValue,
) -> bool {
    subscriptions.iter().any(|subscription| {
        subscription.resolved_stage() == ModuleSubscriptionStage::PostEvent
            && subscription
                .event_kinds
                .iter()
                .any(|pattern| subscription_match(pattern, event_kind))
            && subscription_filters_match(&subscription.filters, FilterKind::Event, event_value)
    })
}

pub fn module_subscribes_to_action(
    subscriptions: &[ModuleSubscription],
    stage: ModuleSubscriptionStage,
    action_kind: &str,
    action_value: &JsonValue,
) -> bool {
    subscriptions.iter().any(|subscription| {
        subscription.resolved_stage() == stage
            && subscription
                .action_kinds
                .iter()
                .any(|pattern| subscription_match(pattern, action_kind))
            && subscription_filters_match(&subscription.filters, FilterKind::Action, action_value)
    })
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

fn subscription_match(pattern: &str, value: &str) -> bool {
    if pattern == "*" {
        return true;
    }
    if pattern.ends_with('*') {
        let prefix = &pattern[..pattern.len().saturating_sub(1)];
        return value.starts_with(prefix);
    }
    pattern == value
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct SubscriptionFilters {
    #[serde(default)]
    event: Option<RuleSet>,
    #[serde(default)]
    action: Option<RuleSet>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
enum RuleSet {
    List(Vec<MatchRule>),
    Group(RuleGroup),
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct RuleGroup {
    #[serde(default)]
    all: Vec<MatchRule>,
    #[serde(default)]
    any: Vec<MatchRule>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct MatchRule {
    path: String,
    #[serde(default)]
    eq: Option<JsonValue>,
    #[serde(default)]
    ne: Option<JsonValue>,
    #[serde(default)]
    gt: Option<f64>,
    #[serde(default)]
    gte: Option<f64>,
    #[serde(default)]
    lt: Option<f64>,
    #[serde(default)]
    lte: Option<f64>,
    #[serde(default)]
    re: Option<String>,
}

#[derive(Debug, Clone, Copy)]
enum FilterKind {
    Event,
    Action,
}

fn subscription_filters_match(
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

fn ruleset_matches(ruleset: &RuleSet, value: &JsonValue) -> bool {
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

fn compare_number<F>(value: &JsonValue, predicate: F) -> bool
where
    F: Fn(f64) -> bool,
{
    value.as_f64().map(predicate).unwrap_or(false)
}

fn validate_ruleset(ruleset: &RuleSet, module_id: &str) -> Result<(), String> {
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

fn validate_rule(rule: &MatchRule, module_id: &str) -> Result<(), String> {
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

    for number in [rule.gt, rule.gte, rule.lt, rule.lte].into_iter().flatten() {
        if !number.is_finite() {
            return Err(format!(
                "module {module_id} subscription filter numeric value must be finite"
            ));
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn event_subscription(patterns: &[&str], filters: Option<JsonValue>) -> ModuleSubscription {
        ModuleSubscription {
            event_kinds: patterns.iter().map(|value| (*value).to_string()).collect(),
            action_kinds: Vec::new(),
            stage: Some(ModuleSubscriptionStage::PostEvent),
            filters,
        }
    }

    fn action_subscription(
        stage: ModuleSubscriptionStage,
        patterns: &[&str],
        filters: Option<JsonValue>,
    ) -> ModuleSubscription {
        ModuleSubscription {
            event_kinds: Vec::new(),
            action_kinds: patterns.iter().map(|value| (*value).to_string()).collect(),
            stage: Some(stage),
            filters,
        }
    }

    #[test]
    fn event_subscription_matches_wildcard_prefix_and_exact_patterns() {
        let payload = json!({ "kind": "world.tick", "hp": 3 });

        let wildcard = [event_subscription(&["*"], None)];
        assert!(module_subscribes_to_event(
            &wildcard,
            "world.tick",
            &payload
        ));

        let prefix = [event_subscription(&["world.*"], None)];
        assert!(module_subscribes_to_event(&prefix, "world.tick", &payload));
        assert!(!module_subscribes_to_event(
            &prefix,
            "action.move",
            &payload
        ));

        let exact = [event_subscription(&["world.tick"], None)];
        assert!(module_subscribes_to_event(&exact, "world.tick", &payload));
        assert!(!module_subscribes_to_event(&exact, "world.idle", &payload));
    }

    #[test]
    fn event_subscription_respects_filters_and_rejects_invalid_filter_schema() {
        let payload = json!({ "status": "ok", "hp": 8 });

        let match_status = [event_subscription(
            &["world.tick"],
            Some(json!({
                "event": [
                    { "path": "/status", "eq": "ok" }
                ]
            })),
        )];
        assert!(module_subscribes_to_event(
            &match_status,
            "world.tick",
            &payload
        ));

        let mismatch_status = [event_subscription(
            &["world.tick"],
            Some(json!({
                "event": [
                    { "path": "/status", "eq": "bad" }
                ]
            })),
        )];
        assert!(!module_subscribes_to_event(
            &mismatch_status,
            "world.tick",
            &payload
        ));

        let invalid_schema = [event_subscription(
            &["world.tick"],
            Some(json!({
                "event": [
                    { "path": "/status", "eq": "ok", "unknown": 1 }
                ]
            })),
        )];
        assert!(!module_subscribes_to_event(
            &invalid_schema,
            "world.tick",
            &payload
        ));
    }

    #[test]
    fn action_subscription_checks_stage_and_filters() {
        let subscription = action_subscription(
            ModuleSubscriptionStage::PreAction,
            &["action.move.*"],
            Some(json!({
                "action": {
                    "all": [
                        { "path": "/cost", "gte": 2.0 }
                    ],
                    "any": [
                        { "path": "/kind", "re": "^move\\." },
                        { "path": "/kind", "eq": "jump" }
                    ]
                }
            })),
        );

        let matched = json!({ "cost": 3.0, "kind": "move.left" });
        assert!(module_subscribes_to_action(
            &[subscription.clone()],
            ModuleSubscriptionStage::PreAction,
            "action.move.step",
            &matched,
        ));

        assert!(!module_subscribes_to_action(
            &[subscription.clone()],
            ModuleSubscriptionStage::PostAction,
            "action.move.step",
            &matched,
        ));

        let low_cost = json!({ "cost": 1.0, "kind": "move.left" });
        assert!(!module_subscribes_to_action(
            &[subscription.clone()],
            ModuleSubscriptionStage::PreAction,
            "action.move.step",
            &low_cost,
        ));

        let kind_not_matched = json!({ "cost": 3.0, "kind": "scan" });
        assert!(!module_subscribes_to_action(
            &[subscription],
            ModuleSubscriptionStage::PreAction,
            "action.move.step",
            &kind_not_matched,
        ));
    }

    #[test]
    fn validate_subscription_stage_rejects_invalid_combinations() {
        let cases = vec![
            (
                ModuleSubscription {
                    event_kinds: vec!["world.tick".to_string()],
                    action_kinds: vec!["action.move".to_string()],
                    stage: None,
                    filters: None,
                },
                "cannot mix event_kinds and action_kinds",
            ),
            (
                ModuleSubscription {
                    event_kinds: Vec::new(),
                    action_kinds: Vec::new(),
                    stage: None,
                    filters: None,
                },
                "requires event_kinds or action_kinds",
            ),
            (
                ModuleSubscription {
                    event_kinds: vec!["world.tick".to_string()],
                    action_kinds: vec!["action.move".to_string()],
                    stage: Some(ModuleSubscriptionStage::PostEvent),
                    filters: None,
                },
                "post_event cannot include action_kinds",
            ),
            (
                ModuleSubscription {
                    event_kinds: vec!["world.tick".to_string()],
                    action_kinds: Vec::new(),
                    stage: Some(ModuleSubscriptionStage::PreAction),
                    filters: None,
                },
                "action stage cannot include event_kinds",
            ),
            (
                ModuleSubscription {
                    event_kinds: vec!["world.tick".to_string()],
                    action_kinds: Vec::new(),
                    stage: Some(ModuleSubscriptionStage::Tick),
                    filters: None,
                },
                "tick stage cannot include event_kinds or action_kinds",
            ),
        ];

        for (subscription, expected) in cases {
            let err = validate_subscription_stage(&subscription, "m.test").unwrap_err();
            assert!(
                err.contains(expected),
                "expected error containing `{expected}`, got `{err}`"
            );
        }

        let event_only = ModuleSubscription {
            event_kinds: vec!["world.tick".to_string()],
            action_kinds: Vec::new(),
            stage: None,
            filters: None,
        };
        validate_subscription_stage(&event_only, "m.test").unwrap();

        let action_only = ModuleSubscription {
            event_kinds: Vec::new(),
            action_kinds: vec!["action.move".to_string()],
            stage: None,
            filters: None,
        };
        validate_subscription_stage(&action_only, "m.test").unwrap();

        let tick = ModuleSubscription {
            event_kinds: Vec::new(),
            action_kinds: Vec::new(),
            stage: Some(ModuleSubscriptionStage::Tick),
            filters: None,
        };
        validate_subscription_stage(&tick, "m.test").unwrap();
    }

    #[test]
    fn validate_subscription_filters_enforces_schema_and_rules() {
        validate_subscription_filters(&None, "m.test").unwrap();
        validate_subscription_filters(&Some(JsonValue::Null), "m.test").unwrap();

        let valid = Some(json!({
            "event": [
                { "path": "/hp", "gt": 0.0 }
            ],
            "action": {
                "all": [
                    { "path": "/kind", "re": "^move\\." }
                ]
            }
        }));
        validate_subscription_filters(&valid, "m.test").unwrap();

        let invalid_path = Some(json!({
            "event": [
                { "path": "hp", "eq": 1 }
            ]
        }));
        let path_err = validate_subscription_filters(&invalid_path, "m.test").unwrap_err();
        assert!(path_err.contains("path must start with '/'"));

        let invalid_operator_count = Some(json!({
            "event": [
                { "path": "/hp", "eq": 1, "gt": 0.0 }
            ]
        }));
        let operator_err =
            validate_subscription_filters(&invalid_operator_count, "m.test").unwrap_err();
        assert!(operator_err.contains("exactly one operator"));

        let invalid_regex = Some(json!({
            "action": [
                { "path": "/kind", "re": "[" }
            ]
        }));
        let regex_err = validate_subscription_filters(&invalid_regex, "m.test").unwrap_err();
        assert!(regex_err.contains("regex invalid"));

        let invalid_schema = Some(json!({
            "event": [
                { "path": "/kind", "eq": "ok", "unexpected": true }
            ]
        }));
        let schema_err = validate_subscription_filters(&invalid_schema, "m.test").unwrap_err();
        assert!(schema_err.contains("subscription filters invalid"));
        assert!(schema_err.starts_with("module m.test subscription filters invalid:"));
    }

    #[test]
    fn bounded_cache_evicts_oldest_entries() {
        let mut cache = BoundedCache::new(2);
        cache.insert("a".to_string(), Arc::new(1u32));
        cache.insert("b".to_string(), Arc::new(2u32));
        cache.insert("c".to_string(), Arc::new(3u32));

        assert!(cache.get_cloned("a").is_none());
        assert_eq!(cache.get_cloned("b").as_deref(), Some(&2u32));
        assert_eq!(cache.get_cloned("c").as_deref(), Some(&3u32));
    }

    #[test]
    #[ignore = "local perf probe"]
    fn perf_probe_subscription_filter_parse_overhead() {
        use std::time::Instant;

        let no_regex_filter_json = json!({
            "event": {
                "all": [
                    { "path": "/actor/kind", "eq": "player" },
                    { "path": "/stats/hp", "gt": 0.0 },
                    { "path": "/region/id", "eq": "region-alpha" }
                ],
                "any": [
                    { "path": "/event_kind", "eq": "world.tick" },
                    { "path": "/event_kind", "eq": "world.effect" },
                    { "path": "/event_kind", "eq": "world.spawn" }
                ]
            }
        });
        let no_regex_subscription = ModuleSubscription {
            event_kinds: vec!["world.*".to_string()],
            action_kinds: Vec::new(),
            stage: Some(ModuleSubscriptionStage::PostEvent),
            filters: Some(no_regex_filter_json.clone()),
        };
        let subscriptions = vec![no_regex_subscription];
        let event_value = json!({
            "actor": { "kind": "player", "id": "p-1" },
            "stats": { "hp": 7.0, "energy": 3.0 },
            "region": { "id": "region-alpha" },
            "event_kind": "world.tick"
        });

        assert!(module_subscribes_to_event(
            &subscriptions,
            "world.tick",
            &event_value
        ));

        let parsed: SubscriptionFilters =
            serde_json::from_value(no_regex_filter_json).expect("parse filters once");
        let parsed_rules = parsed.event.as_ref().expect("event filters");
        assert!(ruleset_matches(parsed_rules, &event_value));

        let iterations = 200_000u32;
        let started = Instant::now();
        for _ in 0..iterations {
            assert!(module_subscribes_to_event(
                &subscriptions,
                "world.tick",
                &event_value
            ));
        }
        let parse_each_time_elapsed = started.elapsed();

        let started = Instant::now();
        for _ in 0..iterations {
            assert!(ruleset_matches(parsed_rules, &event_value));
        }
        let parsed_once_elapsed = started.elapsed();

        let regex_filter_json = json!({
            "event": {
                "all": [
                    { "path": "/actor/kind", "eq": "player" },
                    { "path": "/stats/hp", "gt": 0.0 },
                    { "path": "/region/id", "re": "^region-" }
                ],
                "any": [
                    { "path": "/event_kind", "eq": "world.tick" },
                    { "path": "/event_kind", "eq": "world.effect" },
                    { "path": "/event_kind", "eq": "world.spawn" }
                ]
            }
        });
        let regex_subscription = vec![ModuleSubscription {
            event_kinds: vec!["world.*".to_string()],
            action_kinds: Vec::new(),
            stage: Some(ModuleSubscriptionStage::PostEvent),
            filters: Some(regex_filter_json.clone()),
        }];
        let parsed_regex: SubscriptionFilters =
            serde_json::from_value(regex_filter_json).expect("parse regex filters once");
        let parsed_regex_rules = parsed_regex.event.as_ref().expect("regex event filters");
        assert!(module_subscribes_to_event(
            &regex_subscription,
            "world.tick",
            &event_value
        ));
        assert!(ruleset_matches(parsed_regex_rules, &event_value));

        let started = Instant::now();
        for _ in 0..iterations {
            assert!(module_subscribes_to_event(
                &regex_subscription,
                "world.tick",
                &event_value
            ));
        }
        let regex_parse_each_time_elapsed = started.elapsed();

        let started = Instant::now();
        for _ in 0..iterations {
            assert!(ruleset_matches(parsed_regex_rules, &event_value));
        }
        let regex_parsed_once_elapsed = started.elapsed();

        eprintln!(
            "perf_probe_subscription_filter_parse_overhead: iterations={iterations} no_regex_parse_each_time_ms={:.3} no_regex_parsed_once_ms={:.3} no_regex_ratio={:.2}x regex_parse_each_time_ms={:.3} regex_parsed_once_ms={:.3} regex_ratio={:.2}x",
            parse_each_time_elapsed.as_secs_f64() * 1_000.0,
            parsed_once_elapsed.as_secs_f64() * 1_000.0,
            if parsed_once_elapsed.as_nanos() == 0 {
                0.0
            } else {
                parse_each_time_elapsed.as_secs_f64() / parsed_once_elapsed.as_secs_f64()
            },
            regex_parse_each_time_elapsed.as_secs_f64() * 1_000.0,
            regex_parsed_once_elapsed.as_secs_f64() * 1_000.0,
            if regex_parsed_once_elapsed.as_nanos() == 0 {
                0.0
            } else {
                regex_parse_each_time_elapsed.as_secs_f64() / regex_parsed_once_elapsed.as_secs_f64()
            },
        );
    }
}
