use super::WorldServiceClientError;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// One application's bounded query budget. Clone across recreated transports;
/// independent applications have independent state, without a global map.
#[derive(Debug, Clone, Default)]
pub struct WorldServiceQueryState(Arc<Mutex<Option<(u16, Instant)>>>);

impl WorldServiceQueryState {
    pub(super) fn check(&self) -> Result<(), WorldServiceClientError> {
        let mut state = self.0.lock().map_err(|_| {
            WorldServiceClientError::Configuration("query cooldown lock poisoned".into())
        })?;
        if let Some((status, until)) = *state {
            if let Some(retry_after) = until.checked_duration_since(Instant::now()) {
                return Err(WorldServiceClientError::Cooldown {
                    status,
                    retry_after,
                });
            }
            *state = None;
        }
        Ok(())
    }

    pub(super) fn observe(
        &self,
        status: u16,
        retry_after: Option<&str>,
    ) -> Result<Option<Duration>, WorldServiceClientError> {
        if !matches!(status, 429 | 503) {
            return Ok(None);
        }
        let Some(header) = retry_after else {
            return Ok(None);
        };
        // Only delta-seconds are consumed. Invalid advice falls back to one
        // second; no response can impose an unbounded delay or zero-delay loop.
        let seconds = header.trim().parse::<u64>().unwrap_or(1).clamp(1, 2);
        let delay = Duration::from_secs(seconds);
        let mut state = self.0.lock().map_err(|_| {
            WorldServiceClientError::Configuration("query cooldown lock poisoned".into())
        })?;
        *state = Some((status, Instant::now() + delay));
        Ok(Some(delay))
    }

    #[cfg(test)]
    pub(super) fn expire_for_test(&self) {
        if let Some((_, until)) = self.0.lock().unwrap().as_mut() {
            *until = Instant::now() - Duration::from_secs(1);
        }
    }
}
