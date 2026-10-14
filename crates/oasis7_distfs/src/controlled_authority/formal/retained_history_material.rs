//! Bounded Tick-prefix material validator. The caller supplies Snapshot
//! bytes already verified against the exact Frozen/candidate artifact roots.
//! Leaf lookup must use the verified delta overlay under the after root.
use serde::Deserialize;
use serde_cbor::Value;

const MAX_ARCHIVED_PER_TRANSITION: u64 = 64;

#[derive(Debug, Deserialize)]
pub(super) struct TickSnapshotMaterial {
    #[serde(default)]
    state: Option<TickSnapshotTime>,
    #[serde(default)]
    tick_consensus_archived_record_count: u64,
    #[serde(default)]
    tick_consensus_total_record_count: u64,
    #[serde(default)]
    tick_consensus_records: Vec<Value>,
}

#[derive(Debug, Deserialize)]
struct TickSnapshotTime {
    #[serde(default)]
    time: Option<u64>,
}
impl TickSnapshotMaterial {
    pub(super) fn decode(bytes: &[u8]) -> Result<Self, String> {
        let snapshot: Self = serde_cbor::from_slice(bytes).map_err(|e| e.to_string())?;
        let hot = u64::try_from(snapshot.tick_consensus_records.len())
            .map_err(|_| "Tick hot length overflow")?;
        let total = snapshot
            .tick_consensus_archived_record_count
            .checked_add(hot)
            .ok_or("Tick total count overflow")?;
        if total != snapshot.tick_consensus_total_record_count
            || (snapshot.tick_consensus_archived_record_count > 0 && hot == 0)
        {
            return Err("Tick prefix/total/hot count mismatch".into());
        }
        if let Some(first) = snapshot.tick_consensus_records.first() {
            if snapshot.tick_consensus_archived_record_count == 0
                && text(field(
                    field(field(first, "block")?, "header")?,
                    "parent_hash",
                )?)? != "genesis"
            {
                return Err("Tick genesis parent mismatch".into());
            }
            let time = snapshot
                .state
                .as_ref()
                .and_then(|state| state.time)
                .ok_or("Tick material missing hash-bound Snapshot time")?;
            let last = snapshot.tick_consensus_records.last().unwrap();
            if unsigned(field(field(field(last, "block")?, "header")?, "tick")?)? > time {
                return Err("Tick certificate is beyond hash-bound Snapshot time".into());
            }
        }
        let mut previous: Option<&Value> = None;
        for record in &snapshot.tick_consensus_records {
            validate_record(record)?;
            if let Some(prior) = previous {
                let tick = unsigned(field(field(field(record, "block")?, "header")?, "tick")?)?;
                let prior_tick =
                    unsigned(field(field(field(prior, "block")?, "header")?, "tick")?)?;
                if tick <= prior_tick {
                    return Err("Tick hot timestamps do not advance".into());
                }
                if field(field(field(record, "block")?, "header")?, "parent_hash")?
                    != field(field(prior, "certificate")?, "block_hash")?
                {
                    return Err("Tick hot parent chain mismatch".into());
                }
            }
            previous = Some(record);
        }
        // No fixed total or hot32 input cap: approved legacy Snapshot closures
        // may contain a longer tail, reduced in at most64-record transitions.
        for (offset, record) in snapshot.tick_consensus_records.iter().enumerate() {
            let height = snapshot
                .tick_consensus_archived_record_count
                .checked_add(offset as u64)
                .and_then(|n| n.checked_add(1))
                .ok_or("Tick height overflow")?;
            if unsigned(field(field(record, "certificate")?, "consensus_height")?)? != height {
                return Err("Tick hot certificate height differs from anchored prefix".into());
            }
        }
        Ok(snapshot)
    }

