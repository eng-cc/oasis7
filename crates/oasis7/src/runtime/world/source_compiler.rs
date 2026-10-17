use super::*;

impl World {
    /// Install a process-local development compiler. Admission policy still gates invocation.
    /// The capability is not serialized into snapshots or replay journals.
    pub fn set_module_source_compiler(
        &mut self,
        compiler: Arc<dyn oasis7_agent_api::module_source::ModuleSourceCompiler>,
    ) {
        self.source_compiler =
            super::super::module_source_compiler::RuntimeSourceCompiler::new(compiler);
    }
}
