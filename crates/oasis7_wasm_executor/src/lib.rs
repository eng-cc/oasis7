//! Sandbox execution scaffolding for WASM modules.

#[cfg(feature = "wasmtime")]
use oasis7_wasm_abi::ModuleLimits;
use oasis7_wasm_abi::{
    ModuleCallErrorCode, ModuleCallFailure, ModuleCallRequest, ModuleOutput, ModuleSandbox,
};
#[cfg(feature = "wasmtime")]
use std::collections::{BTreeMap, VecDeque};
use std::fmt;
#[cfg(feature = "wasmtime")]
use std::fs;
use std::path::PathBuf;
#[cfg(feature = "wasmtime")]
use std::sync::{mpsc, Arc, Mutex};

fn count_exceeds_limit(count: usize, limit: u32) -> bool {
    match u32::try_from(count) {
        Ok(value) => value > limit,
        Err(_) => true,
    }
}

/// A sandbox stub that always returns a fixed result.
#[derive(Debug, Clone)]
pub struct FixedSandbox {
    result: Result<ModuleOutput, ModuleCallFailure>,
}

impl FixedSandbox {
    pub fn succeed(output: ModuleOutput) -> Self {
        Self { result: Ok(output) }
    }

    pub fn fail(failure: ModuleCallFailure) -> Self {
        Self {
            result: Err(failure),
        }
    }
}

impl ModuleSandbox for FixedSandbox {
    fn call(&mut self, _request: &ModuleCallRequest) -> Result<ModuleOutput, ModuleCallFailure> {
        self.result.clone()
    }
}

/// Configuration for a real WASM executor backend.
#[derive(Debug, Clone, PartialEq)]
pub struct WasmExecutorConfig {
    pub engine: WasmEngineKind,
    pub max_fuel: u64,
    pub max_mem_bytes: u64,
    pub max_output_bytes: u64,
    pub max_call_ms: u64,
    pub max_cache_entries: usize,
    pub compiled_cache_dir: Option<PathBuf>,
}

/// Selected WASM engine backend.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WasmEngineKind {
    Wasmtime,
}

impl Default for WasmExecutorConfig {
    fn default() -> Self {
        Self {
            engine: WasmEngineKind::Wasmtime,
            max_fuel: 10_000_000,
            max_mem_bytes: 64 * 1024 * 1024,
            max_output_bytes: 4 * 1024 * 1024,
            max_call_ms: 2_000,
            max_cache_entries: 32,
            compiled_cache_dir: None,
        }
    }
}

/// Placeholder WASM executor implementation.
#[derive(Clone)]
pub struct WasmExecutor {
    config: WasmExecutorConfig,
    #[cfg(feature = "wasmtime")]
    engine: wasmtime::Engine,
    #[cfg(feature = "wasmtime")]
    compiled_cache: Arc<Mutex<CompiledModuleCache>>,
    #[cfg(feature = "wasmtime")]
    compiled_disk_cache: Option<Arc<DiskCompiledModuleCache>>,
}

#[cfg(feature = "wasmtime")]
struct WasmStoreState {
    limits: wasmtime::StoreLimits,
}

#[cfg(feature = "wasmtime")]
struct EpochWatchdog {
    stop_tx: Option<mpsc::Sender<()>>,
    join_handle: Option<std::thread::JoinHandle<()>>,
}

#[cfg(feature = "wasmtime")]
impl EpochWatchdog {
    fn start(engine: &wasmtime::Engine, timeout_ms: u64) -> Self {
        let (stop_tx, stop_rx) = mpsc::channel::<()>();
        let engine = engine.clone();
        let timeout = std::time::Duration::from_millis(timeout_ms.max(1));
        let join_handle = std::thread::spawn(move || {
            if stop_rx.recv_timeout(timeout).is_err() {
                engine.increment_epoch();
            }
        });
        Self {
            stop_tx: Some(stop_tx),
            join_handle: Some(join_handle),
        }
    }
}

#[cfg(feature = "wasmtime")]
impl Drop for EpochWatchdog {
    fn drop(&mut self) {
        if let Some(stop_tx) = self.stop_tx.take() {
            let _ = stop_tx.send(());
        }
        if let Some(join_handle) = self.join_handle.take() {
            let _ = join_handle.join();
        }
    }
}

impl fmt::Debug for WasmExecutor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WasmExecutor")
            .field("config", &self.config)
            .finish()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WasmExecutorInitError {
    EngineInit(String),
    DiskCacheInit(String),
}

impl fmt::Display for WasmExecutorInitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EngineInit(detail) => write!(f, "failed to initialize wasmtime engine: {detail}"),
            Self::DiskCacheInit(detail) => {
                write!(
                    f,
                    "failed to initialize wasmtime compiled disk cache: {detail}"
                )
            }
        }
    }
}

impl std::error::Error for WasmExecutorInitError {}