    /// Returns exact original prefix bytes. Before any receipt, the caller must
    /// match each against its immutable height leaf AND authenticated tick
    /// locator in the proposed retained root; comparison alone is insufficient.
    pub(super) fn archived_prefix(&self, after: &Self) -> Result<Vec<(u64, u64, Vec<u8>)>, String> {
        let added = after
            .tick_consensus_archived_record_count
            .checked_sub(self.tick_consensus_archived_record_count)
            .ok_or("Tick archived count rollback")?;
        if added != 0
            && (added > MAX_ARCHIVED_PER_TRANSITION
                || added as usize >= self.tick_consensus_records.len())
        {
            return Err("Tick archive prefix exceeds bounded original material".into());
        }
        let prefix = added as usize;
        let remaining = &self.tick_consensus_records[prefix..];
        if after.tick_consensus_records.len() < remaining.len()
            || after.tick_consensus_records[..remaining.len()] != *remaining
        {
            return Err("Tick candidate rewrites qualified original hot tail".into());
        }
        self.tick_consensus_records[..prefix]
            .iter()
            .enumerate()
            .map(|(offset, value)| {
                let height = self
                    .tick_consensus_archived_record_count
                    .checked_add(offset as u64)
                    .and_then(|n| n.checked_add(1))
                    .ok_or("Tick height overflow")?;
                let tick = unsigned(field(field(field(value, "block")?, "header")?, "tick")?)?;
                // serde_cbor::Value maps use canonical length/byte key ordering.
                // TC records contain integers/text/arrays/maps only; no floats.
                let bytes = serde_cbor::to_vec(value).map_err(|e| e.to_string())?;
                Ok((height, tick, bytes))
            })
            .collect()
    }
}
fn field<'a>(value: &'a Value, name: &str) -> Result<&'a Value, String> {
    let Value::Map(map) = value else {
        return Err("Tick material expected map".into());
    };
    map.get(&Value::Text(name.into()))
        .ok_or_else(|| format!("Tick material missing {name}"))
}
fn unsigned(value: &Value) -> Result<u64, String> {
    let Value::Integer(integer) = value else {
        return Err("Tick material expected integer".into());
    };
    u64::try_from(*integer).map_err(|_| "Tick material unsigned integer range".into())
}

