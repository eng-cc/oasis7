// Action mempool aggregation and deduplication.

use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};

use oasis7_proto::distributed::{ActionBatch, ActionEnvelope};
use serde::Serialize;

use super::error::WorldError;
use super::util::blake3_hex;

#[derive(Debug, Clone)]
pub struct ActionMempoolConfig {
    pub max_actions: usize,
    pub max_per_actor: usize,
}

impl Default for ActionMempoolConfig {
    fn default() -> Self {
        Self {
            max_actions: 10_000,
            max_per_actor: 256,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActionBatchRules {
    pub max_actions: usize,
    pub max_payload_bytes: usize,
}

impl Default for ActionBatchRules {
    fn default() -> Self {
        Self {
            max_actions: 512,
            max_payload_bytes: 512 * 1024,
        }
    }
}

#[derive(Debug, Default)]
pub struct ActionMempool {
    config: ActionMempoolConfig,
    actions: HashMap<String, ActionEnvelope>,
    per_actor: HashMap<String, Vec<String>>,
    idempotency_index: HashMap<String, String>,
    arrival: VecDeque<String>,
}

impl ActionMempool {
    pub fn new(config: ActionMempoolConfig) -> Self {
        Self {
            config,
            actions: HashMap::new(),
            per_actor: HashMap::new(),
            idempotency_index: HashMap::new(),
            arrival: VecDeque::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.actions.len()
    }

    pub fn is_empty(&self) -> bool {
        self.actions.is_empty()
    }

    pub fn add_action(&mut self, action: ActionEnvelope) -> bool {
        if self.actions.contains_key(&action.action_id) {
            return false;
        }
        if let Some(idempotency_lookup_key) =
            actor_idempotency_lookup_key(action.actor_id.as_str(), action.idempotency_key.as_str())
        {
            if self.idempotency_index.contains_key(&idempotency_lookup_key) {
                return false;
            }
        }

        let actor_count = self
            .per_actor
            .get(&action.actor_id)
            .map(|items| items.len())
            .unwrap_or(0);
        if actor_count >= self.config.max_per_actor {
            return false;
        }

        self.evict_if_needed();

        let actor_actions = self.per_actor.entry(action.actor_id.clone()).or_default();
        actor_actions.push(action.action_id.clone());
        self.arrival.push_back(action.action_id.clone());
        if let Some(idempotency_lookup_key) =
            actor_idempotency_lookup_key(action.actor_id.as_str(), action.idempotency_key.as_str())
        {
            self.idempotency_index
                .insert(idempotency_lookup_key, action.action_id.clone());
        }
        self.actions.insert(action.action_id.clone(), action);
        true
    }

    pub fn remove_action(&mut self, action_id: &str) -> Option<ActionEnvelope> {
        let action = self.remove_action_indexes(action_id)?;
        self.arrival.retain(|id| id != action_id);
        Some(action)
    }

    fn remove_action_indexes(&mut self, action_id: &str) -> Option<ActionEnvelope> {
        let action = self.actions.remove(action_id)?;
        if let Some(actor_actions) = self.per_actor.get_mut(&action.actor_id) {
            actor_actions.retain(|id| id != action_id);
            if actor_actions.is_empty() {
                self.per_actor.remove(&action.actor_id);
            }
        }
        if let Some(idempotency_lookup_key) =
            actor_idempotency_lookup_key(action.actor_id.as_str(), action.idempotency_key.as_str())
        {
            if self
                .idempotency_index
                .get(&idempotency_lookup_key)
                .is_some_and(|existing| existing == action_id)
            {
                self.idempotency_index.remove(&idempotency_lookup_key);
            }
        }
        Some(action)
    }

    fn remove_actions<I>(&mut self, action_ids: I) -> Vec<ActionEnvelope>
    where
        I: IntoIterator<Item = String>,
    {
        let mut removed_actions = Vec::new();
        let mut removed_action_ids = HashSet::new();
        for action_id in action_ids {
            if let Some(action) = self.remove_action_indexes(action_id.as_str()) {
                removed_action_ids.insert(action_id);
                removed_actions.push(action);
            }
        }
        if !removed_action_ids.is_empty() {
            self.arrival
                .retain(|action_id| !removed_action_ids.contains(action_id));
        }
        removed_actions
    }

    fn remove_action_groups(
        &mut self,
        selected_action_groups: Vec<Vec<String>>,
        extra_action_ids: Vec<String>,
    ) -> Vec<Vec<ActionEnvelope>> {
        let mut removed_action_ids = HashSet::new();
        for action_id in extra_action_ids {
            if self.remove_action_indexes(action_id.as_str()).is_some() {
                removed_action_ids.insert(action_id);
            }
        }

        let mut removed_groups = Vec::with_capacity(selected_action_groups.len());
        for selected_action_ids in selected_action_groups {
            let mut removed_group = Vec::with_capacity(selected_action_ids.len());
            for action_id in selected_action_ids {
                if let Some(action) = self.remove_action_indexes(action_id.as_str()) {
                    removed_action_ids.insert(action_id);
                    removed_group.push(action);
                }
            }
            removed_groups.push(removed_group);
        }

        if !removed_action_ids.is_empty() {
            self.arrival
                .retain(|action_id| !removed_action_ids.contains(action_id));
        }
        removed_groups
    }

    pub fn take_batch(
        &mut self,
        world_id: &str,
        proposer_id: &str,
        max_actions: usize,
        timestamp_ms: i64,
    ) -> Result<Option<ActionBatch>, WorldError> {
        self.take_batch_with_rules(
            world_id,
            proposer_id,
            ActionBatchRules {
                max_actions,
                max_payload_bytes: usize::MAX,
            },
            timestamp_ms,
        )
    }

    pub fn take_batch_with_rules(
        &mut self,
        world_id: &str,
        proposer_id: &str,
        rules: ActionBatchRules,
        timestamp_ms: i64,
    ) -> Result<Option<ActionBatch>, WorldError> {
        if self.actions.is_empty() {
            return Ok(None);
        }
        let (selected_action_ids, oversized_action_ids) = {
            let mut candidates: Vec<&ActionEnvelope> = self.actions.values().collect();
            candidates.sort_by(|left, right| action_batch_order(left, right));

            let mut selected_action_ids = Vec::new();
            let mut oversized_action_ids = Vec::new();
            let mut total_bytes = 0usize;

            for action in candidates {
                if selected_action_ids.len() >= rules.max_actions {
                    break;
                }
                let size_bytes = action_size_bytes(action)?;
                if size_bytes > rules.max_payload_bytes {
                    oversized_action_ids.push(action.action_id.clone());
                    continue;
                }
                let next_total_bytes =
                    checked_usize_add(total_bytes, size_bytes, "mempool batch payload bytes")?;
                if next_total_bytes > rules.max_payload_bytes {
                    break;
                }
                total_bytes = next_total_bytes;
                selected_action_ids.push(action.action_id.clone());
            }
            (selected_action_ids, oversized_action_ids)
        };

        self.remove_actions(oversized_action_ids);

        if selected_action_ids.is_empty() {
            return Ok(None);
        }

        let actions = self.remove_actions(selected_action_ids);
        if actions.is_empty() {
            return Ok(None);
        }

        let batch_id = batch_id_for_actions(&actions)?;
        Ok(Some(ActionBatch {
            world_id: world_id.to_string(),
            batch_id,
            actions,
            proposer_id: proposer_id.to_string(),
            timestamp_ms,
            signature: String::new(),
        }))
    }

    pub fn take_zone_batches_with_rules(
        &mut self,
        world_id: &str,
        proposer_id: &str,
        rules: ActionBatchRules,
        timestamp_ms: i64,
    ) -> Result<Vec<ActionBatch>, WorldError> {
        if self.actions.is_empty() {
            return Ok(Vec::new());
        }
        let (selected_batch_ids, oversized_action_ids) = {
            let mut zone_candidates = BTreeMap::<String, Vec<&ActionEnvelope>>::new();
            for candidate in self.actions.values() {
                zone_candidates
                    .entry(normalized_zone_id(candidate.zone_id.as_str()))
                    .or_default()
                    .push(candidate);
            }

            let mut selected_batch_ids = Vec::new();
            let mut oversized_action_ids = Vec::new();
            for (_zone, mut zone_actions) in zone_candidates {
                zone_actions.sort_by(|left, right| action_batch_order(left, right));
                let mut selected_ids = Vec::new();
                let mut total_bytes = 0usize;
                for action in zone_actions {
                    if selected_ids.len() >= rules.max_actions {
                        break;
                    }
                    let size_bytes = action_size_bytes(action)?;
                    if size_bytes > rules.max_payload_bytes {
                        oversized_action_ids.push(action.action_id.clone());
                        continue;
                    }
                    let next_total_bytes =
                        checked_usize_add(total_bytes, size_bytes, "mempool zone batch bytes")?;
                    if next_total_bytes > rules.max_payload_bytes {
                        break;
                    }
                    total_bytes = next_total_bytes;
                    selected_ids.push(action.action_id.clone());
                }
                if !selected_ids.is_empty() {
                    selected_batch_ids.push(selected_ids);
                }
            }
            (selected_batch_ids, oversized_action_ids)
        };

        let mut batches = Vec::new();
        for selected in self.remove_action_groups(selected_batch_ids, oversized_action_ids) {
            if selected.is_empty() {
                continue;
            }
            let batch_id = batch_id_for_actions(&selected)?;
            batches.push(ActionBatch {
                world_id: world_id.to_string(),
                batch_id,
                actions: selected,
                proposer_id: proposer_id.to_string(),
                timestamp_ms,
                signature: String::new(),
            });
        }
        Ok(batches)
    }

    fn evict_if_needed(&mut self) {
        while self.actions.len() >= self.config.max_actions {
            let Some(evicted_id) = self.arrival.pop_front() else {
                break;
            };
            self.remove_action(&evicted_id);
        }
    }
}

fn action_batch_order(left: &ActionEnvelope, right: &ActionEnvelope) -> std::cmp::Ordering {
    left.timestamp_ms
        .cmp(&right.timestamp_ms)
        .then_with(|| left.action_id.cmp(&right.action_id))
}

fn batch_id_for_actions(actions: &[ActionEnvelope]) -> Result<String, WorldError> {
    #[derive(Debug, Serialize)]
    struct BatchHashInput<'a> {
        action_id: &'a str,
        intent_batch_hash: &'a str,
        idempotency_key: &'a str,
    }

    let mut entries: Vec<BatchHashInput<'_>> = actions
        .iter()
        .map(|action| BatchHashInput {
            action_id: action.action_id.as_str(),
            intent_batch_hash: action.intent_batch_hash.as_str(),
            idempotency_key: action.idempotency_key.as_str(),
        })
        .collect();
    entries.sort_by(|left, right| left.action_id.cmp(right.action_id));
    let bytes = to_canonical_cbor(&entries)?;
    Ok(blake3_hex(&bytes))
}

fn action_size_bytes(action: &ActionEnvelope) -> Result<usize, WorldError> {
    let bytes = to_canonical_cbor(action)?;
    Ok(bytes.len())
}

fn checked_usize_add(lhs: usize, rhs: usize, context: &str) -> Result<usize, WorldError> {
    lhs.checked_add(rhs)
        .ok_or_else(|| WorldError::DistributedValidationFailed {
            reason: format!("{context} overflow: lhs={lhs}, rhs={rhs}"),
        })
}

fn to_canonical_cbor<T: Serialize>(value: &T) -> Result<Vec<u8>, WorldError> {
    let mut buf = Vec::with_capacity(256);
    let canonical_value = serde_cbor::value::to_value(value)?;
    let mut serializer = serde_cbor::ser::Serializer::new(&mut buf);
    serializer.self_describe()?;
    canonical_value.serialize(&mut serializer)?;
    Ok(buf)
}

fn actor_idempotency_lookup_key(actor_id: &str, idempotency_key: &str) -> Option<String> {
    if idempotency_key.trim().is_empty() {
        return None;
    }
    Some(format!("{actor_id}:{idempotency_key}"))
}

fn normalized_zone_id(zone_id: &str) -> String {
    if zone_id.trim().is_empty() {
        "global".to_string()
    } else {
        zone_id.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn action(id: &str, actor: &str, ts: i64) -> ActionEnvelope {
        action_with_meta(id, actor, ts, "", "")
    }

    fn action_with_meta(
        id: &str,
        actor: &str,
        ts: i64,
        idempotency_key: &str,
        intent_batch_hash: &str,
    ) -> ActionEnvelope {
        ActionEnvelope {
            world_id: "w1".to_string(),
            action_id: id.to_string(),
            actor_id: actor.to_string(),
            action_kind: "test".to_string(),
            payload_cbor: Vec::new(),
            payload_hash: "hash".to_string(),
            nonce: 1,
            timestamp_ms: ts,
            intent_batch_hash: intent_batch_hash.to_string(),
            idempotency_key: idempotency_key.to_string(),
            zone_id: String::new(),
            signature: String::new(),
        }
    }

    #[test]
    fn mempool_dedups_by_action_id() {
        let mut pool = ActionMempool::new(ActionMempoolConfig::default());
        assert!(pool.add_action(action("a1", "actor1", 1)));
        assert!(!pool.add_action(action("a1", "actor1", 2)));
        assert_eq!(pool.len(), 1);
    }

    #[test]
    fn mempool_respects_actor_limit() {
        let mut pool = ActionMempool::new(ActionMempoolConfig {
            max_actions: 10,
            max_per_actor: 1,
        });
        assert!(pool.add_action(action("a1", "actor1", 1)));
        assert!(!pool.add_action(action("a2", "actor1", 2)));
        assert_eq!(pool.len(), 1);
    }

    #[test]
    fn mempool_dedups_by_actor_idempotency_key() {
        let mut pool = ActionMempool::new(ActionMempoolConfig::default());
        assert!(pool.add_action(action_with_meta(
            "a1",
            "actor1",
            1,
            "idem-001",
            "intent-hash-1"
        )));
        assert!(!pool.add_action(action_with_meta(
            "a2",
            "actor1",
            2,
            "idem-001",
            "intent-hash-2"
        )));
        assert!(pool.add_action(action_with_meta(
            "a3",
            "actor2",
            3,
            "idem-001",
            "intent-hash-3"
        )));
        assert_eq!(pool.len(), 2);
    }

    #[test]
    fn mempool_evicts_oldest_when_full() {
        let mut pool = ActionMempool::new(ActionMempoolConfig {
            max_actions: 2,
            max_per_actor: 10,
        });
        assert!(pool.add_action(action("a1", "actor1", 1)));
        assert!(pool.add_action(action("a2", "actor2", 2)));
        assert!(pool.add_action(action("a3", "actor3", 3)));
        assert_eq!(pool.len(), 2);
        assert!(pool.actions.contains_key("a2"));
        assert!(pool.actions.contains_key("a3"));
        assert!(!pool.actions.contains_key("a1"));
    }

    #[test]
    fn take_batch_orders_by_timestamp_then_id() {
        let mut pool = ActionMempool::new(ActionMempoolConfig::default());
        pool.add_action(action("a2", "actor1", 2));
        pool.add_action(action("a1", "actor2", 1));
        pool.add_action(action("a3", "actor3", 2));

        let batch = pool
            .take_batch("w1", "seq", 2, 10)
            .expect("batch result")
            .expect("batch");

        assert_eq!(batch.actions.len(), 2);
        assert_eq!(batch.actions[0].action_id, "a1");
        assert_eq!(batch.actions[1].action_id, "a2");
        assert_eq!(pool.len(), 1);
    }

    #[test]
    fn take_batch_respects_payload_limit() {
        let mut pool = ActionMempool::new(ActionMempoolConfig::default());
        let mut large = action("a1", "actor1", 1);
        large.payload_cbor = vec![0u8; 2048];
        let small = action("a2", "actor2", 2);

        pool.add_action(large);
        pool.add_action(small);

        let batch = pool
            .take_batch_with_rules(
                "w1",
                "seq",
                ActionBatchRules {
                    max_actions: 10,
                    max_payload_bytes: 512,
                },
                10,
            )
            .expect("batch result")
            .expect("batch");

        assert_eq!(batch.actions.len(), 1);
        assert_eq!(batch.actions[0].action_id, "a2");
    }

    #[test]
    fn take_batch_removes_selected_and_oversized_indexes_once() {
        let mut pool = ActionMempool::new(ActionMempoolConfig::default());
        let mut oversized = action_with_meta("a1", "actor1", 1, "idem-a1", "intent-a1");
        oversized.payload_cbor = vec![0u8; 2048];
        assert!(pool.add_action(oversized));
        assert!(pool.add_action(action_with_meta("a2", "actor1", 2, "idem-a2", "intent-a2")));
        assert!(pool.add_action(action_with_meta("a3", "actor1", 3, "idem-a3", "intent-a3")));

        let batch = pool
            .take_batch_with_rules(
                "w1",
                "seq",
                ActionBatchRules {
                    max_actions: 1,
                    max_payload_bytes: 512,
                },
                10,
            )
            .expect("batch result")
            .expect("batch");

        assert_eq!(batch.actions.len(), 1);
        assert_eq!(batch.actions[0].action_id, "a2");
        assert_eq!(
            pool.arrival.iter().cloned().collect::<Vec<_>>(),
            vec!["a3".to_string()]
        );
        assert_eq!(
            pool.per_actor.get("actor1").cloned(),
            Some(vec!["a3".to_string()])
        );
        assert!(!pool.idempotency_index.contains_key("actor1:idem-a1"));
        assert!(!pool.idempotency_index.contains_key("actor1:idem-a2"));
        assert_eq!(
            pool.idempotency_index.get("actor1:idem-a3"),
            Some(&"a3".to_string())
        );
        assert!(pool.add_action(action_with_meta("a4", "actor1", 4, "idem-a1", "intent-a4")));
    }

    #[test]
    fn checked_usize_add_rejects_overflow() {
        let err =
            checked_usize_add(usize::MAX, 1, "mempool checked add").expect_err("must overflow");
        match err {
            WorldError::DistributedValidationFailed { reason } => {
                assert!(reason.contains("mempool checked add overflow"), "{reason}");
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[test]
    fn batch_id_is_stable_with_intent_hash_and_idempotency_keys() {
        let mut pool_a = ActionMempool::new(ActionMempoolConfig::default());
        let mut pool_b = ActionMempool::new(ActionMempoolConfig::default());

        let first = action_with_meta("a1", "actor1", 2, "idem-1", "intent-1");
        let second = action_with_meta("a2", "actor2", 1, "idem-2", "intent-2");

        assert!(pool_a.add_action(first.clone()));
        assert!(pool_a.add_action(second.clone()));

        assert!(pool_b.add_action(second));
        assert!(pool_b.add_action(first));

        let batch_a = pool_a
            .take_batch_with_rules("w1", "seq-1", ActionBatchRules::default(), 1_000)
            .expect("batch")
            .expect("some batch");
        let batch_b = pool_b
            .take_batch_with_rules("w1", "seq-1", ActionBatchRules::default(), 1_000)
            .expect("batch")
            .expect("some batch");
        assert_eq!(batch_a.batch_id, batch_b.batch_id);
    }

    #[test]
    fn take_zone_batches_segments_actions_by_zone() {
        let mut pool = ActionMempool::new(ActionMempoolConfig::default());
        let mut zone_a_1 = action("a1", "actor1", 1);
        zone_a_1.zone_id = "zone-a".to_string();
        let mut zone_a_2 = action("a2", "actor2", 2);
        zone_a_2.zone_id = "zone-a".to_string();
        let mut zone_b_1 = action("a3", "actor3", 1);
        zone_b_1.zone_id = "zone-b".to_string();
        assert!(pool.add_action(zone_a_1));
        assert!(pool.add_action(zone_a_2));
        assert!(pool.add_action(zone_b_1));

        let batches = pool
            .take_zone_batches_with_rules("w1", "seq-1", ActionBatchRules::default(), 1_000)
            .expect("zone batches");
        assert_eq!(batches.len(), 2);
        for batch in &batches {
            let mut zones: Vec<String> = batch
                .actions
                .iter()
                .map(|action| normalized_zone_id(action.zone_id.as_str()))
                .collect();
            zones.sort();
            zones.dedup();
            assert_eq!(zones.len(), 1);
        }
        assert!(pool.is_empty());
    }

    #[test]
    fn take_zone_batches_removes_selected_and_oversized_indexes_once() {
        let mut pool = ActionMempool::new(ActionMempoolConfig::default());
        let mut oversized = action_with_meta("a1", "actor1", 1, "idem-a1", "intent-a1");
        oversized.payload_cbor = vec![0u8; 2048];
        oversized.zone_id = "zone-a".to_string();
        let mut zone_a_selected = action_with_meta("a2", "actor1", 2, "idem-a2", "intent-a2");
        zone_a_selected.zone_id = "zone-a".to_string();
        let mut zone_b_selected = action_with_meta("a3", "actor1", 3, "idem-a3", "intent-a3");
        zone_b_selected.zone_id = "zone-b".to_string();
        let mut zone_a_remaining = action_with_meta("a4", "actor1", 4, "idem-a4", "intent-a4");
        zone_a_remaining.zone_id = "zone-a".to_string();

        assert!(pool.add_action(oversized));
        assert!(pool.add_action(zone_a_selected));
        assert!(pool.add_action(zone_b_selected));
        assert!(pool.add_action(zone_a_remaining));

        let batches = pool
            .take_zone_batches_with_rules(
                "w1",
                "seq",
                ActionBatchRules {
                    max_actions: 1,
                    max_payload_bytes: 512,
                },
                10,
            )
            .expect("zone batches");

        assert_eq!(batches.len(), 2);
        assert_eq!(batches[0].actions[0].action_id, "a2");
        assert_eq!(batches[1].actions[0].action_id, "a3");
        assert_eq!(
            pool.arrival.iter().cloned().collect::<Vec<_>>(),
            vec!["a4".to_string()]
        );
        assert_eq!(
            pool.per_actor.get("actor1").cloned(),
            Some(vec!["a4".to_string()])
        );
        assert!(!pool.idempotency_index.contains_key("actor1:idem-a1"));
        assert!(!pool.idempotency_index.contains_key("actor1:idem-a2"));
        assert!(!pool.idempotency_index.contains_key("actor1:idem-a3"));
        assert_eq!(
            pool.idempotency_index.get("actor1:idem-a4"),
            Some(&"a4".to_string())
        );
        assert!(pool.add_action(action_with_meta("a5", "actor1", 5, "idem-a1", "intent-a5")));
    }
}