impl WasmExecutor {
    pub fn new(config: WasmExecutorConfig) -> Result<Self, WasmExecutorInitError> {
        #[cfg(feature = "wasmtime")]
        {
            let mut engine_config = wasmtime::Config::new();
            engine_config.consume_fuel(true);
            engine_config.epoch_interruption(true);
            engine_config.wasm_multi_value(true);
            engine_config.wasm_reference_types(true);
            engine_config.wasm_threads(false);
            engine_config.cranelift_nan_canonicalization(true);
            engine_config.debug_info(false);
            let engine = wasmtime::Engine::new(&engine_config)
                .map_err(|err| WasmExecutorInitError::EngineInit(err.to_string()))?;
            let compiled_cache = Arc::new(Mutex::new(CompiledModuleCache::new(
                config.max_cache_entries,
            )));
            let compiled_disk_cache = config
                .compiled_cache_dir
                .clone()
                .map(|root| {
                    let fingerprint = compiled_engine_fingerprint(&config);
                    DiskCompiledModuleCache::new(root, fingerprint)
                })
                .transpose()
                .map_err(|err| WasmExecutorInitError::DiskCacheInit(err.to_string()))?;
            Ok(Self {
                config,
                engine,
                compiled_cache,
                compiled_disk_cache: compiled_disk_cache.map(Arc::new),
            })
        }

        #[cfg(not(feature = "wasmtime"))]
        {
            Ok(Self { config })
        }
    }

    pub fn config(&self) -> &WasmExecutorConfig {
        &self.config
    }

    #[cfg(feature = "wasmtime")]
    pub fn engine(&self) -> &wasmtime::Engine {
        &self.engine
    }

    #[cfg(all(feature = "wasmtime", test))]
    pub(crate) fn compiled_cache_len(&self) -> usize {
        self.compiled_cache
            .lock()
            .expect("compiled cache poisoned")
            .len()
    }

    #[cfg(feature = "wasmtime")]
    pub(crate) fn compile_module_cached(
        &self,
        wasm_hash: &str,
        wasm_bytes: &[u8],
    ) -> Result<Arc<wasmtime::Module>, ModuleCallFailure> {
        let mut cache = self.compiled_cache.lock().expect("compiled cache poisoned");
        if let Some(module) = cache.get(wasm_hash) {
            return Ok(module);
        }
        drop(cache);

        if let Some(module) = self.load_compiled_module_from_disk(wasm_hash) {
            let mut cache = self.compiled_cache.lock().expect("compiled cache poisoned");
            cache.insert(wasm_hash.to_string(), module.clone());
            return Ok(module);
        }

        let module = wasmtime::Module::new(&self.engine, wasm_bytes).map_err(|err| {
            self.failure(
                &ModuleCallRequest {
                    module_id: wasm_hash.to_string(),
                    wasm_hash: wasm_hash.to_string(),
                    trace_id: "compile".to_string(),
                    entrypoint: "call".to_string(),
                    input: Vec::new(),
                    limits: ModuleLimits::default(),
                    wasm_bytes: Vec::new(),
                },
                ModuleCallErrorCode::Trap,
                format!("compile failed: {err}"),
            )
        })?;
        let module = Arc::new(module);
        self.store_compiled_module_to_disk(wasm_hash, wasm_bytes);
        let mut cache = self.compiled_cache.lock().expect("compiled cache poisoned");
        cache.insert(wasm_hash.to_string(), module.clone());
        Ok(module)
    }

    #[cfg(feature = "wasmtime")]
    fn load_compiled_module_from_disk(&self, wasm_hash: &str) -> Option<Arc<wasmtime::Module>> {
        let disk_cache = self.compiled_disk_cache.as_ref()?;
        let path = disk_cache.module_path(wasm_hash);
        if !path.exists() {
            return None;
        }
        let bytes = match fs::read(&path) {
            Ok(bytes) => bytes,
            Err(_) => {
                let _ = fs::remove_file(&path);
                return None;
            }
        };
        match wasmtime::Module::new(&self.engine, &bytes) {
            Ok(module) => Some(Arc::new(module)),
            Err(_) => {
                let _ = fs::remove_file(&path);
                None
            }
        }
    }

    #[cfg(feature = "wasmtime")]
    fn store_compiled_module_to_disk(&self, wasm_hash: &str, wasm_bytes: &[u8]) {
        let Some(disk_cache) = self.compiled_disk_cache.as_ref() else {
            return;
        };
        let path = disk_cache.module_path(wasm_hash);
        let Some(parent) = path.parent() else {
            return;
        };
        if fs::create_dir_all(parent).is_err() {
            return;
        }
        let _ = fs::write(path, wasm_bytes);
    }

    #[cfg(all(feature = "wasmtime", test))]
    pub(crate) fn compiled_disk_cache_path_for_test(&self, wasm_hash: &str) -> Option<PathBuf> {
        self.compiled_disk_cache
            .as_ref()
            .map(|cache| cache.module_path(wasm_hash))
    }