use super::business_index::{
    BusinessIndexDeltaV1, BusinessIndexLeafKindV1, BusinessIndexNodeReader, BusinessIndexRootV1,
};
use std::collections::BTreeMap;
struct DeltaReader<'a, R> {
    delta: Option<&'a BusinessIndexDeltaV1>,
    prior: &'a R,
}
impl<R: BusinessIndexNodeReader> BusinessIndexNodeReader for DeltaReader<'_, R> {
    fn read_node(&self, digest: &str) -> Result<Option<Vec<u8>>, String> {
        if let Some(node) = self
            .delta
            .and_then(|delta| delta.node_writes.iter().find(|n| n.digest == digest))
        {
            if blake3::hash(&node.bytes).to_hex().as_str() != digest {
                return Err("Tick delta node checksum".into());
            }
            return Ok(Some(node.bytes.clone()));
        }
        self.prior.read_node(digest)
    }
}
pub(super) fn verify_tick_prefix_material(
    before_bytes: Option<&[u8]>,
    after_bytes: &[u8],
    new_root: Option<&BusinessIndexRootV1>,
    delta: Option<&BusinessIndexDeltaV1>,
    reader: &impl BusinessIndexNodeReader,
) -> Result<(), String> {
    let after = TickSnapshotMaterial::decode(after_bytes)?;
    let Some(before_bytes) = before_bytes else {
        if after.tick_consensus_archived_record_count != 0 {
            return Err("fresh initial cannot assert archived Tick prefix".into());
        }
        return Ok(());
    };
    let before = TickSnapshotMaterial::decode(before_bytes)?;
    let rows = before.archived_prefix(&after)?;
    let overlay = DeltaReader {
        delta,
        prior: reader,
    };
    let archive = |height: u64, tick: u64, record: &[u8]| -> Result<(), String> {
        let root = new_root.ok_or("Tick prefix requires authenticated retained root")?;
        let key = retained_key("world-tick-consensus-height", height)?;
        let leaf = root
            .get(&key, &overlay)?
            .ok_or("Tick height leaf missing")?;
        let body = Value::Map(BTreeMap::from([
            (Value::Text("height".into()), Value::Integer(height.into())),
            (Value::Text("tick".into()), Value::Integer(tick.into())),
            (
                Value::Text("record_cbor".into()),
                Value::Bytes(record.to_vec()),
            ),
            (
                Value::Text("record_digest".into()),
                Value::Text(blake3::hash(record).to_hex().to_string()),
            ),
        ]));
        let expected = serde_cbor::to_vec(&Value::Map(BTreeMap::from([(
            Value::Text("TickConsensusRecord".into()),
            body,
        )])))
        .map_err(|e| e.to_string())?;
        if leaf.kind != BusinessIndexLeafKindV1::Outcome || leaf.canonical_value != expected {
            return Err("Tick archive changed original canonical record".into());
        }
        let locator = root
            .get(&retained_key("world-tick-consensus-tick", tick)?, &overlay)?
            .ok_or("Tick locator missing")?;
        let body = Value::Map(BTreeMap::from([
            (Value::Text("height".into()), Value::Integer(height.into())),
            (Value::Text("tick".into()), Value::Integer(tick.into())),
        ]));
        let expected = serde_cbor::to_vec(&Value::Map(BTreeMap::from([(
            Value::Text("TickConsensusHeight".into()),
            body,
        )])))
        .map_err(|e| e.to_string())?;
        if locator.kind != BusinessIndexLeafKindV1::Outcome || locator.canonical_value != expected {
            return Err("Tick locator differs from original record".into());
        }
        Ok(())
    };
    for (height, tick, bytes) in rows {
        archive(height, tick, &bytes)?;
    }
    // Even unchanged prefixes require their referenced boundary to be present;
    // the root and counter are independently bound by the candidate Snapshot.
    if after.tick_consensus_archived_record_count > 0 {
        let root = new_root.ok_or("Tick prefix root absent")?;
        let boundary = root
            .get(
                &retained_key(
                    "world-tick-consensus-height",
                    after.tick_consensus_archived_record_count,
                )?,
                &overlay,
            )?
            .ok_or("Tick prefix boundary missing")?;
        let value: Value =
            serde_cbor::from_slice(&boundary.canonical_value).map_err(|e| e.to_string())?;
        let body = field(&value, "TickConsensusRecord")?;
        let Value::Bytes(bytes) = field(body, "record_cbor")? else {
            return Err("Tick boundary record bytes".into());
        };
        let record: Value = serde_cbor::from_slice(bytes).map_err(|e| e.to_string())?;
        validate_record(&record)?;
        let cert = field(&record, "certificate")?;
        if unsigned(field(cert, "consensus_height")?)? != after.tick_consensus_archived_record_count
        {
            return Err("Tick boundary height".into());
        }
        let first = after
            .tick_consensus_records
            .first()
            .ok_or("Tick prefix hot tail missing")?;
        if unsigned(field(field(field(first, "block")?, "header")?, "tick")?)?
            <= unsigned(field(body, "tick")?)?
        {
            return Err("Tick prefix/hot timestamps do not advance".into());
        }
        if field(field(field(first, "block")?, "header")?, "parent_hash")?
            != field(cert, "block_hash")?
        {
            return Err("Tick prefix/hot parent mismatch".into());
        }
        archive(
            after.tick_consensus_archived_record_count,
            unsigned(field(body, "tick")?)?,
            bytes,
        )?;
    }
    Ok(())
}
fn retained_key(namespace: &str, key: u64) -> Result<String, String> {
    let bytes = serde_cbor::to_vec(&("oasis7.formal.retained-history.key.v1", namespace, key))
        .map_err(|e| e.to_string())?;
    Ok(blake3::hash(&bytes).to_hex().to_string())
}

