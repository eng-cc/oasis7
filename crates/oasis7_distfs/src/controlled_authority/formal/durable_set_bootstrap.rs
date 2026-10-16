//! Bounded public-proof pages for endpoint-owner bootstrap. The sole origin is
//! independently approved InitialActivation; pages and source RPCs mint no pins.
use super::{crypto::*, *};
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HealthyEndpointActivationStepV1 {
    pub transition: SignedNormalDurableSetTransitionV1,
    pub old_seals: [fencing::EpochSealReceiptV1; 2],
    pub new_installs: [DurableSetInstallReceiptV1; 2],
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum HealthyEndpointLineageStepV1 {
    Commit(Box<FormalQualifiedCommitProofV1>),
    DurableSetActivation(Box<HealthyEndpointActivationStepV1>),
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HealthyEndpointLineagePageV1 {
    pub schema_version: u32,
    pub from: FormalHeadRefV1,
    #[serde(deserialize_with = "bounded_steps")]
    pub steps: Vec<HealthyEndpointLineageStepV1>,
    pub to: FormalHeadRefV1,
}
fn bounded_steps<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Vec<HealthyEndpointLineageStepV1>, D::Error> {
    struct Visitor;
    impl<'de> serde::de::Visitor<'de> for Visitor {
        type Value = Vec<HealthyEndpointLineageStepV1>;
        fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("at most 128 healthy lineage steps")
        }
        fn visit_seq<A: serde::de::SeqAccess<'de>>(
            self,
            mut sequence: A,
        ) -> Result<Self::Value, A::Error> {
            if sequence.size_hint().is_some_and(|n| n > 128) {
                return Err(serde::de::Error::custom("healthy lineage page step bound"));
            }
            let mut values = Vec::new();
            while let Some(value) = sequence.next_element()? {
                if values.len() == 128 {
                    return Err(serde::de::Error::custom("healthy lineage page step bound"));
                }
                values.push(value);
            }
            Ok(values)
        }
    }
    deserializer.deserialize_seq(Visitor)
}
impl HealthyEndpointLineagePageV1 {
    pub fn decode(bytes: &[u8]) -> Result<Self, FormalError> {
        if bytes.is_empty() || bytes.len() > 64 * 1024 * 1024 {
            return Err(invalid("healthy lineage page byte bound"));
        }
        let page: Self = serde_cbor::from_slice(bytes).map_err(|e| invalid(e.to_string()))?;
        if serde_cbor::to_vec(&page).map_err(|e| invalid(e.to_string()))? != bytes {
            return Err(invalid("healthy lineage page canonical CBOR"));
        }
        Ok(page)
    }
}
/// Keeps only current and immediate predecessor tokens. Total history has no
/// step/page cap. Recovery is explicitly unsupported here: it must use the
/// separately verified issuer/retained-origin permission bootstrap, never a raw
/// authority or a relabeled healthy step.
pub fn verify_healthy_endpoint_bootstrap(
    initial: &VerifiedAuthorityV1,
    initial_qualified: &FormalQualifiedCommitProofV1,
    evidence: &DurableSetSealedEvidenceV1,
    pages: impl IntoIterator<Item = Result<HealthyEndpointLineagePageV1, FormalError>>,
) -> Result<VerifiedSealedDurableSetTransitionV1, FormalError> {
    let mut authority = initial.clone();
    let mut current = verify_formal_qualified(initial_qualified, initial, None, None)?;
    let mut previous = None;
    let target = evidence.transition.body.boundary_position;
    if target <= current.head().position {
        return Err(invalid(
            "healthy bootstrap requires a later execution boundary",
        ));
    }
    for page in pages {
        let page = page?;
        if page.schema_version != 1
            || page.steps.is_empty()
            || page.steps.len() > 128
            || page.from != current.head()
        {
            return Err(invalid("healthy bootstrap page schema/from/step bound"));
        }
        for step in page.steps {
            match step {
                HealthyEndpointLineageStepV1::Commit(proof) => {
                    if proof.decision.branch != FormalCompletionBranchV1::Normal {
                        return Err(invalid(
                            "healthy bootstrap lacks independent recovery permission",
                        ));
                    }
                    let next = verify_formal_qualified(&proof, &authority, Some(&current), None)?;
                    if next.head().position > target {
                        return Err(invalid("healthy bootstrap passed exact H"));
                    }
                    previous = Some(current);
                    current = next;
                }
                HealthyEndpointLineageStepV1::DurableSetActivation(step) => {
                    let parent = previous
                        .as_ref()
                        .ok_or_else(|| invalid("healthy bootstrap activation lacks H-1"))?;
                    let transition = verify_normal_durable_set_transition(
                        &step.transition,
                        &authority,
                        parent,
                        &current,
                    )?;
                    let activation = verify_durable_set_activation(
                        &transition,
                        [&step.old_seals[0], &step.old_seals[1]],
                        [&step.new_installs[0], &step.new_installs[1]],
                    )?;
                    authority = activation.successor().clone();
                }
            }
        }
        if page.to != current.head() {
            return Err(invalid("healthy bootstrap page exact end"));
        }
        if current.head().position == target {
            return verify_sealed_durable_set_transition(
                evidence,
                &authority,
                previous
                    .as_ref()
                    .ok_or_else(|| invalid("healthy bootstrap boundary lacks H-1"))?,
                &current,
            );
        }
    }
    Err(invalid("healthy bootstrap missing qualified lineage page"))
}
