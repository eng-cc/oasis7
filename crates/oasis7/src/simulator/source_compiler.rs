//! Process-local compiler capability; never part of kernel persistence.
use oasis7_agent_api::module_source::ModuleSourceCompiler;
use oasis7_agent_api::world_action::ModuleSourcePackage;
use std::sync::Arc;

#[derive(Clone)]
pub(crate) struct KernelSourceCompiler(Arc<dyn ModuleSourceCompiler>);
impl KernelSourceCompiler {
    pub(crate) fn new(compiler: Arc<dyn ModuleSourceCompiler>) -> Self {
        Self(compiler)
    }
    pub(crate) fn compile(
        &self,
        module_id: &str,
        package: &ModuleSourcePackage,
    ) -> Result<Vec<u8>, String> {
        self.0
            .compile(module_id, package)
            .map_err(|error| error.to_string())
    }
}
impl std::fmt::Debug for KernelSourceCompiler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("KernelSourceCompiler { .. }")
    }
}
impl PartialEq for KernelSourceCompiler {
    fn eq(&self, _other: &Self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests;