fn text(value: &Value) -> Result<&str, String> {
    match value {
        Value::Text(text) => Ok(text),
        _ => Err("Tick expected text".into()),
    }
}
fn validate_record(record: &Value) -> Result<(), String> {
    use sha2::{Digest, Sha256};
    let cert = field(record, "certificate")?;
    let header = field(field(record, "block")?, "header")?;
    let parent = text(field(header, "parent_hash")?)?;
    let tick = unsigned(field(header, "tick")?)?;
    let events = text(field(header, "events_hash")?)?;
    let state = text(field(header, "state_root")?)?;
    let executor = text(field(header, "executor_version")?)?;
    let schema = unsigned(field(header, "schema_version")?)?;
    let optional = |key: &str| -> Result<String, String> {
        match field(header, key) {
            Ok(Value::Text(s)) => Ok(s.clone()),
            Ok(Value::Integer(n)) => Ok(n.to_string()),
            Ok(Value::Null) | Err(_) => Ok("0".into()),
            _ => Err("Tick optional header scalar".into()),
        }
    };
    // Legacy V2 audit headers may omit optional strings. Preserve the original
    // deterministic empty-string hash semantics; this is not a fabricated
    // committed node context. New formal execution rejects empty inputs.
    let optional_text = |key: &str| -> Result<&str, String> {
        match field(header, key) {
            Ok(Value::Text(value)) => Ok(value),
            Ok(Value::Null) | Err(_) => Ok(""),
            _ => Err("Tick optional header text".into()),
        }
    };
    let payload = if schema >= 2 {
        format!(
            "tickblock:v2|{parent}|{tick}|{events}|{state}|{executor}|{}|{}|{}|{}|{}|{}",
            optional("chain_height")?,
            optional("chain_slot")?,
            optional("chain_epoch")?,
            optional_text("node_block_hash")?,
            optional_text("action_root")?,
            optional("committed_at_unix_ms")?
        )
    } else {
        format!("tickblock:v1|{parent}|{tick}|{events}|{state}|{executor}")
    };
    let hash = hex::encode(Sha256::digest(payload.as_bytes()));
    if text(field(cert, "block_hash")?)? != hash {
        return Err("Tick block checksum".into());
    }
    let source = text(field(cert, "authority_source")?)?;
    let Value::Map(signatures) = field(cert, "signatures")? else {
        return Err("Tick signatures map".into());
    };
    let threshold = unsigned(field(cert, "threshold")?)?;
    if source.trim().is_empty() || threshold == 0 || threshold > signatures.len() as u64 {
        return Err("Tick threshold/source".into());
    }
    let signature = format!(
        "sha256:{}",
        hex::encode(Sha256::digest(
            format!("tickcert:v1|{source}|{hash}").as_bytes()
        ))
    );
    if signatures.get(&Value::Text(source.into())) != Some(&Value::Text(signature)) {
        return Err("Tick source audit signature".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn record(height: u64) -> Value {
        use sha2::{Digest, Sha256};
        let mut parent = "genesis".to_string();
        for tick in 1..height {
            parent = hex::encode(Sha256::digest(
                format!("tickblock:v1|{parent}|{tick}|events|state|fixture").as_bytes(),
            ));
        }
        let hash = hex::encode(Sha256::digest(
            format!("tickblock:v1|{parent}|{height}|events|state|fixture").as_bytes(),
        ));
        let signature = format!(
            "sha256:{}",
            hex::encode(Sha256::digest(
                format!("tickcert:v1|fixture|{hash}").as_bytes()
            ))
        );
        serde_cbor::value::to_value(serde_json::json!({"certificate":{"consensus_height":height,"block_hash":hash,"authority_source":"fixture","threshold":1,"signatures":{"fixture":signature}},"block":{"header":{"schema_version":1,"tick":height,"parent_hash":parent,"events_hash":"events","state_root":"state","executor_version":"fixture"}}})).unwrap()
    }
    fn material(count: u64, rows: std::ops::RangeInclusive<u64>) -> TickSnapshotMaterial {
        let tick_consensus_records: Vec<_> = rows.map(record).collect();
        TickSnapshotMaterial {
            state: Some(TickSnapshotTime {
                time: tick_consensus_records.last().map(|record| {
                    unsigned(
                        field(
                            field(field(record, "block").unwrap(), "header").unwrap(),
                            "tick",
                        )
                        .unwrap(),
                    )
                    .unwrap()
                }),
            }),
            tick_consensus_archived_record_count: count,
            tick_consensus_total_record_count: count + tick_consensus_records.len() as u64,
            tick_consensus_records,
        }
    }
    fn snapshot(material: &TickSnapshotMaterial) -> Vec<u8> {
        serde_cbor::to_vec(&serde_json::json!({"state":{"time":material.state.as_ref().and_then(|s|s.time)},"tick_consensus_archived_record_count":material.tick_consensus_archived_record_count,"tick_consensus_total_record_count":material.tick_consensus_total_record_count,"tick_consensus_records": material.tick_consensus_records})).unwrap()
    }
    struct Empty;
    impl BusinessIndexNodeReader for Empty {
        fn read_node(&self, _: &str) -> Result<Option<Vec<u8>>, String> {
            Ok(None)
        }
    }
    #[test]
    fn retained_tick_material_rejects_missing_authenticated_archive_and_rewritten_signature() {
        let before = material(0, 1..=3);
        let after = material(1, 2..=4);
        let root = BusinessIndexRootV1::empty().unwrap();
        assert!(
            verify_tick_prefix_material(
                Some(&snapshot(&before)),
                &snapshot(&after),
                Some(&root),
                None,
                &Empty
            )
            .is_err()
        );
        let mut corrupted = material(0, 1..=3);
        let Value::Map(record) = &mut corrupted.tick_consensus_records[0] else {
            unreachable!()
        };
        let Value::Map(cert) = record.get_mut(&Value::Text("certificate".into())).unwrap() else {
            unreachable!()
        };
        cert.insert(
            Value::Text("signatures".into()),
            Value::Map(BTreeMap::new()),
        );
        assert!(TickSnapshotMaterial::decode(&snapshot(&corrupted)).is_err());
    }
    #[test]
    fn retained_tick_material_rejects_genesis_fork_and_future_tick() {
        let mut future = material(0, 1..=3);
        future.state.as_mut().unwrap().time = Some(2);
        assert!(
            TickSnapshotMaterial::decode(&snapshot(&future))
                .unwrap_err()
                .contains("beyond hash-bound")
        );
        let mut fork = material(0, 1..=3);
        let Value::Map(record) = &mut fork.tick_consensus_records[0] else {
            unreachable!()
        };
        let Value::Map(block) = record.get_mut(&Value::Text("block".into())).unwrap() else {
            unreachable!()
        };
        let Value::Map(header) = block.get_mut(&Value::Text("header".into())).unwrap() else {
            unreachable!()
        };
        header.insert(
            Value::Text("parent_hash".into()),
            Value::Text("fork".into()),
        );
        assert!(
            TickSnapshotMaterial::decode(&snapshot(&fork))
                .unwrap_err()
                .contains("genesis parent")
        );
    }
    #[test]
    fn retained_tick_material_accepts_legacy_v2_none_without_inventing_node_hash() {
        use sha2::{Digest, Sha256};
        let mut old = record(1);
        let Value::Map(body) = &mut old else {
            unreachable!()
        };
        let Value::Map(block) = body.get_mut(&Value::Text("block".into())).unwrap() else {
            unreachable!()
        };
        let Value::Map(header) = block.get_mut(&Value::Text("header".into())).unwrap() else {
            unreachable!()
        };
        header.insert(Value::Text("schema_version".into()), Value::Integer(2));
        for (name, value) in [
            ("chain_height", 1),
            ("chain_slot", 0),
            ("chain_epoch", 0),
            ("committed_at_unix_ms", 0),
        ] {
            header.insert(Value::Text(name.into()), Value::Integer(value));
        }
        header.insert(
            Value::Text("action_root".into()),
            Value::Text(String::new()),
        );
        let hash = hex::encode(Sha256::digest(
            b"tickblock:v2|genesis|1|events|state|fixture|1|0|0|||0",
        ));
        let signature = format!(
            "sha256:{}",
            hex::encode(Sha256::digest(
                format!("tickcert:v1|fixture|{hash}").as_bytes()
            ))
        );
        let Value::Map(cert) = body.get_mut(&Value::Text("certificate".into())).unwrap() else {
            unreachable!()
        };
        cert.insert(Value::Text("block_hash".into()), Value::Text(hash));
        cert.insert(
            Value::Text("signatures".into()),
            Value::Map(BTreeMap::from([(
                Value::Text("fixture".into()),
                Value::Text(signature),
            )])),
        );
        validate_record(&old).unwrap();
        let Value::Map(body) = &mut old else {
            unreachable!()
        };
        let Value::Map(block) = body.get_mut(&Value::Text("block".into())).unwrap() else {
            unreachable!()
        };
        let Value::Map(header) = block.get_mut(&Value::Text("header".into())).unwrap() else {
            unreachable!()
        };
        header.insert(
            Value::Text("node_block_hash".into()),
            Value::Text("a".repeat(64)),
        );
        assert!(
            validate_record(&old)
                .unwrap_err()
                .contains("block checksum")
        );
    }
    #[test]
    fn retained_tick_material_migrates_legacy_tail_in_bounded_exact_prefixes() {
        let original = material(0, 1..=150);
        let next = material(64, 65..=151);
        let rows = original.archived_prefix(&next).unwrap();
        assert_eq!(rows.len(), 64);
        assert_eq!(rows[0].0, 1);
        assert_eq!(rows[63].0, 64);
        let too_many = material(65, 66..=151);
        assert!(original.archived_prefix(&too_many).is_err());
        let mut rewritten = material(64, 65..=151);
        rewritten.tick_consensus_records[0] = record(66);
        assert!(original.archived_prefix(&rewritten).is_err());
        assert!(next.archived_prefix(&original).is_err());
    }
}
