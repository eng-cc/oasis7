use std::{
    collections::VecDeque,
    io::Read,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};
const STDERR_LIMIT: usize = 16 * 1024;
/// A bounded tail of the real worker stderr. The pipe is continuously drained
/// so a verbose worker cannot block on a full pipe, while retained diagnostics
/// remain bounded for the local operation result.
#[derive(Clone)]
pub(super) struct WorkerDiagnostics {
    tail: Arc<Mutex<VecDeque<u8>>>,
    exited: Arc<AtomicBool>,
}

impl WorkerDiagnostics {
    pub(super) fn drain(stderr: impl Read + Send + 'static) -> Result<Self, String> {
        let tail = Arc::new(Mutex::new(VecDeque::with_capacity(STDERR_LIMIT)));
        let exited = Arc::new(AtomicBool::new(false));
        let reader_tail = Arc::clone(&tail);
        let reader_exited = Arc::clone(&exited);
        std::thread::Builder::new()
            .name("formal-owner-stderr".into())
            .spawn(move || {
                let mut stderr = stderr;
                let mut chunk = [0u8; 2048];
                loop {
                    match stderr.read(&mut chunk) {
                        Ok(0) | Err(_) => break,
                        Ok(count) => {
                            let Ok(mut tail) = reader_tail.lock() else {
                                break;
                            };
                            for byte in &chunk[..count] {
                                if tail.len() == STDERR_LIMIT {
                                    tail.pop_front();
                                }
                                tail.push_back(*byte);
                            }
                        }
                    }
                }
                reader_exited.store(true, Ordering::Release);
            })
            .map_err(|error| {
                format!(
                    "ORIGINAL_OUTCOME_UNKNOWN: owner worker started but stderr reader could not start: {error}"
                )
            })?;
        Ok(Self { tail, exited })
    }

    fn stderr_closed(&self) -> bool {
        self.exited.load(Ordering::Acquire)
    }

    pub(super) fn wait_for_stderr_close(&self, timeout: Duration) {
        let deadline = Instant::now() + timeout;
        while !self.stderr_closed() && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    fn text(&self) -> Option<String> {
        let mut tail = self.tail.lock().ok()?;
        if tail.is_empty() {
            return None;
        }
        let text = String::from_utf8_lossy(tail.make_contiguous());
        let diagnostic: String = text
            .chars()
            .filter(|character| !character.is_control() || matches!(character, '\n' | '\t' | '\r'))
            .take(2048)
            .collect();
        (!diagnostic.trim().is_empty()).then_some(diagnostic)
    }
}

pub(super) fn append_worker_diagnostic(message: &str, diagnostics: &WorkerDiagnostics) -> String {
    match diagnostics.text() {
        Some(stderr) => format!("{message}; worker stderr: {stderr}"),
        None => message.to_owned(),
    }
}
