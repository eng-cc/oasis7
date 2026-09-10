pub(super) const RATE_LIMIT_RETRY_DELAY_MS: u64 = 250;
pub(super) const RATE_LIMIT_RETRY_ATTEMPTS: u32 = 1;

pub(super) fn retry_attempts(max_model_calls: Option<u32>) -> u32 {
    max_model_calls.map_or(RATE_LIMIT_RETRY_ATTEMPTS, |_| 0)
}

pub(super) fn is_concurrency_limit_error(raw_body: &str) -> bool {
    let lowered = raw_body.to_ascii_lowercase();
    lowered.contains("\"type\":\"rate_limit_error\"")
        && lowered.contains("concurrency limit exceeded")
}
