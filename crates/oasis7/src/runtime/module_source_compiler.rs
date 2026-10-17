//! Compatibility adapter from the host compiler capability to Runtime errors.
use super::{ModuleSourcePackage, WorldError};
use oasis7_agent_api::module_source::ModuleSourceCompileError;

pub(crate) fn compile_module_artifact_from_source(
    module_id: &str,
    package: &ModuleSourcePackage,
) -> Result<Vec<u8>, WorldError> {
    crate::module_source_compiler::compile_module_artifact_from_source(module_id, package).map_err(
        |error| match error {
            ModuleSourceCompileError::ModuleChangeInvalid { reason } => {
                WorldError::ModuleChangeInvalid { reason }
            }
            ModuleSourceCompileError::Io(message) => WorldError::Io(message),
            ModuleSourceCompileError::Unavailable(reason) => {
                WorldError::ModuleChangeInvalid { reason }
            }
        },
    )
}