    fn failure(
        &self,
        request: &ModuleCallRequest,
        code: ModuleCallErrorCode,
        detail: impl Into<String>,
    ) -> ModuleCallFailure {
        ModuleCallFailure {
            module_id: request.module_id.clone(),
            trace_id: request.trace_id.clone(),
            code,
            detail: detail.into(),
        }
    }

    #[cfg_attr(not(feature = "wasmtime"), allow(dead_code))]
    fn validate_request_limits(
        &self,
        request: &ModuleCallRequest,
    ) -> Result<(), ModuleCallFailure> {
        let limits = &request.limits;
        if limits.max_output_bytes > self.config.max_output_bytes {
            return Err(self.failure(
                request,
                ModuleCallErrorCode::OutputTooLarge,
                "requested output limit exceeds executor max",
            ));
        }
        if limits.max_gas > self.config.max_fuel {
            return Err(self.failure(
                request,
                ModuleCallErrorCode::Timeout,
                "requested fuel limit exceeds executor max",
            ));
        }
        if limits.max_mem_bytes > self.config.max_mem_bytes {
            return Err(self.failure(
                request,
                ModuleCallErrorCode::Trap,
                "requested memory limit exceeds executor max",
            ));
        }
        Ok(())
    }

    #[allow(dead_code)]
    fn validate_output_limits(
        &self,
        request: &ModuleCallRequest,
        output: &ModuleOutput,
    ) -> Result<(), ModuleCallFailure> {
        let limits = &request.limits;
        if count_exceeds_limit(output.effects.len(), limits.max_effects) {
            return Err(self.failure(
                request,
                ModuleCallErrorCode::EffectLimitExceeded,
                "effects exceeded",
            ));
        }
        if count_exceeds_limit(output.emits.len(), limits.max_emits) {
            return Err(self.failure(
                request,
                ModuleCallErrorCode::EmitLimitExceeded,
                "emits exceeded",
            ));
        }
        if output.output_bytes > limits.max_output_bytes {
            return Err(self.failure(
                request,
                ModuleCallErrorCode::OutputTooLarge,
                "output bytes exceeded",
            ));
        }
        Ok(())
    }

    #[cfg_attr(not(feature = "wasmtime"), allow(dead_code))]
    fn requested_fuel(&self, request: &ModuleCallRequest) -> u64 {
        if request.limits.max_gas == 0 {
            self.config.max_fuel
        } else {
            request.limits.max_gas
        }
    }

    #[cfg(feature = "wasmtime")]
    fn build_store_limits(&self, request: &ModuleCallRequest) -> wasmtime::StoreLimits {
        let memory_limit = usize::try_from(request.limits.max_mem_bytes).unwrap_or(usize::MAX);
        wasmtime::StoreLimitsBuilder::new()
            .memory_size(memory_limit)
            .trap_on_grow_failure(true)
            .build()
    }

    #[cfg(feature = "wasmtime")]
    fn map_wasmtime_error(
        &self,
        request: &ModuleCallRequest,
        err: wasmtime::Error,
    ) -> ModuleCallFailure {
        if let Some(trap) = err.downcast_ref::<wasmtime::Trap>() {
            let code = match trap {
                wasmtime::Trap::OutOfFuel => ModuleCallErrorCode::OutOfFuel,
                wasmtime::Trap::Interrupt => ModuleCallErrorCode::Interrupted,
                _ => ModuleCallErrorCode::Trap,
            };
            return self.failure(request, code, trap.to_string());
        }
        self.failure(request, ModuleCallErrorCode::Trap, err.to_string())
    }
}

