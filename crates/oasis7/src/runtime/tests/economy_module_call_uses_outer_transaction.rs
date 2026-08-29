//! Regression contract for module-backed economy evaluation inside a staged action.
//!
//! Economy evaluation is already called on the outer action's staged `World`.
//! Runtime clone count is intentionally not observable from `ModuleSandbox`, so
//! this test guards the narrow source-level call-boundary contract until a
//! production-neutral instrumentation seam exists.

#[test]
fn economy_module_call_uses_outer_transaction() {
    let economy_source = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/runtime/world/economy.rs"
    ));
    let economy_call = economy_source
        .split_once("    fn execute_economy_module_call<T: Serialize>(")
        .map(|(_, body)| body)
        .expect("economy module call helper exists")
        .split_once("\n    }\n")
        .map(|(body, _)| body)
        .expect("economy module call helper body is delimited");

    assert!(
        economy_call.contains("execute_module_call_with_manifest_and_state_key("),
        "already-staged economy evaluation must use the current-world inner call path"
    );
    assert!(
        !economy_call.contains("execute_module_call("),
        "economy evaluation must not re-enter the public clone-and-publish boundary"
    );

    let publication_source = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/runtime/world/module_runtime_publication.rs"
    ));
    let public_call = publication_source
        .split_once("    pub fn execute_module_call(")
        .map(|(_, body)| body)
        .expect("public module call entrypoint exists")
        .split_once("\n    }\n")
        .map(|(body, _)| body)
        .expect("public module call entrypoint body is delimited");

    assert!(
        public_call.contains("let mut staged = self.clone();"),
        "public module calls must retain the clone-and-publish boundary"
    );
    assert!(
        public_call.contains("self.publish_staged_module_output(staged, result)"),
        "public module calls must publish only after the staged result completes"
    );
}
