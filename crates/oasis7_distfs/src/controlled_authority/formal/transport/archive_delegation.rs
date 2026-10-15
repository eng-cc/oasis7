//! Writer-authorized sealed archive copy using an independent reader key.
//! The grant cannot authorize a mutation or establish a new authority.
use super::*;
const DOMAIN: &str = "oasis7.formal.sealed_archive_read_grant.writer.v1";
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArchiveReadGrantBodyV1 {
    pub schema_version: u32,
    pub world: FormalWorldIdentityV1,
    pub old_authority_id: String,
    pub source_instance_digest: String,
    pub transition_digest: String,
    pub sealed_cut: FormalHeadRefV1,
    pub next_epoch_floor: u64,
    pub target_instance_digest: String,
    pub reader_public_key: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SignedArchiveReadGrantV1 {
    pub body: ArchiveReadGrantBodyV1,
    pub writer_signature_hex: String,
}
pub fn archive_read_grant_signing_bytes(
    body: &ArchiveReadGrantBodyV1,
) -> Result<Vec<u8>, FormalError> {
    bytes(DOMAIN, body)
}
impl SignedArchiveReadGrantV1 {
    pub fn verify(
        &self,
        authority: &VerifiedAuthorityV1,
        role: EndpointRole,
    ) -> Result<(), FormalError> {
        let body = &self.body;
        for digest in [
            &body.source_instance_digest,
            &body.transition_digest,
            &body.target_instance_digest,
            &body.sealed_cut.record_id,
        ] {
            hash_hex(digest)?;
        }
        public_key(&body.reader_public_key)?;
        let source = match role {
            EndpointRole::Primary => &authority.definition().primary.instance,
            EndpointRole::Replica => &authority.definition().replica.instance,
        };
        if body.schema_version != 1
            || body.world != authority.definition().world
            || body.old_authority_id != authority.authority_id()?
            || body.source_instance_digest
                != source
                    .digest()
                    .map_err(super::super::endpoint::fencing_error)?
            || body.sealed_cut.position < authority.definition().valid_from_position
            || body.next_epoch_floor <= authority.definition().epoch
            || body.reader_public_key == authority.definition().writer_public_key
            || body.reader_public_key == authority.definition().primary.instance.endpoint_public_key
            || body.reader_public_key == authority.definition().replica.instance.endpoint_public_key
        {
            return Err(invalid(
                "sealed archive grant exact authority/source/independent reader",
            ));
        }
        verify(
            DOMAIN,
            body,
            &self.writer_signature_hex,
            &authority.definition().writer_public_key,
        )
    }
}
