use super::*;
use ed25519_dalek::SigningKey;
use std::fs::File;
use std::path::{Path, PathBuf};
use storage::Fault;

const MAX_ENTRIES: usize = 64;
const MAX_ALL_RECORD_BYTES: usize = 16 * 1024 * 1024;
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Preparation {
    proposal: SignedProposal,
    receipt: Option<DurableReceipt>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Entry {
    proposal: SignedProposal,
    replica_prepare: DurableReceipt,
    primary_receipt: Option<DurableReceipt>,
    replica_receipt: Option<DurableReceipt>,
    published: bool,
}
impl Entry {
    fn evidence(&self) -> Option<DurabilityEvidence> {
        Some(DurabilityEvidence {
            proposal: self.proposal.clone(),
            replica_prepare: self.replica_prepare.clone(),
            primary_receipt: self.primary_receipt.clone()?,
            replica_receipt: self.replica_receipt.clone()?,
        })
    }
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct State {
    schema_version: u32,
    trust: FixedTrust,
    role: EndpointRole,
    #[serde(deserialize_with = "decode_prepared")]
    prepared: Vec<Preparation>,
    #[serde(deserialize_with = "decode_decisions")]
    decisions: Vec<Entry>,
}
fn decode_prepared<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Vec<Preparation>, D::Error> {
    package::bounded_vec::<D, Preparation, MAX_ENTRIES>(d)
}
fn decode_decisions<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Vec<Entry>, D::Error> {
    package::bounded_vec::<D, Entry, MAX_ENTRIES>(d)
}

/// This endpoint owns its final storage snapshot: callers cannot advance epoch
/// or replace identity through an append request. Its lock/CAS covers cooperating
/// processes sharing this actual directory. A service must exclusively expose
/// these methods to remote writers; direct disk access is outside this contract.
pub struct FileEndpoint {
    path: PathBuf,
    _lock: File,
    key: SigningKey,
    state: State,
    head: HeadAnchor,
    poisoned: bool,
    #[cfg(test)]
    fault: Fault,
    #[cfg(test)]
    fault_after: usize,
    #[cfg(test)]
    fail_read_sync: bool,
}
impl FileEndpoint {
    pub fn open(
        path: impl AsRef<Path>,
        trust: FixedTrust,
        role: EndpointRole,
        key: SigningKey,
        minimum: &HeadAnchor,
    ) -> Result<Self, ProtocolError> {
        trust.validate()?;
        if hex::encode(key.verifying_key().to_bytes()) != trust.endpoint(role).1 {
            return Err(invalid("endpoint signer mismatch"));
        }
        let path = path.as_ref().to_owned();
        let lock = storage::lock(&path)?;
        let state = match storage::read(&path)? {
            Some(bytes) => {
                serde_json::from_slice::<State>(&bytes).map_err(|e| invalid(e.to_string()))?
            }
            None => State {
                schema_version: 1,
                trust: trust.clone(),
                role,
                prepared: Vec::new(),
                decisions: Vec::new(),
            },
        };
        let head = validate_state(&state, &trust, role, minimum)?;
        let mut endpoint = Self {
            path,
            _lock: lock,
            key,
            state,
            head,
            poisoned: false,
            #[cfg(test)]
            fault: Fault::None,
            #[cfg(test)]
            fault_after: 1,
            #[cfg(test)]
            fail_read_sync: false,
        };
        if storage::read(&endpoint.path)?.is_none() {
            endpoint.persist(endpoint.state.clone())?;
        } else {
            storage::sync(&endpoint.path)?;
        }
        Ok(endpoint)
    }
    pub fn role(&self) -> EndpointRole {
        self.state.role
    }
    pub fn trust(&self) -> &FixedTrust {
        &self.state.trust
    }
    pub fn head(&self) -> HeadAnchor {
        self.head.clone()
    }
    pub(super) fn path(&self) -> &Path {
        &self.path
    }
    /// Re-read, validate and sync under the retained lock. Does not unpoison.
    /// A missing local record never proves global absence or an aborted request.
    pub fn lookup(
        &mut self,
        request: &LocalRequestIdentity,
    ) -> Result<EndpointStatus, ProtocolError> {
        request.validate().map_err(|e| invalid(e.to_string()))?;
        let refreshed = (|| {
            let bytes =
                storage::read(&self.path)?.ok_or_else(|| invalid("endpoint disappeared"))?;
            let next: State = serde_json::from_slice(&bytes).map_err(|e| invalid(e.to_string()))?;
            let head = validate_state(&next, &self.state.trust, self.state.role, &self.head)?;
            #[cfg(test)]
            if self.fail_read_sync {
                return Err(io(std::io::Error::other("injected read sync failure")));
            }
            storage::sync(&self.path)?;
            Ok((next, head))
        })();
        let (next, head) = match refreshed {
            Ok(value) => value,
            Err(error) => {
                self.poisoned = true;
                return Err(error);
            }
        };
        self.state = next;
        self.head = head;
        if let Some(e) = self
            .state
            .decisions
            .iter()
            .find(|e| &e.proposal.body.request == request)
        {
            return Ok(if e.published {
                EndpointStatus::Qualified(Box::new(e.evidence().expect("validated")))
            } else {
                EndpointStatus::DecisionUnqualified(Box::new(e.proposal.clone()))
            });
        }
        Ok(self
            .state
            .prepared
            .iter()
            .find(|e| &e.proposal.body.request == request)
            .map(|e| EndpointStatus::Prepared(Box::new(e.proposal.clone())))
            .unwrap_or(EndpointStatus::NotRecorded))
    }
    pub fn prepare(&mut self, proposal: &SignedProposal) -> Result<DurableReceipt, ProtocolError> {
        self.ensure_writable()?;
        let digest = crypto::verify_proposal(proposal, &self.state.trust)?;
        self.check_collision(proposal)?;
        if let Some(e) = self.state.prepared.iter().find(|p| p.proposal == *proposal)
            && let Some(r) = &e.receipt
        {
            return Ok(r.clone());
        }
        if let Some(e) = self
            .state
            .decisions
            .iter()
            .find(|p| p.proposal == *proposal)
        {
            // A durable decision subsumes this preparation. Replica's original
            // prepare receipt remains stored in the decision transaction.
            if self.role() == EndpointRole::Replica {
                return Ok(e.replica_prepare.clone());
            }
        }
        self.fence(proposal)?;
        let mut next = self.state.clone();
        if !next.prepared.iter().any(|p| p.proposal == *proposal) {
            next.prepared.push(Preparation {
                proposal: proposal.clone(),
                receipt: None,
            });
            self.persist(next)?;
        }
        // Signing occurs only AFTER the full proposal/artifact closure fsync.
        let receipt = crypto::sign_receipt(
            &self.state.trust,
            self.role(),
            ReceiptStage::Prepared,
            digest,
            &self.key,
        )?;
        let mut next = self.state.clone();
        next.prepared
            .iter_mut()
            .find(|p| p.proposal == *proposal)
            .expect("prepared")
            .receipt = Some(receipt.clone());
        self.persist(next)?;
        Ok(receipt)
    }
    pub fn decide_primary(
        &mut self,
        proposal: &SignedProposal,
        replica_prepare: &DurableReceipt,
    ) -> Result<DurableReceipt, ProtocolError> {
        if self.role() != EndpointRole::Primary {
            return Err(invalid("primary method at replica"));
        }
        self.decide(proposal, replica_prepare, None)
    }
    pub fn decide_replica(
        &mut self,
        proposal: &SignedProposal,
        replica_prepare: &DurableReceipt,
        primary_receipt: &DurableReceipt,
    ) -> Result<DurableReceipt, ProtocolError> {
        if self.role() != EndpointRole::Replica {
            return Err(invalid("replica method at primary"));
        }
        self.decide(proposal, replica_prepare, Some(primary_receipt))
    }
    fn decide(
        &mut self,
        proposal: &SignedProposal,
        preparation: &DurableReceipt,
        primary: Option<&DurableReceipt>,
    ) -> Result<DurableReceipt, ProtocolError> {
        self.ensure_writable()?;
        let digest = crypto::verify_proposal(proposal, &self.state.trust)?;
        crypto::verify_receipt(
            preparation,
            &self.state.trust,
            EndpointRole::Replica,
            ReceiptStage::Prepared,
            &digest,
        )?;
        if let Some(receipt) = primary {
            crypto::verify_receipt(
                receipt,
                &self.state.trust,
                EndpointRole::Primary,
                ReceiptStage::DecisionDurable,
                &digest,
            )?;
        }
        self.check_collision(proposal)?;
        let existing = self
            .state
            .decisions
            .iter()
            .position(|e| e.proposal == *proposal);
        if let Some(i) = existing {
            let local = match self.role() {
                EndpointRole::Primary => &self.state.decisions[i].primary_receipt,
                EndpointRole::Replica => &self.state.decisions[i].replica_receipt,
            };
            if let Some(r) = local {
                return Ok(r.clone());
            }
        } else {
            self.fence(proposal)?;
            if self.state.decisions.last().is_some_and(|e| !e.published) {
                return Err(invalid("finish previous decision before advancing"));
            }
            let mut next = self.state.clone();
            next.prepared
                .retain(|p| p.proposal.body.request != proposal.body.request);
            next.decisions.push(Entry {
                proposal: proposal.clone(),
                replica_prepare: preparation.clone(),
                primary_receipt: primary.cloned(),
                replica_receipt: None,
                published: false,
            });
            // The irrevocable CAS updates history/head/request facts in ONE snapshot.
            self.persist(next)?;
        }
        let receipt = crypto::sign_receipt(
            &self.state.trust,
            self.role(),
            ReceiptStage::DecisionDurable,
            digest,
            &self.key,
        )?;
        let mut next = self.state.clone();
        let entry = next
            .decisions
            .iter_mut()
            .find(|e| e.proposal == *proposal)
            .expect("decision");
        match self.state.role {
            EndpointRole::Primary => entry.primary_receipt = Some(receipt.clone()),
            EndpointRole::Replica => entry.replica_receipt = Some(receipt.clone()),
        }
        self.persist(next)?;
        Ok(receipt)
    }
    /// Final public evidence is saved locally only after both durable decision
    /// attestations exist. Coordinator saves it at BOTH endpoints before return.
    pub fn finalize(&mut self, evidence: &DurabilityEvidence) -> Result<(), ProtocolError> {
        self.ensure_writable()?;
        let body = &evidence.proposal.body;
        if body.position == 0 {
            return Err(invalid("zero evidence position"));
        }
        let parent = HeadAnchor {
            position: body.position - 1,
            decision_hash: body.parent_hash.clone(),
            qualified: false,
        };
        crypto::verify_evidence(evidence, &self.state.trust, &parent)?;
        let mut next = self.state.clone();
        let entry = next
            .decisions
            .iter_mut()
            .find(|e| e.proposal == evidence.proposal)
            .ok_or_else(|| invalid("finalize without local decision"))?;
        if entry.replica_prepare != evidence.replica_prepare {
            return Err(invalid("different preparation"));
        }
        entry.primary_receipt = Some(evidence.primary_receipt.clone());
        entry.replica_receipt = Some(evidence.replica_receipt.clone());
        entry.published = true;
        self.persist(next)
    }
    /// Export locally stored evidence for recovery, not a live activation claim.
    pub fn export_evidence(&self) -> Vec<DurabilityEvidence> {
        self.state
            .decisions
            .iter()
            .filter_map(Entry::evidence)
            .collect()
    }
    /// After explicit reopen, regenerate a missing local receipt from the same
    /// verified/synced decision. This does not advance head or publish evidence.
    /// It lets a surviving replica finish its receipt before restoring a lost
    /// primary, even when the original reply/receipt write was interrupted.
    pub fn repair_local_receipt(
        &mut self,
        request: &LocalRequestIdentity,
    ) -> Result<ReceiptRepairOutcome, ProtocolError> {
        request.validate().map_err(|e| invalid(e.to_string()))?;
        if self.poisoned {
            return Ok(ReceiptRepairOutcome::Unknown {
                request: request.clone(),
            });
        }
        let result = self.repair_receipt_inner(request);
        match result {
            Err(_) if self.poisoned => Ok(ReceiptRepairOutcome::Unknown {
                request: request.clone(),
            }),
            other => other,
        }
    }
    fn repair_receipt_inner(
        &mut self,
        request: &LocalRequestIdentity,
    ) -> Result<ReceiptRepairOutcome, ProtocolError> {
        self.lookup(request)?;
        let Some(entry) = self
            .state
            .decisions
            .iter()
            .find(|e| &e.proposal.body.request == request)
            .cloned()
        else {
            return Ok(ReceiptRepairOutcome::NotLocallyDecided);
        };
        let receipt = match self.role() {
            EndpointRole::Primary => {
                self.decide_primary(&entry.proposal, &entry.replica_prepare)?
            }
            EndpointRole::Replica => self.decide_replica(
                &entry.proposal,
                &entry.replica_prepare,
                entry
                    .primary_receipt
                    .as_ref()
                    .ok_or_else(|| invalid("replica missing primary receipt"))?,
            )?,
        };
        Ok(ReceiptRepairOutcome::Durable(Box::new(receipt)))
    }
    /// Restore a lost endpoint only from a complete, contiguous two-receipt
    /// history and external minimum anchor. No endpoint/key/epoch replacement.
    /// Prepared-only or unqualified history cannot be promoted by this method.
    pub fn restore(
        path: impl AsRef<Path>,
        trust: FixedTrust,
        role: EndpointRole,
        key: SigningKey,
        minimum: &HeadAnchor,
        history: &[DurabilityEvidence],
    ) -> Result<Self, ProtocolError> {
        trust.validate()?;
        if history.is_empty() {
            return Err(invalid("empty recovery history"));
        }
        let mut endpoint = Self::open(path, trust.clone(), role, key, &trust.genesis_anchor()?)?;
        if !endpoint.state.decisions.is_empty() || !endpoint.state.prepared.is_empty() {
            return Err(invalid("restore requires new empty endpoint"));
        }
        let mut next = endpoint.state.clone();
        let mut parent = trust.genesis_anchor()?;
        for e in history {
            parent = crypto::verify_evidence(e, &trust, &parent)?;
            next.decisions.push(Entry {
                proposal: e.proposal.clone(),
                replica_prepare: e.replica_prepare.clone(),
                primary_receipt: Some(e.primary_receipt.clone()),
                replica_receipt: Some(e.replica_receipt.clone()),
                published: true,
            });
        }
        validate_state(&next, &trust, role, minimum)?;
        endpoint.persist(next)?;
        Ok(endpoint)
    }
    fn fence(&self, proposal: &SignedProposal) -> Result<(), ProtocolError> {
        if self.head.position.checked_add(1) != Some(proposal.body.position)
            || self.head.decision_hash != proposal.body.parent_hash
        {
            return Err(invalid("atomic expected epoch/head/sequence fence"));
        }
        Ok(())
    }
    fn check_collision(&self, proposal: &SignedProposal) -> Result<(), ProtocolError> {
        let previous = self
            .state
            .decisions
            .iter()
            .map(|e| &e.proposal)
            .chain(self.state.prepared.iter().map(|e| &e.proposal))
            .find(|p| p.body.request == proposal.body.request);
        if previous.is_some_and(|p| p != proposal) {
            return Err(invalid(
                "request identity bound to different payload/record/parent",
            ));
        }
        Ok(())
    }
    fn ensure_writable(&self) -> Result<(), ProtocolError> {
        if self.poisoned {
            Err(ProtocolError::Poisoned)
        } else {
            Ok(())
        }
    }
    fn persist(&mut self, next: State) -> Result<(), ProtocolError> {
        let head = validate_state(&next, &self.state.trust, self.state.role, &self.head)?;
        let bytes = serde_json::to_vec(&next).map_err(|e| invalid(e.to_string()))?;
        #[cfg(test)]
        let fault = if self.fault_after <= 1 {
            self.fault
        } else {
            self.fault_after -= 1;
            Fault::None
        };
        #[cfg(not(test))]
        let fault = Fault::None;
        if let Err(error) = storage::persist(&self.path, &bytes, fault) {
            self.poisoned = true;
            return Err(error);
        }
        self.state = next;
        self.head = head;
        Ok(())
    }
    #[cfg(test)]
    pub(super) fn inject_after(&mut self, writes: usize, fault: Fault) {
        self.fault = fault;
        self.fault_after = writes;
    }
    #[cfg(test)]
    pub(super) fn inject_read_sync_failure(&mut self, fail: bool) {
        self.fail_read_sync = fail;
    }
}

fn validate_state(
    state: &State,
    trust: &FixedTrust,
    role: EndpointRole,
    minimum: &HeadAnchor,
) -> Result<HeadAnchor, ProtocolError> {
    if state.schema_version != 1
        || &state.trust != trust
        || state.role != role
        || state.decisions.len() > MAX_ENTRIES
        || state.prepared.len() > MAX_ENTRIES
    {
        return Err(invalid("endpoint identity/schema/entry limit"));
    }
    crypto::hash_hex(&minimum.decision_hash)?;
    if minimum.position == 0 && minimum != &trust.genesis_anchor()? {
        return Err(invalid("false genesis minimum"));
    }
    let mut requests = std::collections::BTreeSet::new();
    let mut head = trust.genesis_anchor()?;
    let mut minimum_seen = minimum.position == 0;
    let mut total = 0usize;
    for e in &state.decisions {
        let digest = crypto::verify_proposal(&e.proposal, trust)?;
        let b = &e.proposal.body;
        if b.position != head.position + 1
            || b.parent_hash != head.decision_hash
            || !requests.insert(b.request.clone())
        {
            return Err(invalid("decision history/request fork"));
        }
        crypto::verify_receipt(
            &e.replica_prepare,
            trust,
            EndpointRole::Replica,
            ReceiptStage::Prepared,
            &digest,
        )?;
        if role == EndpointRole::Replica && e.primary_receipt.is_none() {
            return Err(invalid("replica decision missing primary receipt"));
        }
        for (receipt, receipt_role) in [
            (&e.primary_receipt, EndpointRole::Primary),
            (&e.replica_receipt, EndpointRole::Replica),
        ] {
            if let Some(r) = receipt {
                crypto::verify_receipt(
                    r,
                    trust,
                    receipt_role,
                    ReceiptStage::DecisionDurable,
                    &digest,
                )?;
            }
        }
        if e.published {
            crypto::verify_evidence(
                &e.evidence()
                    .ok_or_else(|| invalid("published without dual receipts"))?,
                trust,
                &head,
            )?;
        }
        head = HeadAnchor {
            position: b.position,
            decision_hash: digest,
            qualified: e.published,
        };
        if head.position == minimum.position {
            if head.decision_hash != minimum.decision_hash || (minimum.qualified && !head.qualified)
            {
                return Err(invalid("minimum decision/evidence anchor mismatch"));
            }
            minimum_seen = true;
        }
        total = total
            .checked_add(b.record.byte_len())
            .ok_or_else(|| invalid("byte overflow"))?;
    }
    for p in &state.prepared {
        let digest = crypto::verify_proposal(&p.proposal, trust)?;
        if !requests.insert(p.proposal.body.request.clone()) {
            return Err(invalid("duplicate prepared request"));
        }
        if let Some(r) = &p.receipt {
            crypto::verify_receipt(r, trust, role, ReceiptStage::Prepared, &digest)?;
        }
        // Competing preparations may become stale; preserve them as pending,
        // never infer abort or apply them to the decided world.
        total = total
            .checked_add(p.proposal.body.record.byte_len())
            .ok_or_else(|| invalid("byte overflow"))?;
    }
    if !minimum_seen || total > MAX_ALL_RECORD_BYTES {
        return Err(invalid("rollback/minimum anchor or aggregate byte limit"));
    }
    Ok(head)
}