impl ModuleSandbox for WasmExecutor {
    fn call(&mut self, request: &ModuleCallRequest) -> Result<ModuleOutput, ModuleCallFailure> {
        #[cfg(feature = "wasmtime")]
        {
            if let Err(failure) = self.validate_request_limits(request) {
                return Err(failure);
            }

            if request.wasm_bytes.is_empty() {
                return Err(self.failure(request, ModuleCallErrorCode::Trap, "missing wasm bytes"));
            }

            let module = self.compile_module_cached(&request.wasm_hash, &request.wasm_bytes)?;
            let start = std::time::Instant::now();
            let mut store = wasmtime::Store::new(
                &self.engine,
                WasmStoreState {
                    limits: self.build_store_limits(request),
                },
            );
            store.limiter(|state| &mut state.limits);
            store.epoch_deadline_trap();
            store.set_epoch_deadline(1);
            let _watchdog = EpochWatchdog::start(&self.engine, self.config.max_call_ms);
            store
                .set_fuel(self.requested_fuel(request))
                .map_err(|err| self.failure(request, ModuleCallErrorCode::Trap, err.to_string()))?;
            let linker = wasmtime::Linker::new(&self.engine);
            let instance = linker
                .instantiate(&mut store, &module)
                .map_err(|err| self.map_wasmtime_error(request, err))?;
            let memory = instance.get_memory(&mut store, "memory").ok_or_else(|| {
                self.failure(
                    request,
                    ModuleCallErrorCode::InvalidOutput,
                    "missing memory export",
                )
            })?;
            let alloc = instance
                .get_typed_func::<i32, i32>(&mut store, "alloc")
                .map_err(|err| {
                    self.failure(
                        request,
                        ModuleCallErrorCode::InvalidOutput,
                        format!("missing alloc export: {err}"),
                    )
                })?;
            enum EntrypointAbi {
                Multi(wasmtime::TypedFunc<(i32, i32), (i32, i32)>),
                PackedI64(wasmtime::TypedFunc<(i32, i32), i64>),
                SRet(wasmtime::TypedFunc<(i32, i32, i32), ()>),
            }
            let multi = instance
                .get_typed_func::<(i32, i32), (i32, i32)>(&mut store, request.entrypoint.as_str());
            let packed = if multi.is_err() {
                instance
                    .get_typed_func::<(i32, i32), i64>(&mut store, request.entrypoint.as_str())
                    .ok()
            } else {
                None
            };
            let sret = if multi.is_err() && packed.is_none() {
                instance
                    .get_typed_func::<(i32, i32, i32), ()>(&mut store, request.entrypoint.as_str())
                    .ok()
            } else {
                None
            };
            let call = if let Ok(func) = multi {
                EntrypointAbi::Multi(func)
            } else if let Some(func) = packed {
                EntrypointAbi::PackedI64(func)
            } else if let Some(func) = sret {
                EntrypointAbi::SRet(func)
            } else {
                let multi_err = instance
                    .get_typed_func::<(i32, i32), (i32, i32)>(
                        &mut store,
                        request.entrypoint.as_str(),
                    )
                    .err()
                    .map(|err| err.to_string())
                    .unwrap_or_else(|| "unavailable".to_string());
                let i64_err = instance
                    .get_typed_func::<(i32, i32), i64>(&mut store, request.entrypoint.as_str())
                    .err()
                    .map(|err| err.to_string())
                    .unwrap_or_else(|| "unavailable".to_string());
                let sret_err = instance
                    .get_typed_func::<(i32, i32, i32), ()>(&mut store, request.entrypoint.as_str())
                    .err()
                    .map(|err| err.to_string())
                    .unwrap_or_else(|| "unavailable".to_string());
                return Err(self.failure(
                    request,
                    ModuleCallErrorCode::InvalidOutput,
                    format!(
                        "missing {} export: multi-value `{multi_err}`; i64 `{i64_err}`; sret `{sret_err}`",
                        request.entrypoint
                    ),
                ));
            };

            let input_len = i32::try_from(request.input.len()).map_err(|_| {
                self.failure(
                    request,
                    ModuleCallErrorCode::Trap,
                    "input too large for wasm32",
                )
            })?;
            let input_ptr = alloc
                .call(&mut store, input_len)
                .map_err(|err| self.map_wasmtime_error(request, err))?;
            if input_ptr < 0 {
                return Err(self.failure(
                    request,
                    ModuleCallErrorCode::InvalidOutput,
                    "alloc returned negative pointer",
                ));
            }
            if input_len > 0 {
                const WASM_PAGE_SIZE: u64 = 65_536;
                let current_pages = memory.size(&store) as u64;
                let current_size = current_pages.saturating_mul(WASM_PAGE_SIZE);
                let needed_size = (input_ptr as u64).saturating_add(input_len as u64);
                if needed_size > current_size {
                    let required_pages = (needed_size + WASM_PAGE_SIZE - 1) / WASM_PAGE_SIZE;
                    let delta = required_pages.saturating_sub(current_pages);
                    if delta > 0 {
                        memory.grow(&mut store, delta).map_err(|err| {
                            self.failure(
                                request,
                                ModuleCallErrorCode::Trap,
                                format!("memory grow failed: {err}"),
                            )
                        })?;
                    }
                }
            }
            if input_len > 0 {
                memory
                    .write(&mut store, input_ptr as usize, &request.input)
                    .map_err(|err| {
                        self.failure(request, ModuleCallErrorCode::Trap, err.to_string())
                    })?;
            }
            let (output_ptr, output_len) = match call {
                EntrypointAbi::Multi(func) => func
                    .call(&mut store, (input_ptr, input_len))
                    .map_err(|err| self.map_wasmtime_error(request, err))?,
                EntrypointAbi::PackedI64(func) => {
                    let packed = func
                        .call(&mut store, (input_ptr, input_len))
                        .map_err(|err| self.map_wasmtime_error(request, err))?
                        as u64;
                    (
                        (packed & 0xffff_ffff) as u32 as i32,
                        ((packed >> 32) & 0xffff_ffff) as u32 as i32,
                    )
                }
                EntrypointAbi::SRet(func) => {
                    let out_pair_ptr = alloc
                        .call(&mut store, 8)
                        .map_err(|err| self.map_wasmtime_error(request, err))?;
                    if out_pair_ptr < 0 {
                        return Err(self.failure(
                            request,
                            ModuleCallErrorCode::InvalidOutput,
                            "alloc returned negative output pair pointer",
                        ));
                    }
                    func.call(&mut store, (out_pair_ptr, input_ptr, input_len))
                        .map_err(|err| self.map_wasmtime_error(request, err))?;
                    let mut pair = [0_u8; 8];
                    memory
                        .read(&mut store, out_pair_ptr as usize, &mut pair)
                        .map_err(|err| {
                            self.failure(request, ModuleCallErrorCode::Trap, err.to_string())
                        })?;
                    let output_ptr = i32::from_le_bytes([pair[0], pair[1], pair[2], pair[3]]);
                    let output_len = i32::from_le_bytes([pair[4], pair[5], pair[6], pair[7]]);
                    (output_ptr, output_len)
                }
            };
            let memory_size = memory.data_size(&store) as u64;
            if memory_size > request.limits.max_mem_bytes {
                return Err(self.failure(
                    request,
                    ModuleCallErrorCode::Trap,
                    "memory limit exceeded",
                ));
            }
            let output_len = usize::try_from(output_len).map_err(|_| {
                self.failure(request, ModuleCallErrorCode::Trap, "negative output length")
            })?;
            if output_len as u64 > request.limits.max_output_bytes {
                return Err(self.failure(
                    request,
                    ModuleCallErrorCode::OutputTooLarge,
                    "output bytes exceeded",
                ));
            }
            if output_ptr < 0 {
                return Err(self.failure(
                    request,
                    ModuleCallErrorCode::InvalidOutput,
                    "output pointer negative",
                ));
            }
            let memory_bytes = memory.data_size(&store) as u64;
            let output_end = (output_ptr as u64).saturating_add(output_len as u64);
            if output_end > memory_bytes {
                return Err(self.failure(
                    request,
                    ModuleCallErrorCode::Trap,
                    "output exceeds memory bounds",
                ));
            }
            let mut output_buf = vec![0u8; output_len];
            if output_len > 0 {
                memory
                    .read(&mut store, output_ptr as usize, &mut output_buf)
                    .map_err(|err| {
                        self.failure(request, ModuleCallErrorCode::Trap, err.to_string())
                    })?;
            }
            if start.elapsed().as_millis() as u64 > self.config.max_call_ms {
                return Err(self.failure(
                    request,
                    ModuleCallErrorCode::Timeout,
                    "execution exceeded max_call_ms",
                ));
            }
            let mut output: ModuleOutput = serde_cbor::from_slice(&output_buf).map_err(|err| {
                self.failure(
                    request,
                    ModuleCallErrorCode::InvalidOutput,
                    format!("output CBOR decode failed: {err}"),
                )
            })?;
            output.output_bytes = output_buf.len() as u64;
            self.validate_output_limits(request, &output)?;
            return Ok(output);
        }

        #[cfg(not(feature = "wasmtime"))]
        {
            let detail = "wasmtime feature not enabled";
            return Err(ModuleCallFailure {
                module_id: request.module_id.clone(),
                trace_id: request.trace_id.clone(),
                code: ModuleCallErrorCode::SandboxUnavailable,
                detail: detail.to_string(),
            });
        }
    }
}

