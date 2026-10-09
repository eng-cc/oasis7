use oasis7_wasm_abi::{ModuleSubscription, ModuleSubscriptionStage};
use oasis7_wasm_router::*;
use serde_json::{Value, json};

fn subscription(kind: &str, rule: Value) -> ModuleSubscription {
    ModuleSubscription {
        event_kinds: if kind == "event" {
            vec!["*".into()]
        } else {
            vec![]
        },
        action_kinds: if kind == "action" {
            vec!["*".into()]
        } else {
            vec![]
        },
        stage: Some(if kind == "event" {
            ModuleSubscriptionStage::PostEvent
        } else {
            ModuleSubscriptionStage::PreAction
        }),
        filters: Some(json!({kind: [rule]})),
    }
}

fn matches(sub: &ModuleSubscription, payload: &Value) -> (bool, bool) {
    let prepared = prepare_subscriptions(std::slice::from_ref(sub), "test").unwrap();
    if sub.stage == Some(ModuleSubscriptionStage::PostEvent) {
        (
            module_subscribes_to_event(std::slice::from_ref(sub), "event", payload),
            prepared_module_subscribes_to_event(&prepared, "event", payload),
        )
    } else {
        (
            module_subscribes_to_action(
                std::slice::from_ref(sub),
                ModuleSubscriptionStage::PreAction,
                "action",
                payload,
            ),
            prepared_module_subscribes_to_action(
                &prepared,
                ModuleSubscriptionStage::PreAction,
                "action",
                payload,
            ),
        )
    }
}

#[test]
fn null_equality_and_inequality_distinguish_missing_paths() {
    for kind in ["event", "action"] {
        for (operator, expected) in [("eq", [true, false, false]), ("ne", [false, true, false])] {
            let sub = subscription(kind, json!({"path":"/x", operator:null}));
            validate_subscription_filters(&sub.filters, "test").unwrap();
            for (payload, expected) in [json!({"x":null}), json!({"x":1}), json!({})]
                .iter()
                .zip(expected)
            {
                assert_eq!(matches(&sub, payload), (expected, expected));
            }
        }
    }
}

#[test]
fn invalid_rules_are_rejected_by_validation_preparation_and_direct_routing() {
    for kind in ["event", "action"] {
        for rule in [
            json!({"path":"/x", "eq":null, "ne":1}),
            json!({"path":"/x", "eq":null, "ne":null}),
            json!({"path":"/x", "eq":1, "gt":null}),
            json!({"path":"/x", "eq":1, "re":null}),
            json!({"path":"/x~2", "eq":1}),
            json!({"path":"/x~", "eq":1}),
            json!({"path":"/x~01~bad", "eq":1}),
        ] {
            let sub = subscription(kind, rule.clone());
            assert!(
                validate_subscription_filters(&sub.filters, "test").is_err(),
                "{rule}"
            );
            assert!(
                prepare_subscriptions(std::slice::from_ref(&sub), "test").is_err(),
                "{rule}"
            );
            let payload = json!({"x":1, "x~2":1, "x~":1});
            assert!(!module_subscribes_to_event(
                std::slice::from_ref(&sub),
                "event",
                &payload
            ));
            assert!(!module_subscribes_to_action(
                std::slice::from_ref(&sub),
                ModuleSubscriptionStage::PreAction,
                "action",
                &payload
            ));
        }
    }
}

#[test]
fn valid_pointer_escapes_empty_tokens_and_root_match() {
    for kind in ["event", "action"] {
        let payload = json!({"a/b":{"~key":7}, "":9, "~1":10, "雪":11});
        for (path, expected) in [
            ("/a~1b/~0key", json!(7)),
            ("/", json!(9)),
            ("/~01", json!(10)),
            ("/雪", json!(11)),
            ("", payload.clone()),
        ] {
            let sub = subscription(kind, json!({"path":path, "eq":expected}));
            validate_subscription_filters(&sub.filters, "test").unwrap();
            assert_eq!(matches(&sub, &payload), (true, true), "{path}");
        }
    }
}

#[test]
fn invalid_opposite_kind_rules_reject_the_entire_filter() {
    for kind in ["event", "action"] {
        for include_selected in [false, true] {
            let mut sub = subscription(kind, json!({"path":"/x", "eq":1}));
            let other = if kind == "event" { "action" } else { "event" };
            let mut filters = json!({other: [{"path":"/x~2", "eq":1}]});
            if include_selected {
                filters[kind] = json!([{ "path":"/x", "eq":1 }]);
            }
            sub.filters = Some(filters);
            assert!(validate_subscription_filters(&sub.filters, "test").is_err());
            assert!(prepare_subscriptions(std::slice::from_ref(&sub), "test").is_err());
            let payload = json!({"x":1});
            assert!(!module_subscribes_to_event(
                std::slice::from_ref(&sub),
                "event",
                &payload
            ));
            assert!(!module_subscribes_to_action(
                std::slice::from_ref(&sub),
                ModuleSubscriptionStage::PreAction,
                "action",
                &payload
            ));
        }
    }
}
