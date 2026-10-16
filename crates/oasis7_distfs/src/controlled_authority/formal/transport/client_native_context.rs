//! Constructor visibility is confined to the authenticated client's own module.
//! No endpoint/public Query parser can manufacture a native promotion context.
use super::*;
#[derive(Clone)]
pub(crate) struct VerifiedNativeRpcContextV1 {
    checkpoint_bytes: Vec<u8>,
    minimum: FormalHeadRefV1,
    endpoint_public_key: String,
    checkpoint: fencing::EndpointCheckpointBindingV1,
    authority: VerifiedAuthorityV1,
    resource_binding: String,
    instance_digest: String,
}
pub(super) fn context_after_authenticated_call(
    response: &ResponseBody,
    authority: &VerifiedAuthorityV1,
    role: EndpointRole,
) -> Result<VerifiedNativeRpcContextV1, FormalError> {
    let instance = match role {
        EndpointRole::Primary => &authority.definition().primary.instance,
        EndpointRole::Replica => &authority.definition().replica.instance,
    };
    // call() has already verified exact freshly generated challenge, original
    // request digest, protected custody, floor and minimum ancestry bridge.
    Ok(VerifiedNativeRpcContextV1 {
        checkpoint_bytes: response.checkpoint_bytes.clone(),
        minimum: response.minimum.clone(),
        endpoint_public_key: instance.endpoint_public_key.clone(),
        checkpoint: response.checkpoint.clone(),
        authority: authority.clone(),
        resource_binding: response.provider.statement.resource_binding.clone(),
        instance_digest: response.instance_digest.clone(),
    })
}
impl VerifiedNativeRpcContextV1 {
    pub(crate) fn resource_binding(&self) -> &str {
        &self.resource_binding
    }
    pub(crate) fn instance_digest(&self) -> &str {
        &self.instance_digest
    }
    pub(crate) fn checkpoint_bytes(&self) -> &[u8] {
        &self.checkpoint_bytes
    }
    pub(crate) fn endpoint_public_key(&self) -> &str {
        &self.endpoint_public_key
    }
    pub(crate) fn minimum(&self) -> &FormalHeadRefV1 {
        &self.minimum
    }
    pub(crate) fn authority(&self) -> &VerifiedAuthorityV1 {
        &self.authority
    }
    pub(crate) fn checkpoint(&self) -> &fencing::EndpointCheckpointBindingV1 {
        &self.checkpoint
    }
}
