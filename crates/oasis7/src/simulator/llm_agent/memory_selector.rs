use super::super::memory::{AgentMemory, LongTermMemoryEntry, MemoryEntry, MemoryEntryKind};
use super::super::types::WorldTime;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemorySelectorConfig {
    pub short_term_candidate_limit: usize,
    pub long_term_candidate_limit: usize,
    pub short_term_top_k: usize,
    pub long_term_top_k: usize,
}

impl Default for MemorySelectorConfig {
    fn default() -> Self {
        Self {
            short_term_candidate_limit: 12,
            long_term_candidate_limit: 20,
            short_term_top_k: 4,
            long_term_top_k: 6,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemorySelectorResult {
    pub digest: String,
    pub candidates_total: usize,
    pub selected_total: usize,
}

#[derive(Debug, Clone)]
struct ScoredMemory {
    source: MemorySource,
    score: f64,
    timestamp: WorldTime,
    content: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MemorySource {
    ShortTerm,
    LongTerm,
}

pub struct MemorySelector;

impl MemorySelector {
    pub fn select(
        memory: &AgentMemory,
        now: WorldTime,
        config: &MemorySelectorConfig,
    ) -> MemorySelectorResult {
        let short_term_candidates = collect_short_term_candidates(memory, now, config);
        let long_term_candidates = collect_long_term_candidates(memory, now, config);

        let candidates_total = short_term_candidates.len() + long_term_candidates.len();

        let mut selected = Vec::new();
        selected.extend(top_k(short_term_candidates, config.short_term_top_k));
        selected.extend(top_k(long_term_candidates, config.long_term_top_k));

        selected.sort_by(|left, right| {
            right
                .score
                .partial_cmp(&left.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let mut dedup = std::collections::HashSet::new();
        selected.retain(|item| dedup.insert(normalize_content_key(item.content.as_str())));

        let selected_total = selected.len();
        let digest = if selected.is_empty() {
            "No relevant memory selected.".to_string()
        } else {
            selected
                .iter()
                .map(|item| {
                    let source = match item.source {
                        MemorySource::ShortTerm => "ST",
                        MemorySource::LongTerm => "LT",
                    };
                    format!(
                        "[{}][T{}][score={:.2}] {}",
                        source, item.timestamp, item.score, item.content
                    )
                })
                .collect::<Vec<_>>()
                .join("\n")
        };

        MemorySelectorResult {
            digest,
            candidates_total,
            selected_total,
        }
    }
}

fn collect_short_term_candidates(
    memory: &AgentMemory,
    now: WorldTime,
    config: &MemorySelectorConfig,
) -> Vec<ScoredMemory> {
    memory
        .short_term
        .recent(config.short_term_candidate_limit)
        .map(|entry| {
            let recency = recency_score(now, entry.time);
            let failure_bonus = match entry.kind {
                MemoryEntryKind::ActionResult { success: false, .. } => 0.10,
                _ => 0.0,
            };
            let score = 0.65 * entry.importance + 0.35 * recency + failure_bonus;
            ScoredMemory {
                source: MemorySource::ShortTerm,
                score,
                timestamp: entry.time,
                content: short_term_entry_to_text(entry),
            }
        })
        .collect()
}

fn collect_long_term_candidates(
    memory: &AgentMemory,
    now: WorldTime,
    config: &MemorySelectorConfig,
) -> Vec<ScoredMemory> {
    memory
        .long_term
        .top_by_importance(config.long_term_candidate_limit)
        .into_iter()
        .map(|entry| {
            let recency = recency_score(now, entry.created_at);
            let score = 0.70 * entry.importance + 0.30 * recency;
            ScoredMemory {
                source: MemorySource::LongTerm,
                score,
                timestamp: entry.created_at,
                content: long_term_entry_to_text(entry),
            }
        })
        .collect()
}

fn top_k(mut items: Vec<ScoredMemory>, top_k: usize) -> Vec<ScoredMemory> {
    items.sort_by(|left, right| {
        right
            .score
            .partial_cmp(&left.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    items.truncate(top_k);
    items
}

fn short_term_entry_to_text(entry: &MemoryEntry) -> String {
    match &entry.kind {
        MemoryEntryKind::Observation { summary } => format!("Observed: {}", summary),
        MemoryEntryKind::Decision { decision } => format!("Decision: {:?}", decision),
        MemoryEntryKind::ActionResult { action, success } => {
            format!("Action {:?} success={}", action, success)
        }
        MemoryEntryKind::Event { description } => format!("Event: {}", description),
        MemoryEntryKind::Note { content } => format!("Note: {}", content),
    }
}

fn long_term_entry_to_text(entry: &LongTermMemoryEntry) -> String {
    entry.content.clone()
}

fn normalize_content_key(content: &str) -> String {
    let normalized = content.trim().to_lowercase();
    for prefix in ["note: ", "observed: ", "event: ", "decision: ", "action "] {
        if let Some(rest) = normalized.strip_prefix(prefix) {
            return rest.trim().to_string();
        }
    }
    normalized
}

fn recency_score(now: WorldTime, timestamp: WorldTime) -> f64 {
    let delta = now.saturating_sub(timestamp) as f64;
    1.0 / (1.0 + delta)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::simulator::{Action, AgentDecision};

    #[test]
    fn memory_selector_respects_top_k_limits() {
        let mut memory = AgentMemory::with_capacities(32, 32);
        for index in 0..10 {
            memory
                .short_term
                .add(MemoryEntry::note(index, format!("note-{index}")).with_importance(0.6));
            memory.long_term.store_with_tags(
                format!("long-{index}"),
                index,
                vec!["tag".to_string()],
            );
        }

        let config = MemorySelectorConfig {
            short_term_candidate_limit: 10,
            long_term_candidate_limit: 10,
            short_term_top_k: 2,
            long_term_top_k: 3,
        };

        let result = MemorySelector::select(&memory, 10, &config);
        assert!(result.selected_total <= 5);
        assert!(result.candidates_total >= result.selected_total);
    }

    #[test]
    fn memory_selector_deduplicates_same_content() {
        let mut memory = AgentMemory::with_capacities(16, 16);
        memory
            .short_term
            .add(MemoryEntry::note(5, "same-content").with_importance(0.9));
        memory
            .long_term
            .store_with_tags("same-content", 4, vec!["dup".to_string()]);

        let result = MemorySelector::select(&memory, 6, &MemorySelectorConfig::default());
        let matches = result.digest.matches("same-content").count();
        assert_eq!(matches, 1);
    }

    #[test]
    fn memory_selector_scores_failed_actions_with_bonus() {
        let mut memory = AgentMemory::with_capacities(16, 16);
        memory.short_term.add(
            MemoryEntry::action_result(
                10,
                Action::MoveAgent {
                    agent_id: "agent-1".to_string(),
                    to: "loc-a".to_string(),
                },
                false,
            )
            .with_importance(0.4),
        );
        memory
            .short_term
            .add(MemoryEntry::decision(9, AgentDecision::Wait).with_importance(0.7));

        let config = MemorySelectorConfig {
            short_term_candidate_limit: 8,
            long_term_candidate_limit: 0,
            short_term_top_k: 1,
            long_term_top_k: 0,
        };
        let result = MemorySelector::select(&memory, 10, &config);
        assert!(result.digest.contains("Action"));
    }
}