#[cfg(feature = "wasmtime")]
fn compiled_engine_fingerprint(config: &WasmExecutorConfig) -> String {
    format!(
        "wasmtime-cf-v1-fuel{}-mem{}-out{}-call{}",
        config.max_fuel, config.max_mem_bytes, config.max_output_bytes, config.max_call_ms
    )
}

#[cfg(feature = "wasmtime")]
fn sanitize_cache_key(raw: &str) -> String {
    let mut key = String::with_capacity(raw.len());
    for ch in raw.chars() {
        if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
            key.push(ch);
        } else {
            key.push('_');
        }
    }
    if key.is_empty() {
        key.push_str("module");
    }
    key
}

#[cfg(feature = "wasmtime")]
#[derive(Debug, Clone)]
struct DiskCompiledModuleCache {
    root: PathBuf,
    engine_fingerprint: String,
}

#[cfg(feature = "wasmtime")]
impl DiskCompiledModuleCache {
    fn new(root: PathBuf, engine_fingerprint: String) -> std::io::Result<Self> {
        let cache = Self {
            root,
            engine_fingerprint,
        };
        fs::create_dir_all(cache.cache_dir())?;
        Ok(cache)
    }

    fn cache_dir(&self) -> PathBuf {
        self.root.join(&self.engine_fingerprint)
    }

    fn module_path(&self, wasm_hash: &str) -> PathBuf {
        let key = sanitize_cache_key(wasm_hash);
        self.cache_dir().join(format!("{key}.wasm"))
    }
}

#[cfg(feature = "wasmtime")]
#[derive(Debug)]
struct CompiledModuleCache {
    max_entries: usize,
    cache: BTreeMap<String, Arc<wasmtime::Module>>,
    lru: VecDeque<String>,
}

