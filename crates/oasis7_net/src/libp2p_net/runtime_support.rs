pub(super) fn new_current_thread_runtime(label: &str) -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect(label)
}

#[cfg(test)]
pub(crate) fn run_on_libp2p_test_runtime<T>(f: impl FnOnce() -> T) -> T {
    let runtime = new_current_thread_runtime("build libp2p test tokio runtime");
    runtime.block_on(async move { f() })
}
