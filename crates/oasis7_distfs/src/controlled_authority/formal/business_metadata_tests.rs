use super::business_index::{BusinessIndexLeafKindV1, BusinessIndexLeafV1, metadata_key_digest};
use super::*;
struct Empty;
impl BusinessIndexNodeReader for Empty {
    fn read_node(&self, _: &str) -> Result<Option<Vec<u8>>, String> {
        Ok(None)
    }
}
fn leaf(key: String, kind: BusinessIndexLeafKindV1) -> BusinessIndexLeafV1 {
    BusinessIndexLeafV1 {
        key_digest: key,
        kind,
        canonical_value: vec![1],
    }
}
#[test]
fn business_contract_still_requires_one_fixed_metadata_despite_generic_archive_deltas() {
    let empty = BusinessIndexRootV1::empty().unwrap();
    let none = BusinessIndexDeltaV1::build(
        empty.clone(),
        vec![leaf("ab".repeat(32), BusinessIndexLeafKindV1::Outcome)],
        &Empty,
    )
    .unwrap();
    assert!(verify_business_metadata_contract(&none, &Empty).is_err());
    let many = BusinessIndexDeltaV1::build(
        empty.clone(),
        vec![
            leaf("ab".repeat(32), BusinessIndexLeafKindV1::CommitMetadata),
            leaf("cd".repeat(32), BusinessIndexLeafKindV1::CommitMetadata),
        ],
        &Empty,
    )
    .unwrap();
    assert!(verify_business_metadata_contract(&many, &Empty).is_err());
    let one = BusinessIndexDeltaV1::build(
        empty,
        vec![leaf(
            metadata_key_digest(),
            BusinessIndexLeafKindV1::CommitMetadata,
        )],
        &Empty,
    )
    .unwrap();
    assert!(verify_business_metadata_contract(&one, &Empty).is_ok());
}
