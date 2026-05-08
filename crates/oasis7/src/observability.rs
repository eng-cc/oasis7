#[cfg(not(target_arch = "wasm32"))]
use std::sync::Once;

#[cfg(not(target_arch = "wasm32"))]
use tracing::Level;
#[cfg(not(target_arch = "wasm32"))]
use tracing_subscriber::EnvFilter;

#[cfg(not(target_arch = "wasm32"))]
static TRACING_INIT: Once = Once::new();

#[cfg(not(target_arch = "wasm32"))]
pub fn init_tracing(service_name: &str) {
    TRACING_INIT.call_once(|| {
        let filter = EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| EnvFilter::new(format!("{service_name}=info,oasis7=info,warn")));
        tracing_subscriber::fmt()
            .with_env_filter(filter)
            .with_target(true)
            .with_thread_names(true)
            .compact()
            .init();
    });
}

#[cfg(not(target_arch = "wasm32"))]
pub fn emit_stderr_or_event(level: Level, stderr_message: &str, event_message: &str) {
    if tracing::dispatcher::has_been_set() {
        match level {
            Level::ERROR => tracing::error!(message = %stderr_message, "{event_message}"),
            Level::WARN => tracing::warn!(message = %stderr_message, "{event_message}"),
            Level::INFO => tracing::info!(message = %stderr_message, "{event_message}"),
            Level::DEBUG => tracing::debug!(message = %stderr_message, "{event_message}"),
            Level::TRACE => tracing::trace!(message = %stderr_message, "{event_message}"),
        }
    } else {
        eprintln!("{stderr_message}");
    }
}

#[cfg(test)]
mod tests {
    use super::{emit_stderr_or_event, init_tracing};
    use tracing::Level;

    #[test]
    fn init_tracing_is_idempotent() {
        init_tracing("oasis7_test");
        init_tracing("oasis7_test");
    }

    #[test]
    fn emit_stderr_or_event_allows_tracing_path_after_init() {
        init_tracing("oasis7_test");
        emit_stderr_or_event(Level::WARN, "stderr warning", "structured warning");
    }
}
