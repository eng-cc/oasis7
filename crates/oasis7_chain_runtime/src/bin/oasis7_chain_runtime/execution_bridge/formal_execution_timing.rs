//! Opt-in phase timings contain only execution positions and elapsed durations.
use std::time::Instant;

pub(super) struct FormalExecutionTiming {
    clocks: Option<(Instant, Instant)>,
    position: u64,
    operation: &'static str,
}
impl FormalExecutionTiming {
    pub(super) fn begin(operation: &'static str, position: u64) -> Self {
        let clocks = (std::env::var_os("OASIS7_FORMAL_EXECUTION_TIMING").as_deref()
            == Some(std::ffi::OsStr::new("1")))
        .then(|| {
            let now = Instant::now();
            eprintln!(
                "formal_execution_timing operation={operation} position={position} event=begin"
            );
            (now, now)
        });
        Self {
            clocks,
            position,
            operation,
        }
    }
    pub(super) fn mark(&mut self, phase: &'static str) {
        if let Some((start, last)) = &mut self.clocks {
            let now = Instant::now();
            eprintln!(
                "formal_execution_timing operation={} position={} phase={phase} stage_ms={} total_ms={}",
                self.operation,
                self.position,
                now.duration_since(*last).as_millis(),
                now.duration_since(*start).as_millis(),
            );
            *last = now;
        }
    }
}
impl Drop for FormalExecutionTiming {
    fn drop(&mut self) {
        self.mark("end");
    }
}
