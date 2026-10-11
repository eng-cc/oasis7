#[path = "../../oasis7_context/tests/support/mod.rs"]
mod support;
#[test]
fn context_does_not_require_signer_installation() {
    support::ordinary_contract(env!("CARGO_BIN_EXE_oasis7_local_signer"), &["context"]);
}