#[cfg(feature = "wasmtime")]
impl CompiledModuleCache {
    fn new(max_entries: usize) -> Self {
        Self {
            max_entries,
            cache: BTreeMap::new(),
            lru: VecDeque::new(),
        }
    }

    #[cfg(test)]
    fn len(&self) -> usize {
        self.cache.len()
    }

    fn get(&mut self, wasm_hash: &str) -> Option<Arc<wasmtime::Module>> {
        let module = self.cache.get(wasm_hash)?.clone();
        self.touch(wasm_hash);
        Some(module)
    }

    fn insert(&mut self, wasm_hash: String, module: Arc<wasmtime::Module>) {
        self.cache.insert(wasm_hash.clone(), module);
        self.touch(&wasm_hash);
        self.prune();
    }

    fn touch(&mut self, wasm_hash: &str) {
        self.lru.retain(|entry| entry != wasm_hash);
        self.lru.push_back(wasm_hash.to_string());
    }

    fn prune(&mut self) {
        if self.max_entries == 0 {
            self.cache.clear();
            self.lru.clear();
            return;
        }
        while self.cache.len() > self.max_entries {
            if let Some(evicted) = self.lru.pop_front() {
                self.cache.remove(&evicted);
            } else {
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oasis7_wasm_abi::ModuleLimits;
    #[cfg(feature = "wasmtime")]
    use std::fs;
    #[cfg(feature = "wasmtime")]
    use std::path::PathBuf;
    #[cfg(feature = "wasmtime")]
    use std::time::{SystemTime, UNIX_EPOCH};

    fn make_request(limits: ModuleLimits) -> ModuleCallRequest {
        ModuleCallRequest {
            module_id: "m.test".to_string(),
            wasm_hash: "hash".to_string(),
            trace_id: "trace-1".to_string(),
            entrypoint: "call".to_string(),
            input: vec![],
            limits,
            wasm_bytes: Vec::new(),
        }
    }

    fn test_executor(config: WasmExecutorConfig) -> WasmExecutor {
        WasmExecutor::new(config).expect("initialize wasm executor")
    }

    #[test]
    fn fixed_sandbox_succeed_returns_cloned_output() {
        let output = ModuleOutput {
            new_state: Some(vec![1, 2, 3]),
            effects: Vec::new(),
            emits: Vec::new(),
            tick_lifecycle: None,
            output_bytes: 3,
        };
        let mut sandbox = FixedSandbox::succeed(output.clone());
        let request = make_request(ModuleLimits::default());

        let first = sandbox.call(&request).unwrap();
        assert_eq!(first, output);

        let second = sandbox.call(&request).unwrap();
        assert_eq!(second, output);
    }

    #[test]
    fn fixed_sandbox_fail_returns_cloned_failure() {
        let failure = ModuleCallFailure {
            module_id: "m.test".to_string(),
            trace_id: "trace-err".to_string(),
            code: ModuleCallErrorCode::Trap,
            detail: "boom".to_string(),
        };
        let mut sandbox = FixedSandbox::fail(failure.clone());
        let request = make_request(ModuleLimits::default());

        let first = sandbox.call(&request).unwrap_err();
        assert_eq!(first, failure);

        let second = sandbox.call(&request).unwrap_err();
        assert_eq!(second, failure);
    }

    #[cfg(feature = "wasmtime")]
    #[test]
    fn wasm_executor_returns_disk_cache_init_error() {
        let cache_root = std::env::temp_dir().join("oasis7-wasm-init-error-file");
        fs::write(&cache_root, b"not-a-directory").expect("create temp cache file");

        let err = WasmExecutor::new(WasmExecutorConfig {
            compiled_cache_dir: Some(cache_root.clone()),
            ..WasmExecutorConfig::default()
        })
        .expect_err("file path should fail cache dir initialization");

        assert!(matches!(err, WasmExecutorInitError::DiskCacheInit(_)));

        let _ = fs::remove_file(cache_root);
    }

    #[test]
    fn wasm_executor_rejects_output_limit_overflow() {
        let executor = test_executor(WasmExecutorConfig::default());
        let request = make_request(ModuleLimits {
            max_mem_bytes: executor.config().max_mem_bytes,
            max_gas: executor.config().max_fuel,
            max_call_rate: 0,
            max_output_bytes: 4,
            max_effects: 0,
            max_emits: 0,
        });
        let output = ModuleOutput {
            new_state: None,
            effects: Vec::new(),
            emits: Vec::new(),
            tick_lifecycle: None,
            output_bytes: 8,
        };

        let err = executor
            .validate_output_limits(&request, &output)
            .unwrap_err();
        assert_eq!(err.code, ModuleCallErrorCode::OutputTooLarge);
    }

    #[test]
    fn count_exceeds_limit_treats_usize_overflow_as_exceeded() {
        assert!(count_exceeds_limit(usize::MAX, 1));
        assert!(!count_exceeds_limit(1, 1));
    }

    #[test]
    fn wasm_executor_rejects_fuel_limit_as_timeout() {
        let executor = test_executor(WasmExecutorConfig {
            max_fuel: 10,
            ..WasmExecutorConfig::default()
        });
        let request = make_request(ModuleLimits {
            max_mem_bytes: executor.config().max_mem_bytes,
            max_gas: 11,
            max_call_rate: 0,
            max_output_bytes: executor.config().max_output_bytes,
            max_effects: 0,
            max_emits: 0,
        });

        let err = executor.validate_request_limits(&request).unwrap_err();
        assert_eq!(err.code, ModuleCallErrorCode::Timeout);
    }

    #[test]
    fn wasm_executor_uses_executor_max_fuel_when_request_limit_is_zero() {
        let executor = test_executor(WasmExecutorConfig {
            max_fuel: 123,
            ..WasmExecutorConfig::default()
        });
        let request = make_request(ModuleLimits {
            max_mem_bytes: executor.config().max_mem_bytes,
            max_gas: 0,
            max_call_rate: 0,
            max_output_bytes: executor.config().max_output_bytes,
            max_effects: 0,
            max_emits: 0,
        });

        assert_eq!(executor.requested_fuel(&request), 123);
    }

    #[test]
    fn wasm_executor_rejects_memory_limit_overflow_as_trap() {
        let executor = test_executor(WasmExecutorConfig {
            max_mem_bytes: 64,
            ..WasmExecutorConfig::default()
        });
        let request = make_request(ModuleLimits {
            max_mem_bytes: 65,
            max_gas: executor.config().max_fuel,
            max_call_rate: 0,
            max_output_bytes: executor.config().max_output_bytes,
            max_effects: 0,
            max_emits: 0,
        });

        let err = executor.validate_request_limits(&request).unwrap_err();
        assert_eq!(err.code, ModuleCallErrorCode::Trap);
    }

    #[test]
    fn wasm_executor_rejects_requested_output_limit_over_executor_max() {
        let executor = test_executor(WasmExecutorConfig {
            max_output_bytes: 16,
            ..WasmExecutorConfig::default()
        });
        let request = make_request(ModuleLimits {
            max_mem_bytes: executor.config().max_mem_bytes,
            max_gas: executor.config().max_fuel,
            max_call_rate: 0,
            max_output_bytes: 17,
            max_effects: 0,
            max_emits: 0,
        });

        let err = executor.validate_request_limits(&request).unwrap_err();
        assert_eq!(err.code, ModuleCallErrorCode::OutputTooLarge);
    }

    #[cfg(feature = "wasmtime")]
    #[test]
    fn wasm_executor_maps_interrupt_trap_to_interrupted() {
        let executor = test_executor(WasmExecutorConfig::default());
        let request = make_request(ModuleLimits {
            max_mem_bytes: executor.config().max_mem_bytes,
            max_gas: executor.config().max_fuel,
            max_call_rate: 0,
            max_output_bytes: executor.config().max_output_bytes,
            max_effects: 0,
            max_emits: 0,
        });

        let err = executor.map_wasmtime_error(&request, wasmtime::Trap::Interrupt.into());
        assert_eq!(err.code, ModuleCallErrorCode::Interrupted);
    }

    #[cfg(feature = "wasmtime")]
    #[test]
    fn wasm_executor_maps_out_of_fuel_trap_to_out_of_fuel() {
        let executor = test_executor(WasmExecutorConfig::default());
        let request = make_request(ModuleLimits {
            max_mem_bytes: executor.config().max_mem_bytes,
            max_gas: executor.config().max_fuel,
            max_call_rate: 0,
            max_output_bytes: executor.config().max_output_bytes,
            max_effects: 0,
            max_emits: 0,
        });

        let err = executor.map_wasmtime_error(&request, wasmtime::Trap::OutOfFuel.into());
        assert_eq!(err.code, ModuleCallErrorCode::OutOfFuel);
    }

    #[cfg(feature = "wasmtime")]
    #[test]
    fn wasm_executor_store_limits_enforce_requested_memory_cap() {
        let executor = test_executor(WasmExecutorConfig::default());
        let request = make_request(ModuleLimits {
            max_mem_bytes: 64,
            max_gas: executor.config().max_fuel,
            max_call_rate: 0,
            max_output_bytes: executor.config().max_output_bytes,
            max_effects: 0,
            max_emits: 0,
        });
        let mut limits = executor.build_store_limits(&request);

        let allow = <wasmtime::StoreLimits as wasmtime::ResourceLimiter>::memory_growing(
            &mut limits,
            32,
            64,
            Some(128),
        )
        .expect("memory growth decision");
        assert!(allow);

        let deny = <wasmtime::StoreLimits as wasmtime::ResourceLimiter>::memory_growing(
            &mut limits,
            64,
            65,
            Some(128),
        );
        assert!(deny.is_err());
    }

    #[cfg(feature = "wasmtime")]
    #[test]
    fn wasm_executor_epoch_watchdog_preempts_infinite_loop() {
        let mut executor = test_executor(WasmExecutorConfig {
            max_call_ms: 20,
            max_fuel: u64::MAX,
            ..WasmExecutorConfig::default()
        });
        let wasm = wat::parse_str(
            r#"(module
                 (memory (export "memory") 1)
                 (func (export "alloc") (param i32) (result i32)
                   i32.const 0)
                 (func (export "call") (param i32 i32) (result i64)
                   (loop $l
                     br $l)
                   i64.const 0))"#,
        )
        .expect("compile test wat");
        let request = ModuleCallRequest {
            module_id: "m.loop".to_string(),
            wasm_hash: "hash-loop".to_string(),
            trace_id: "trace-loop".to_string(),
            entrypoint: "call".to_string(),
            input: Vec::new(),
            limits: ModuleLimits {
                max_mem_bytes: 64 * 1024,
                max_gas: 0,
                max_call_rate: 0,
                max_output_bytes: 1024,
                max_effects: 0,
                max_emits: 0,
            },
            wasm_bytes: wasm,
        };

        let started = std::time::Instant::now();
        let err = executor
            .call(&request)
            .expect_err("infinite loop should be interrupted by watchdog");
        assert_eq!(err.code, ModuleCallErrorCode::Interrupted);
        assert!(
            started.elapsed().as_millis() < 3_000,
            "watchdog timeout should preempt quickly"
        );
    }

    #[cfg(feature = "wasmtime")]
    #[test]
    fn wasm_executor_compiled_cache_evicts_old_entries() {
        let executor = test_executor(WasmExecutorConfig {
            max_cache_entries: 1,
            ..WasmExecutorConfig::default()
        });
        let wasm_a = [0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00];
        let wasm_b = [0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00];

        executor.compile_module_cached("hash-a", &wasm_a).unwrap();
        assert_eq!(executor.compiled_cache_len(), 1);

        executor.compile_module_cached("hash-b", &wasm_b).unwrap();
        assert_eq!(executor.compiled_cache_len(), 1);
    }

    #[cfg(feature = "wasmtime")]
    #[test]
    fn wasm_executor_compiled_cache_zero_capacity_stays_empty() {
        let executor = test_executor(WasmExecutorConfig {
            max_cache_entries: 0,
            ..WasmExecutorConfig::default()
        });
        let wasm = [0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00];

        executor.compile_module_cached("hash-a", &wasm).unwrap();
        assert_eq!(executor.compiled_cache_len(), 0);

        executor.compile_module_cached("hash-b", &wasm).unwrap();
        assert_eq!(executor.compiled_cache_len(), 0);
    }

    #[cfg(feature = "wasmtime")]
    fn unique_temp_cache_dir(suffix: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock drift")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("oasis7-wasm-cache-{suffix}-{nonce}"));
        fs::create_dir_all(&dir).expect("create temp cache dir");
        dir
    }

    #[cfg(feature = "wasmtime")]
    #[test]
    fn wasm_executor_disk_cache_hits_when_memory_cache_disabled() {
        let cache_dir = unique_temp_cache_dir("hit");
        let executor = test_executor(WasmExecutorConfig {
            max_cache_entries: 0,
            compiled_cache_dir: Some(cache_dir.clone()),
            ..WasmExecutorConfig::default()
        });
        let wasm = [0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00];
        let invalid_wasm = [0x01, 0x02, 0x03];

        executor
            .compile_module_cached("hash-disk-hit", &wasm)
            .unwrap();
        executor
            .compile_module_cached("hash-disk-hit", &invalid_wasm)
            .expect("load compiled module from disk cache");

        let _ = fs::remove_dir_all(cache_dir);
    }

    #[cfg(feature = "wasmtime")]
    #[test]
    fn wasm_executor_disk_cache_recovers_from_corruption() {
        let cache_dir = unique_temp_cache_dir("corrupt");
        let executor = test_executor(WasmExecutorConfig {
            max_cache_entries: 0,
            compiled_cache_dir: Some(cache_dir.clone()),
            ..WasmExecutorConfig::default()
        });
        let wasm = [0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00];
        let wasm_hash = "hash-disk-corrupt";

        executor.compile_module_cached(wasm_hash, &wasm).unwrap();
        let cache_file = executor
            .compiled_disk_cache_path_for_test(wasm_hash)
            .expect("cache path");
        fs::write(&cache_file, b"corrupt-bytes").expect("write corrupt cache");

        executor
            .compile_module_cached(wasm_hash, &wasm)
            .expect("recompile after corrupt cache");

        let repaired = fs::read(&cache_file).expect("read repaired cache");
        assert_ne!(repaired, b"corrupt-bytes");

        let _ = fs::remove_dir_all(cache_dir);
    }
}
