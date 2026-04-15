use bevy::prelude::Resource;
use oasis7::simulator::WorldEvent;

const EVENT_WINDOW_SIZE_ENV: &str = "OASIS7_VIEWER_EVENT_WINDOW_SIZE";
const EVENT_WINDOW_RECENT_ENV: &str = "OASIS7_VIEWER_EVENT_WINDOW_RECENT";
const EVENT_WINDOW_SAMPLE_STRIDE_ENV: &str = "OASIS7_VIEWER_EVENT_SAMPLE_STRIDE";
const DEFAULT_SAMPLE_STRIDE: usize = 4;

#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct EventWindowPolicy {
    pub max_events: usize,
    pub recent_full_events: usize,
    pub sample_stride: usize,
}

impl EventWindowPolicy {
    pub(crate) fn new(max_events: usize, recent_full_events: usize, sample_stride: usize) -> Self {
        let max_events = max_events.max(1);
        let recent_full_events = recent_full_events.clamp(1, max_events);
        let sample_stride = sample_stride.max(1);
        Self {
            max_events,
            recent_full_events,
            sample_stride,
        }
    }
}

pub(crate) fn event_window_policy_from_env(default_max_events: usize) -> EventWindowPolicy {
    event_window_policy_from_values(
        crate::viewer_env::viewer_env_var(EVENT_WINDOW_SIZE_ENV),
        crate::viewer_env::viewer_env_var(EVENT_WINDOW_RECENT_ENV),
        crate::viewer_env::viewer_env_var(EVENT_WINDOW_SAMPLE_STRIDE_ENV),
        default_max_events,
    )
}

fn event_window_policy_from_values(
    max_events_raw: Option<String>,
    recent_events_raw: Option<String>,
    sample_stride_raw: Option<String>,
    default_max_events: usize,
) -> EventWindowPolicy {
    let max_events = parse_positive_usize(max_events_raw).unwrap_or(default_max_events.max(1));
    let recent_full_events = parse_positive_usize(recent_events_raw)
        .unwrap_or_else(|| default_recent_full_events(max_events));
    let sample_stride = parse_positive_usize(sample_stride_raw).unwrap_or(DEFAULT_SAMPLE_STRIDE);
    EventWindowPolicy::new(max_events, recent_full_events, sample_stride)
}

fn default_recent_full_events(max_events: usize) -> usize {
    (max_events.saturating_mul(3) / 5).clamp(1, max_events)
}

fn parse_positive_usize(raw: Option<String>) -> Option<usize> {
    raw.and_then(|value| value.trim().parse::<usize>().ok())
        .filter(|value| *value > 0)
}

pub(crate) fn push_event_with_window(
    events: &mut Vec<WorldEvent>,
    event: WorldEvent,
    policy: EventWindowPolicy,
) {
    events.push(event);
    compact_event_window(events, policy);
}

fn compact_event_window(events: &mut Vec<WorldEvent>, policy: EventWindowPolicy) {
    if events.len() <= policy.max_events {
        return;
    }

    let split_idx = events.len().saturating_sub(policy.recent_full_events);
    if split_idx == 0 {
        let overflow = events.len() - policy.max_events;
        events.drain(0..overflow);
        return;
    }

    let mut compacted = Vec::with_capacity(policy.max_events);
    for (idx, event) in events.drain(0..split_idx).enumerate() {
        if idx % policy.sample_stride == 0 {
            compacted.push(event);
        }
    }
    compacted.append(events);

    if compacted.len() > policy.max_events {
        let overflow = compacted.len() - policy.max_events;
        compacted.drain(0..overflow);
    }
    *events = compacted;
}

#[cfg(test)]
mod tests {
    use super::*;
    use oasis7::simulator::{RejectReason, WorldEventKind};
    use std::{hint::black_box, time::Instant};

    #[test]
    fn event_window_policy_normalizes_values() {
        let policy = EventWindowPolicy::new(0, 0, 0);
        assert_eq!(policy.max_events, 1);
        assert_eq!(policy.recent_full_events, 1);
        assert_eq!(policy.sample_stride, 1);

        let policy = EventWindowPolicy::new(12, 20, 0);
        assert_eq!(policy.max_events, 12);
        assert_eq!(policy.recent_full_events, 12);
        assert_eq!(policy.sample_stride, 1);
    }

    #[test]
    fn event_window_policy_from_values_uses_defaults_for_invalid_input() {
        let policy = event_window_policy_from_values(
            Some("not-a-number".to_string()),
            Some("-3".to_string()),
            Some("0".to_string()),
            10,
        );
        assert_eq!(policy.max_events, 10);
        assert_eq!(policy.recent_full_events, 6);
        assert_eq!(policy.sample_stride, DEFAULT_SAMPLE_STRIDE);
    }

    #[test]
    fn push_event_with_window_samples_old_and_keeps_recent_dense_tail() {
        let policy = EventWindowPolicy::new(6, 3, 2);
        let mut events = Vec::new();

        for id in 1..=8_u64 {
            push_event_with_window(
                &mut events,
                WorldEvent {
                    id,
                    time: id,
                    kind: WorldEventKind::ActionRejected {
                        reason: RejectReason::InvalidAmount { amount: id as i64 },
                    },
                    runtime_event: None,
                },
                policy,
            );
        }

        let ids: Vec<u64> = events.iter().map(|event| event.id).collect();
        assert_eq!(ids, vec![1, 3, 5, 6, 7, 8]);
    }

    #[test]
    fn push_event_with_window_degrades_to_plain_rolling_window_when_stride_is_one() {
        let policy = EventWindowPolicy::new(4, 4, 1);
        let mut events = Vec::new();

        for id in 1..=6_u64 {
            push_event_with_window(
                &mut events,
                WorldEvent {
                    id,
                    time: id,
                    kind: WorldEventKind::ActionRejected {
                        reason: RejectReason::InvalidAmount { amount: id as i64 },
                    },
                    runtime_event: None,
                },
                policy,
            );
        }

        let ids: Vec<u64> = events.iter().map(|event| event.id).collect();
        assert_eq!(ids, vec![3, 4, 5, 6]);
    }

    #[test]
    #[ignore = "perf harness"]
    fn perf_push_event_with_window_after_capacity() {
        let policy = EventWindowPolicy::new(512, 320, 4);
        let total_events = 40_000_u64;
        let mut events = Vec::new();
        let started_at = Instant::now();

        for id in 1..=total_events {
            push_event_with_window(
                &mut events,
                WorldEvent {
                    id,
                    time: id,
                    kind: WorldEventKind::ActionRejected {
                        reason: RejectReason::InvalidAmount { amount: id as i64 },
                    },
                    runtime_event: None,
                },
                policy,
            );
        }

        let elapsed = started_at.elapsed();
        println!(
            "perf event_window total_ms={:.2} avg_us_per_push={:.3} final_len={} checksum={}",
            elapsed.as_secs_f64() * 1000.0,
            elapsed.as_secs_f64() * 1_000_000.0 / total_events as f64,
            events.len(),
            black_box(events.iter().map(|event| event.id as usize).sum::<usize>()),
        );

        assert!(events.len() <= policy.max_events);
    }
}
