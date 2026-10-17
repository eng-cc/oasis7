//! Hash-bound fixed-prefix catalogs; no whole-history in-memory ID collection.
use super::*;
pub(super) const CATALOG: &str = ".catalog-v2";
const SHARD_IDS: usize = 4096;
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ShardRef {
    pub digest: String,
    pub count: usize,
}
pub(super) type Catalogs = BTreeMap<String, ShardRef>;
fn prefix(id: &str) -> Result<&str, FormalError> {
    hash_hex(id)?;
    Ok(&id[..3])
}
pub(super) fn validate(catalogs: &Catalogs) -> Result<(), FormalError> {
    if catalogs.len() > 4096 {
        return Err(invalid("generation catalog shard count"));
    }
    for (key, value) in catalogs {
        if key.len() != 3
            || !key
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
            || value.count == 0
            || value.count > SHARD_IDS
        {
            return Err(invalid("generation catalog prefix/count"));
        }
        hash_hex(&value.digest)?;
    }
    Ok(())
}
pub(super) fn load(
    dir: &Path,
    key: &str,
    reference: &ShardRef,
) -> Result<BTreeSet<String>, FormalError> {
    let bytes = read(&dir.join(CATALOG).join(&reference.digest), 512 * 1024)?;
    if blake3::hash(&bytes).to_hex().as_str() != reference.digest {
        return Err(invalid("generation catalog checksum"));
    }
    let ids: Vec<String> = serde_cbor::from_slice(&bytes).map_err(io)?;
    if ids.len() != reference.count || ids.len() > SHARD_IDS || ids.windows(2).any(|w| w[0] >= w[1])
    {
        return Err(invalid("generation catalog order/count"));
    }
    for id in &ids {
        if prefix(id)? != key {
            return Err(invalid("generation catalog key prefix"));
        }
    }
    Ok(ids.into_iter().collect())
}
pub(super) fn add(dir: &Path, catalogs: &mut Catalogs, ids: &[String]) -> Result<(), FormalError> {
    directory(&dir.join(CATALOG))?;
    let mut groups: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for id in ids {
        groups
            .entry(prefix(id)?.to_owned())
            .or_default()
            .push(id.clone());
    }
    for (key, added) in groups {
        let mut values = catalogs
            .get(&key)
            .map(|r| load(dir, &key, r))
            .transpose()?
            .unwrap_or_default();
        values.extend(added);
        if values.len() > SHARD_IDS {
            return Err(invalid("generation catalog shard budget; source retained"));
        }
        let ordered: Vec<_> = values.into_iter().collect();
        let bytes = serde_cbor::to_vec(&ordered).map_err(io)?;
        let digest = blake3::hash(&bytes).to_hex().to_string();
        publication::publish(&dir.join(CATALOG), &dir.join(CATALOG).join(&digest), &bytes)?;
        catalogs.insert(
            key,
            ShardRef {
                digest,
                count: ordered.len(),
            },
        );
    }
    validate(catalogs)
}
pub(super) fn count(catalogs: &Catalogs) -> Result<usize, FormalError> {
    catalogs.values().try_fold(0usize, |n, r| {
        n.checked_add(r.count)
            .ok_or_else(|| invalid("generation count overflow"))
    })
}
pub(super) fn page(
    dir: &Path,
    catalogs: &Catalogs,
    after: Option<&str>,
    limit: usize,
) -> Result<Vec<String>, FormalError> {
    let mut ids = Vec::new();
    for (key, reference) in catalogs {
        if after.is_some_and(|id| key.as_str() < &id[..3]) {
            continue;
        }
        for id in load(dir, key, reference)? {
            if after.is_none_or(|old| id.as_str() > old) {
                ids.push(id);
                if ids.len() == limit {
                    return Ok(ids);
                }
            }
        }
    }
    Ok(ids)
}
pub(super) fn contains(dir: &Path, catalogs: &Catalogs, id: &str) -> Result<bool, FormalError> {
    let key = prefix(id)?;
    catalogs
        .get(key)
        .map(|reference| load(dir, key, reference).map(|ids| ids.contains(id)))
        .unwrap_or(Ok(false))
}
pub(super) fn pending(
    dir: &Path,
    all: &Catalogs,
    done: &Catalogs,
    limit: usize,
) -> Result<Vec<String>, FormalError> {
    let mut result = Vec::new();
    for (key, reference) in all {
        if done.get(key) == Some(reference) {
            continue;
        }
        let completed = done
            .get(key)
            .map(|r| load(dir, key, r))
            .transpose()?
            .unwrap_or_default();
        for id in load(dir, key, reference)?.difference(&completed) {
            result.push(id.clone());
            if result.len() == limit {
                return Ok(result);
            }
        }
    }
    Ok(result)
}
