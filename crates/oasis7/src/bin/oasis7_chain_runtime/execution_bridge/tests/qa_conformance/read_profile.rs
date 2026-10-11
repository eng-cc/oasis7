//! Test-only read timings, transferred into failure-only fixture diagnostics.
//! Nested stages overlap; thread-local counters retain no request material.
use std::{cell::RefCell, time::Instant};

#[derive(Clone, Copy)]
pub(crate) enum Stage {
    PinWhole,
    PinDecodeProof,
    WorldRestore,
    ViewScope,
    ViewProjection,
    AgentProjection,
}
#[derive(Clone, Copy, Default)]
pub(crate) struct Sample {
    pub count: u64,
    pub total_us: u64,
    pub max_us: u64,
}
thread_local! { static SAMPLES: RefCell<[Sample; 6]> = const { RefCell::new([Sample { count: 0, total_us: 0, max_us: 0 }; 6]) }; }
pub(crate) fn reset() {
    SAMPLES.with(|samples| *samples.borrow_mut() = [Sample::default(); 6]);
}
pub(crate) fn take() -> [Sample; 6] {
    SAMPLES.with(|samples| std::mem::take(&mut *samples.borrow_mut()))
}
pub(crate) struct Timer {
    stage: Stage,
    start: Instant,
}
impl Timer {
    pub(crate) fn start(stage: Stage) -> Self {
        Self {
            stage,
            start: Instant::now(),
        }
    }
}
impl Drop for Timer {
    fn drop(&mut self) {
        let elapsed = self.start.elapsed().as_micros().min(u64::MAX as u128) as u64;
        SAMPLES.with(|samples| {
            let mut samples = samples.borrow_mut();
            let sample = &mut samples[self.stage as usize];
            sample.count += 1;
            sample.total_us += elapsed;
            sample.max_us = sample.max_us.max(elapsed);
        });
    }
}

#[test]
fn read_profile_transfer_consumes_only_completed_event_counts() {
    reset();
    let timer = Timer::start(Stage::WorldRestore);
    assert_eq!(take()[Stage::WorldRestore as usize].count, 0);
    drop(timer);
    let completed = take();
    assert_eq!(completed[Stage::WorldRestore as usize].count, 1);
    assert_eq!(take()[Stage::WorldRestore as usize].count, 0);
    let timer = Timer::start(Stage::PinWhole);
    drop(timer);
    reset();
    assert_eq!(take()[Stage::PinWhole as usize].count, 0);
}
