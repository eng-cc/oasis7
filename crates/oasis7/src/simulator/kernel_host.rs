//! Existing host defaults for the kernel compiler capability.
//! This adapter stays with host assembly when the native kernel is extracted.
use super::source_compiler::KernelSourceCompiler;
use std::sync::Arc;

impl Default for KernelSourceCompiler {
    fn default() -> Self {
        #[cfg(not(target_arch = "wasm32"))]
        {
            Self::new(Arc::new(
                crate::module_source_compiler::HostModuleSourceCompiler,
            ))
        }
        #[cfg(target_arch = "wasm32")]
        {
            Self::new(Arc::new(UnavailableCompiler))
        }
    }
}
#[cfg(target_arch = "wasm32")]
struct UnavailableCompiler;
#[cfg(target_arch = "wasm32")]
impl oasis7_agent_api::module_source::ModuleSourceCompiler for UnavailableCompiler {
    fn compile(
        &self,
        _: &str,
        _: &oasis7_agent_api::world_action::ModuleSourcePackage,
    ) -> Result<Vec<u8>, oasis7_agent_api::module_source::ModuleSourceCompileError> {
        Err(
            oasis7_agent_api::module_source::ModuleSourceCompileError::Unavailable(
                "source compilation is unavailable on wasm32 target".into(),
            ),
        )
    }
}
