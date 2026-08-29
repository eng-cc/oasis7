use oasis7_wasm_abi::{
    ModuleCallCaller, ModuleCallFailure, ModuleCallOrigin, ModuleCommandEnvelope,
    ModuleInvocationProvenance, ModuleOutput, ModuleSandbox,
};

use super::super::{WorldError, WorldEventBody};
use super::World;

impl World {
    pub fn execute_module_call(
        &mut self,
        module_id: &str,
        trace_id: impl Into<String>,
        input: Vec<u8>,
        sandbox: &mut dyn ModuleSandbox,
    ) -> Result<ModuleOutput, WorldError> {
        let mut staged = self.clone();
        let result = (|| {
            let manifest = staged.active_module_manifest(module_id)?.clone();
            staged.execute_module_call_with_manifest_and_state_key(
                module_id,
                module_id,
                &manifest,
                trace_id.into(),
                input,
                sandbox,
            )
        })();
        self.publish_staged_module_output(staged, result)
    }

    /// Execute a declared module command through the existing metered call path.
    ///
    /// Command admission is completed before the sandbox is touched, then the
    /// complete call is staged so only a successful result publishes mutation.
    pub fn execute_module_command(
        &mut self,
        module_id: &str,
        trace_id: impl Into<String>,
        envelope: ModuleCommandEnvelope,
        sandbox: &mut dyn ModuleSandbox,
    ) -> Result<ModuleOutput, WorldError> {
        self.execute_module_command_with_provenance(
            module_id,
            trace_id,
            envelope,
            ModuleInvocationProvenance {
                caller: ModuleCallCaller::LegacyUnspecified,
                origin: ModuleCallOrigin {
                    kind: "legacy_unspecified".to_string(),
                    id: "legacy_unspecified".to_string(),
                },
            },
            sandbox,
        )
    }

    /// Execute a module command while keeping trusted provenance outside the
    /// untrusted command envelope and publishing the resulting world atomically.
    pub fn execute_module_command_with_provenance(
        &mut self,
        module_id: &str,
        trace_id: impl Into<String>,
        envelope: ModuleCommandEnvelope,
        provenance: ModuleInvocationProvenance,
        sandbox: &mut dyn ModuleSandbox,
    ) -> Result<ModuleOutput, WorldError> {
        let mut staged = self.clone();
        let result = staged.execute_module_command_with_provenance_inner(
            module_id, trace_id, envelope, provenance, sandbox,
        );
        self.publish_staged_module_output(staged, result)
    }

    fn publish_staged_module_output(
        &mut self,
        staged: Self,
        result: Result<ModuleOutput, WorldError>,
    ) -> Result<ModuleOutput, WorldError> {
        match result {
            Ok(output) => {
                *self = staged;
                Ok(output)
            }
            Err(error @ WorldError::ModuleCallFailed { .. }) => {
                let WorldError::ModuleCallFailed {
                    module_id,
                    trace_id,
                    code,
                    detail,
                } = &error
                else {
                    unreachable!("error pattern checked above")
                };
                self.append_event(
                    WorldEventBody::ModuleCallFailed(ModuleCallFailure {
                        module_id: module_id.clone(),
                        trace_id: trace_id.clone(),
                        code: code.clone(),
                        detail: detail.clone(),
                    }),
                    None,
                )?;
                Err(error)
            }
            Err(error) => Err(error),
        }
    }
}
