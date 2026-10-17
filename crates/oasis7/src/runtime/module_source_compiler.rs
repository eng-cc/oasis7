//! Compatibility adapter from the host compiler capability to Runtime errors.
use super::{ModuleSourcePackage, WorldError};
use oasis7_agent_api::module_source::{ModuleSourceCompileError, ModuleSourceCompiler};
use std::sync::Arc;

#[derive(Clone)]
pub(crate) struct RuntimeSourceCompiler(Arc<dyn ModuleSourceCompiler>);
impl std::fmt::Debug for RuntimeSourceCompiler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("RuntimeSourceCompiler { .. }")
    }
}
impl Default for RuntimeSourceCompiler {
    fn default() -> Self {
        // Existing host assembly; moved to the host when Runtime is extracted.
        Self(Arc::new(
            crate::module_source_compiler::HostModuleSourceCompiler,
        ))
    }
}
impl RuntimeSourceCompiler {
    pub(crate) fn new(compiler: Arc<dyn ModuleSourceCompiler>) -> Self {
        Self(compiler)
    }
    pub(crate) fn compile(
        &self,
        module_id: &str,
        package: &ModuleSourcePackage,
    ) -> Result<Vec<u8>, WorldError> {
        map_compile_result(self.0.compile(module_id, package))
    }
}

fn map_compile_result(
    result: Result<Vec<u8>, ModuleSourceCompileError>,
) -> Result<Vec<u8>, WorldError> {
    result.map_err(|error| match error {
        ModuleSourceCompileError::ModuleChangeInvalid { reason } => {
            WorldError::ModuleChangeInvalid { reason }
        }
        ModuleSourceCompileError::Io(message) => WorldError::Io(message),
        ModuleSourceCompileError::Unavailable(reason) => WorldError::ModuleChangeInvalid { reason },
    })
}
