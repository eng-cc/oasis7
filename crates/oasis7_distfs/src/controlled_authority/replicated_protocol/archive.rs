//! Bounded hot checkpoint and lazy immutable archives. The endpoint signs the
//! verified prefix, radix-index root, and skip-linked head; staged files alone
//! never authorize a request, decision, or absence.
use super::endpoint::{Entry, Preparation, State};
use super::*;
use crate::controlled_authority::fencing::{
    CheckpointIntentV1, EndpointCheckpointBindingV1, EpochSealBodyV1, MonotonicEndpointGuard,
};
use ed25519_dalek::SigningKey;
use serde::de::DeserializeOwned;
use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};
use storage::Fault;
const CHECKPOINT: &str = "endpoint-hot-v2.json";
const DOMAIN: &str = "oasis7.endpoint.hot.checkpoint.v2";
const MAX_META: usize = 64 * 1024;
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Body {
    version: u32,
    trust: FixedTrust,
    role: EndpointRole,
    revision: u64,
    head: HeadAnchor,
    node: Option<String>,
    prepared: Option<String>,
    index: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    frozen: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    instance_binding: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    epoch_seal: Option<EpochSealBodyV1>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Checkpoint {
    body: Body,
    signature: String,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Node {
    anchor: HeadAnchor,
    entry: String,
    ancestors: Vec<String>,
}
pub(super) struct Store {
    path: PathBuf,
    body: Body,
}
fn read_bytes(path: &Path, max: usize) -> Result<Vec<u8>, ProtocolError> {
    storage::no_symlink(path)?;
    let meta = fs::symlink_metadata(path).map_err(io)?;
    if !meta.is_file() || meta.len() > max as u64 {
        return Err(invalid("archive regular file/size"));
    }
    let mut bytes = Vec::new();
    File::open(path)
        .map_err(io)?
        .take((max + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(io)?;
    if bytes.len() > max {
        return Err(invalid("archive size"));
    }
    Ok(bytes)
}
fn object_path(path: &Path, kind: &str, hash: &str) -> Result<PathBuf, ProtocolError> {
    crypto::hash_hex(hash)?;
    Ok(path
        .join("archive-v2")
        .join(kind)
        .join(&hash[..2])
        .join(format!("{hash}.cbor")))
}
pub(super) fn object_read<T: DeserializeOwned>(
    path: &Path,
    kind: &str,
    hash: &str,
) -> Result<T, ProtocolError> {
    let file = object_path(path, kind, hash)?;
    for parent in file.ancestors().take(4) {
        storage::no_symlink(parent)?;
    }
    let max = if kind == "entry" || kind == "prepared" || kind == "frozen" {
        storage::MAX_FILE
    } else {
        MAX_META
    };
    let bytes = read_bytes(&file, max)?;
    if blake3::hash(&bytes).to_hex().as_str() != hash {
        return Err(invalid("archive checksum"));
    }
    serde_cbor::from_slice(&bytes).map_err(|e| invalid(e.to_string()))
}
pub(super) fn object_write<T: Serialize>(
    path: &Path,
    kind: &str,
    value: &T,
) -> Result<String, ProtocolError> {
    let bytes = serde_cbor::to_vec(value).map_err(|e| invalid(e.to_string()))?;
    let max = if kind == "entry" || kind == "prepared" || kind == "frozen" {
        storage::MAX_FILE
    } else {
        MAX_META
    };
    if bytes.len() > max {
        return Err(invalid("archive object limit"));
    }
    let hash = blake3::hash(&bytes).to_hex().to_string();
    let file = object_path(path, kind, &hash)?;
    let root = path.join("archive-v2");
    storage::private_dir(&root)?;
    storage::sync_dir(path)?;
    let type_dir = root.join(kind);
    storage::private_dir(&type_dir)?;
    storage::sync_dir(&root)?;
    let shard = file.parent().expect("object shard");
    storage::private_dir(shard)?;
    storage::sync_dir(&type_dir)?;
    if file.exists() {
        if read_bytes(&file, max)? != bytes {
            return Err(invalid("immutable archive changed"));
        }
        File::open(&file).and_then(|f| f.sync_all()).map_err(io)?;
        storage::sync_dir(shard)?;
    } else {
        super::journal::atomic(shard, &file, &bytes, Fault::None)?;
    }
    Ok(hash)
}
impl Store {
    pub(super) fn exists(path: &Path) -> bool {
        path.join(CHECKPOINT).exists()
    }
    pub(super) fn empty(
        path: &Path,
        trust: FixedTrust,
        role: EndpointRole,
    ) -> Result<Self, ProtocolError> {
        let head = trust.genesis_anchor()?;
        Ok(Self {
            path: path.into(),
            body: Body {
                version: 2,
                trust,
                role,
                revision: 0,
                head,
                node: None,
                prepared: None,
                index: None,
                frozen: None,
                instance_binding: None,
                epoch_seal: None,
            },
        })
    }
    pub(super) fn open(
        path: &Path,
        trust: &FixedTrust,
        role: EndpointRole,
        minimum: &HeadAnchor,
    ) -> Result<Self, ProtocolError> {
        let store = Self::load(path, trust, role)?;
        store.minimum(minimum)?;
        store.sync_checkpoint()?;
        Ok(store)
    }
    fn load(path: &Path, trust: &FixedTrust, role: EndpointRole) -> Result<Self, ProtocolError> {
        let checkpoint: Checkpoint =
            serde_json::from_slice(&read_bytes(&path.join(CHECKPOINT), MAX_META)?)
                .map_err(|e| invalid(e.to_string()))?;
        let body = checkpoint.body;
        if body.version != 2 || &body.trust != trust || body.role != role {
            return Err(invalid("hot checkpoint identity"));
        }
        crypto::verify(DOMAIN, &body, &checkpoint.signature, trust.endpoint(role).1)?;
        if body.node.is_none() && body.head != trust.genesis_anchor()? {
            return Err(invalid("hot checkpoint genesis"));
        }
        let store = Self {
            path: path.into(),
            body,
        };
        if let Some(hash) = &store.body.node {
            let node = store.node(hash)?;
            if node.anchor != store.body.head {
                return Err(invalid("checkpoint head/node"));
            }
        }
        Ok(store)
    }
    fn node(&self, hash: &str) -> Result<Node, ProtocolError> {
        let node: Node = object_read(&self.path, "node", hash)?;
        if node.anchor.position == 0
            || node.ancestors.len() > 64
            || node.ancestors.len() != (64 - (node.anchor.position - 1).leading_zeros()) as usize
        {
            return Err(invalid("archive skip-node shape"));
        }
        crypto::hash_hex(&node.anchor.decision_hash)?;
        crypto::hash_hex(&node.entry)?;
        for hash in &node.ancestors {
            crypto::hash_hex(hash)?;
        }
        Ok(node)
    }
    fn ancestor(&self, position: u64) -> Result<Node, ProtocolError> {
        if position == 0 || position > self.body.head.position {
            return Err(invalid("archive ancestry position"));
        }
        let mut hash = self
            .body
            .node
            .clone()
            .ok_or_else(|| invalid("archive missing head"))?;
        let mut node = self.node(&hash)?;
        while node.anchor.position > position {
            let distance = node.anchor.position - position;
            let jump = (63 - distance.leading_zeros()) as usize;
            hash = node
                .ancestors
                .get(jump)
                .ok_or_else(|| invalid("archive missing skip link"))?
                .clone();
            let next = self.node(&hash)?;
            if next.anchor.position != node.anchor.position - (1u64 << jump) {
                return Err(invalid("archive skip-link position"));
            }
            node = next;
        }
        Ok(node)
    }
    pub(super) fn minimum(&self, minimum: &HeadAnchor) -> Result<(), ProtocolError> {
        crypto::hash_hex(&minimum.decision_hash)?;
        if minimum.position == 0 {
            return if minimum == &self.body.trust.genesis_anchor()? {
                Ok(())
            } else {
                Err(invalid("false genesis minimum"))
            };
        }
        let node = self.ancestor(minimum.position)?;
        if node.anchor.decision_hash != minimum.decision_hash
            || (minimum.qualified && !node.anchor.qualified)
        {
            return Err(invalid("minimum ancestry/qualification"));
        }
        // Verify the requested archived record, not only metadata membership.
        self.entry(&node)?;
        Ok(())
    }
    fn parent(&self, node: &Node) -> Result<HeadAnchor, ProtocolError> {
        if let Some(hash) = node.ancestors.first() {
            let previous = self.node(hash)?;
            if previous.anchor.position + 1 != node.anchor.position || !previous.anchor.qualified {
                return Err(invalid("archive predecessor"));
            }
            Ok(previous.anchor)
        } else {
            self.body.trust.genesis_anchor()
        }
    }
    fn entry(&self, node: &Node) -> Result<Entry, ProtocolError> {
        let entry: Entry = object_read(&self.path, "entry", &node.entry)?;
        let parent = self.parent(node)?;
        let state = State {
            schema_version: 1,
            trust: self.body.trust.clone(),
            role: self.body.role,
            prepared: vec![],
            decisions: vec![entry.clone()],
            frozen: None,
        };
        let head = super::endpoint::validate_tail(
            &state,
            &self.body.trust,
            self.body.role,
            &parent,
            &parent,
        )?;
        if head != node.anchor {
            return Err(invalid("archive entry/anchor"));
        }
        Ok(entry)
    }
    pub(super) fn head(&self) -> HeadAnchor {
        self.body.head.clone()
    }
    pub(super) fn hot(&self) -> Result<(State, HeadAnchor), ProtocolError> {
        let mut state = State {
            schema_version: 1,
            trust: self.body.trust.clone(),
            role: self.body.role,
            prepared: vec![],
            decisions: vec![],
            frozen: None,
        };
        let base = if let Some(hash) = &self.body.node {
            let node = self.node(hash)?;
            state.decisions.push(self.entry(&node)?);
            self.parent(&node)?
        } else {
            self.body.trust.genesis_anchor()?
        };
        if let Some(hash) = &self.body.prepared {
            state
                .prepared
                .push(object_read(&self.path, "prepared", hash)?);
        }
        if let Some(hash) = &self.body.frozen {
            state.frozen = Some(object_read(&self.path, "frozen", hash)?);
        }
        super::endpoint::validate_tail(
            &state,
            &self.body.trust,
            self.body.role,
            &base,
            &self.body.head,
        )?;
        Ok((state, base))
    }
    pub(super) fn refresh(&self, minimum: &HeadAnchor) -> Result<Self, ProtocolError> {
        Self::open(&self.path, &self.body.trust, self.body.role, minimum)
    }
    pub(super) fn prepared(
        &self,
        request: &LocalRequestIdentity,
    ) -> Result<Option<Preparation>, ProtocolError> {
        let key = super::archive_trie::key(request)?;
        let Some(binding) = super::archive_trie::get(&self.path, self.body.index.as_deref(), &key)?
        else {
            return Ok(None);
        };
        if binding.request != *request {
            return Err(invalid("prepared request index collision"));
        }
        let Some(hash) = binding.prepared else {
            return Ok(None);
        };
        let prepared: Preparation = object_read(&self.path, "prepared", &hash)?;
        let digest = crypto::verify_proposal(&prepared.proposal, &self.body.trust)?;
        if prepared.proposal.body.request != *request
            || prepared.proposal.body.position != binding.position
            || binding.proposal.as_deref() != Some(&digest)
        {
            return Err(invalid("archived prepared index binding"));
        }
        if let Some(receipt) = &prepared.receipt {
            crypto::verify_receipt(
                receipt,
                &self.body.trust,
                self.body.role,
                ReceiptStage::Prepared,
                &digest,
            )?;
        }
        Ok(Some(prepared))
    }
    pub(super) fn lookup(
        &self,
        request: &LocalRequestIdentity,
    ) -> Result<Option<Entry>, ProtocolError> {
        let key = super::archive_trie::key(request)?;
        let Some(binding) = super::archive_trie::get(&self.path, self.body.index.as_deref(), &key)?
        else {
            return Ok(None);
        };
        if &binding.request != request {
            return Err(invalid("request index collision"));
        }
        if binding.prepared.is_some() {
            self.prepared(request)?;
            return Ok(None);
        }
        if binding.position > self.body.head.position {
            let Some(hash) = &self.body.prepared else {
                if let Some(hash) = &self.body.frozen {
                    let frozen: FrozenReservation = object_read(&self.path, "frozen", hash)?;
                    frozen.validate(&self.body.trust, self.body.role)?;
                    if frozen.input.body.request == *request
                        && binding.frozen.as_deref()
                            == Some(&frozen.input.verify(&self.body.trust)?)
                    {
                        return Ok(None);
                    }
                }
                return Err(invalid("index outside committed prefix"));
            };
            let prepared: Preparation = object_read(&self.path, "prepared", hash)?;
            if prepared.proposal.body.request != *request
                || crypto::verify_proposal(&prepared.proposal, &self.body.trust)?.as_str()
                    != binding.proposal.as_deref().unwrap_or("")
            {
                return Err(invalid("prepared index binding"));
            }
            return Ok(None);
        }
        let node = self.ancestor(binding.position)?;
        let entry = self.entry(&node)?;
        if entry.proposal.body.request != *request
            || node.anchor.decision_hash.as_str() != binding.proposal.as_deref().unwrap_or("")
        {
            return Err(invalid("archive request binding"));
        }
        Ok(Some(entry))
    }
    fn bind(&mut self, proposal: &SignedProposal) -> Result<(), ProtocolError> {
        let binding = super::archive_trie::Binding {
            request: proposal.body.request.clone(),
            position: proposal.body.position,
            proposal: Some(crypto::verify_proposal(proposal, &self.body.trust)?),
            frozen: proposal.body.frozen_input_digest.clone(),
            prepared: None,
        };
        self.body.index = Some(super::archive_trie::insert(
            &self.path,
            self.body.index.as_deref(),
            binding,
        )?);
        Ok(())
    }
    pub(super) fn stage(&mut self, next: &State) -> Result<(), ProtocolError> {
        if next.trust != self.body.trust || next.role != self.body.role || next.decisions.len() > 2
        {
            return Err(invalid("bounded hot state identity/size"));
        }
        for entry in &next.decisions {
            let position = entry.proposal.body.position;
            if position < self.body.head.position {
                continue;
            }
            if position > self.body.head.position + 1 {
                return Err(invalid("archive append gap"));
            }
            self.bind(&entry.proposal)?;
            let digest = crypto::verify_proposal(&entry.proposal, &self.body.trust)?;
            let ancestors = if position == self.body.head.position {
                if digest != self.body.head.decision_hash {
                    return Err(invalid("archive head replacement"));
                }
                self.node(
                    self.body
                        .node
                        .as_deref()
                        .ok_or_else(|| invalid("head node"))?,
                )?
                .ancestors
            } else {
                if (self.body.head.position != 0 && !self.body.head.qualified)
                    || entry.proposal.body.parent_hash != self.body.head.decision_hash
                {
                    return Err(invalid("archive append parent"));
                }
                let mut links = Vec::new();
                if let Some(hash) = &self.body.node {
                    links.push(hash.clone());
                }
                let mut level = 1;
                while let Some(hash) = links.get(level - 1) {
                    let previous = self.node(hash)?;
                    let Some(ancestor) = previous.ancestors.get(level - 1) else {
                        break;
                    };
                    links.push(ancestor.clone());
                    level += 1;
                }
                links
            };
            let node = Node {
                anchor: HeadAnchor {
                    position,
                    decision_hash: digest,
                    qualified: entry.published,
                },
                entry: object_write(&self.path, "entry", entry)?,
                ancestors,
            };
            self.entry(&node)?;
            let hash = object_write(&self.path, "node", &node)?;
            self.body.node = Some(hash);
            self.body.head = node.anchor;
        }
        self.body.frozen = if let Some(frozen) = &next.frozen {
            frozen.validate(&self.body.trust, self.body.role)?;
            let digest = frozen.input.verify(&self.body.trust)?;
            let key = super::archive_trie::key(&frozen.input.body.request)?;
            let existing = super::archive_trie::get(&self.path, self.body.index.as_deref(), &key)?;
            let binding = super::archive_trie::Binding {
                request: frozen.input.body.request.clone(),
                position: frozen.input.body.next_position,
                proposal: existing.as_ref().and_then(|v| v.proposal.clone()),
                prepared: existing.as_ref().and_then(|v| v.prepared.clone()),
                frozen: Some(digest),
            };
            self.body.index = Some(super::archive_trie::insert(
                &self.path,
                self.body.index.as_deref(),
                binding,
            )?);
            Some(object_write(&self.path, "frozen", frozen)?)
        } else {
            None
        };
        self.body.prepared = None;
        for preparation in &next.prepared {
            let proposal = &preparation.proposal;
            let hash = object_write(&self.path, "prepared", preparation)?;
            let binding = super::archive_trie::Binding {
                request: proposal.body.request.clone(),
                position: proposal.body.position,
                proposal: Some(crypto::verify_proposal(proposal, &self.body.trust)?),
                frozen: proposal.body.frozen_input_digest.clone(),
                prepared: Some(hash.clone()),
            };
            self.body.index = Some(super::archive_trie::insert(
                &self.path,
                self.body.index.as_deref(),
                binding,
            )?);
            if proposal.body.position == self.body.head.position + 1 {
                self.body.prepared = Some(hash);
            }
        }
        Ok(())
    }
    pub(super) fn publish(&mut self, key: &SigningKey, fault: Fault) -> Result<(), ProtocolError> {
        self.publish_guarded(key, fault, None)
    }
    pub(super) fn publish_guarded(
        &mut self,
        key: &SigningKey,
        fault: Fault,
        guard: Option<&mut dyn MonotonicEndpointGuard>,
    ) -> Result<(), ProtocolError> {
        let previous = Self::checkpoint_binding_at(&self.path).ok();
        self.body.revision = self
            .body
            .revision
            .checked_add(1)
            .ok_or_else(|| invalid("checkpoint revision exhausted"))?;
        let signature = crypto::sign(DOMAIN, &self.body, key)?;
        let bytes = serde_json::to_vec(&Checkpoint {
            body: self.body.clone(),
            signature,
        })
        .map_err(|e| invalid(e.to_string()))?;
        if bytes.len() > MAX_META {
            return Err(invalid("checkpoint limit"));
        }
        if let Some(guard) = guard {
            let previous = previous.ok_or_else(|| invalid("guarded checkpoint absent"))?;
            let next = EndpointCheckpointBindingV1 {
                revision: self.body.revision,
                commitment: blake3::hash(&bytes).to_hex().to_string(),
                head: self.body.head.clone(),
            };
            let seal = if guard.seal().is_none() {
                self.body.epoch_seal.clone()
            } else {
                None
            };
            guard
                .begin(CheckpointIntentV1 {
                    previous,
                    next,
                    checkpoint_bytes: bytes.clone(),
                    seal,
                })
                .map_err(super::endpoint::fencing_error)?;
            super::journal::atomic(&self.path, &self.path.join(CHECKPOINT), &bytes, fault)?;
            guard.finish().map_err(super::endpoint::fencing_error)?;
            Ok(())
        } else {
            if self.body.instance_binding.is_some() {
                return Err(invalid(
                    "FENCING_UNPROVEN: bound checkpoint requires provider guard",
                ));
            }
            super::journal::atomic(&self.path, &self.path.join(CHECKPOINT), &bytes, fault)
        }
    }
    pub(super) fn checkpoint_binding_at(
        path: &Path,
    ) -> Result<EndpointCheckpointBindingV1, ProtocolError> {
        let bytes = read_bytes(&path.join(CHECKPOINT), MAX_META)?;
        let cp: Checkpoint = serde_json::from_slice(&bytes).map_err(|e| invalid(e.to_string()))?;
        Ok(EndpointCheckpointBindingV1 {
            revision: cp.body.revision,
            commitment: blake3::hash(&bytes).to_hex().to_string(),
            head: cp.body.head,
        })
    }
    pub(super) fn instance_binding(&self) -> Option<&str> {
        self.body.instance_binding.as_deref()
    }
    pub(super) fn bind_instance(&mut self, digest: String) {
        self.body.instance_binding = Some(digest);
    }
    pub(super) fn seal(&self) -> Option<&EpochSealBodyV1> {
        self.body.epoch_seal.as_ref()
    }
    pub(super) fn stage_seal(&mut self, seal: EpochSealBodyV1) {
        self.body.epoch_seal = Some(seal);
    }
    pub(super) fn reconcile_guard(
        path: &Path,
        trust: &FixedTrust,
        role: EndpointRole,
        minimum: &HeadAnchor,
        guard: &mut dyn MonotonicEndpointGuard,
    ) -> Result<(), ProtocolError> {
        let actual = Self::checkpoint_binding_at(path)?;
        if let Some(intent) = guard.pending().cloned() {
            if actual != intent.previous && actual != intent.next {
                return Err(ProtocolError::Poisoned);
            }
            if blake3::hash(&intent.checkpoint_bytes).to_hex().as_str() != intent.next.commitment {
                return Err(ProtocolError::Poisoned);
            }
            let cp: Checkpoint = serde_json::from_slice(&intent.checkpoint_bytes)
                .map_err(|e| invalid(e.to_string()))?;
            if &cp.body.trust != trust
                || cp.body.role != role
                || cp.body.head != intent.next.head
                || cp.body.revision != intent.next.revision
            {
                return Err(ProtocolError::Poisoned);
            }
            crypto::verify(DOMAIN, &cp.body, &cp.signature, trust.endpoint(role).1)?;
            let candidate = Self {
                path: path.into(),
                body: cp.body,
            };
            candidate.minimum(minimum)?;
            candidate.hot()?;
            if actual == intent.previous {
                super::journal::atomic(
                    path,
                    &path.join(CHECKPOINT),
                    &intent.checkpoint_bytes,
                    Fault::None,
                )?;
            }
            // A previous crash may have exposed the new rename before its
            // directory sync. Never finalize external progress until the
            // native checkpoint is durable in either recovery branch.
            File::open(path.join(CHECKPOINT))
                .and_then(|f| f.sync_all())
                .map_err(io)?;
            storage::sync_dir(path)?;
            guard.finish().map_err(super::endpoint::fencing_error)?;
        } else if &actual != guard.checkpoint() {
            return Err(ProtocolError::Poisoned);
        }
        Ok(())
    }
    fn sync_checkpoint(&self) -> Result<(), ProtocolError> {
        File::open(self.path.join(CHECKPOINT))
            .and_then(|f| f.sync_all())
            .map_err(io)?;
        storage::sync_dir(&self.path)
    }
    pub(super) fn export(&self) -> Result<Vec<DurabilityEvidence>, ProtocolError> {
        let mut result = Vec::new();
        for position in 1..=self.body.head.position {
            let entry = self.entry(&self.ancestor(position)?)?;
            if let Some(e) = entry.evidence() {
                result.push(e);
            }
        }
        Ok(result)
    }
}
