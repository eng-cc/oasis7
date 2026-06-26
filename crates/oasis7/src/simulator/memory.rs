//! Agent memory system: short-term and long-term memory.

use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::{BTreeMap, VecDeque};

use super::agent::AgentDecision;
use super::types::{Action, WorldTime};

// ============================================================================
// Memory Entry
// ============================================================================

/// Short-term memory entry: a recent observation with context.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MemoryEntry {
    /// The world time when this memory was created.
    pub time: WorldTime,
    /// The type of memory entry.
    pub kind: MemoryEntryKind,
    /// Optional importance score (0.0 to 1.0).
    pub importance: f64,
}

/// Types of memory entries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum MemoryEntryKind {
    /// An observation that was received.
    Observation { summary: String },
    /// A decision that was made.
    Decision { decision: AgentDecision },
    /// An action result.
    ActionResult { action: Action, success: bool },
    /// An external event that affected the agent.
    Event { description: String },
    /// A custom note or reflection.
    Note { content: String },
}

impl MemoryEntry {
    /// Create a new memory entry for an observation.
    pub fn observation(time: WorldTime, summary: impl Into<String>) -> Self {
        Self {
            time,
            kind: MemoryEntryKind::Observation {
                summary: summary.into(),
            },
            importance: 0.5,
        }
    }

    /// Create a new memory entry for a decision.
    pub fn decision(time: WorldTime, decision: AgentDecision) -> Self {
        Self {
            time,
            kind: MemoryEntryKind::Decision { decision },
            importance: 0.6,
        }
    }

    /// Create a new memory entry for an action result.
    pub fn action_result(time: WorldTime, action: Action, success: bool) -> Self {
        let importance = if success { 0.5 } else { 0.8 }; // Failed actions are more memorable
        Self {
            time,
            kind: MemoryEntryKind::ActionResult { action, success },
            importance,
        }
    }

    /// Create a new memory entry for an event.
    pub fn event(time: WorldTime, description: impl Into<String>) -> Self {
        Self {
            time,
            kind: MemoryEntryKind::Event {
                description: description.into(),
            },
            importance: 0.7,
        }
    }

    /// Create a new memory entry for a custom note.
    pub fn note(time: WorldTime, content: impl Into<String>) -> Self {
        Self {
            time,
            kind: MemoryEntryKind::Note {
                content: content.into(),
            },
            importance: 0.5,
        }
    }

    /// Set the importance score.
    pub fn with_importance(mut self, importance: f64) -> Self {
        self.importance = importance.clamp(0.0, 1.0);
        self
    }
}

// ============================================================================
// Short-Term Memory
// ============================================================================

/// Short-term memory buffer with configurable capacity.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ShortTermMemory {
    /// The memory entries in chronological order.
    entries: VecDeque<MemoryEntry>,
    /// Maximum number of entries to keep.
    capacity: usize,
    /// Total entries ever added (for statistics).
    total_added: u64,
}

impl Default for ShortTermMemory {
    fn default() -> Self {
        Self::new(100)
    }
}

impl ShortTermMemory {
    /// Create a new short-term memory with the given capacity.
    pub fn new(capacity: usize) -> Self {
        Self {
            entries: VecDeque::with_capacity(capacity),
            capacity,
            total_added: 0,
        }
    }

    /// Add a new memory entry.
    pub fn add(&mut self, entry: MemoryEntry) {
        self.total_added += 1;
        if self.entries.len() >= self.capacity {
            self.entries.pop_front();
        }
        self.entries.push_back(entry);
    }

    /// Get the most recent N entries.
    pub fn recent(&self, n: usize) -> impl Iterator<Item = &MemoryEntry> {
        self.entries.iter().rev().take(n)
    }

    /// Get all entries.
    pub fn all(&self) -> impl Iterator<Item = &MemoryEntry> {
        self.entries.iter()
    }

    /// Get entries since a given time.
    pub fn since(&self, time: WorldTime) -> impl Iterator<Item = &MemoryEntry> {
        self.entries.iter().filter(move |e| e.time >= time)
    }

    /// Get entries with importance above a threshold.
    pub fn important(&self, threshold: f64) -> impl Iterator<Item = &MemoryEntry> {
        self.entries
            .iter()
            .filter(move |e| e.importance >= threshold)
    }

    /// Get the number of entries in memory.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if the memory is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Get the capacity of the memory.
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Get the total number of entries ever added.
    pub fn total_added(&self) -> u64 {
        self.total_added
    }

    /// Clear all entries.
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// Create a summary of recent memory (for context injection).
    pub fn summarize(&self, max_entries: usize) -> String {
        let recent: Vec<_> = self.recent(max_entries).collect();
        if recent.is_empty() {
            return "No recent memories.".to_string();
        }

        let mut lines = Vec::new();
        for entry in recent.iter().rev() {
            let line = match &entry.kind {
                MemoryEntryKind::Observation { summary } => {
                    format!("[T{}] Observed: {}", entry.time, summary)
                }
                MemoryEntryKind::Decision { decision } => {
                    format!("[T{}] Decided: {:?}", entry.time, decision)
                }
                MemoryEntryKind::ActionResult { action, success } => {
                    let status = if *success { "succeeded" } else { "failed" };
                    format!("[T{}] Action {:?} {}", entry.time, action, status)
                }
                MemoryEntryKind::Event { description } => {
                    format!("[T{}] Event: {}", entry.time, description)
                }
                MemoryEntryKind::Note { content } => {
                    format!("[T{}] Note: {}", entry.time, content)
                }
            };
            lines.push(line);
        }
        lines.join("\n")
    }
}

// ============================================================================
// Long-Term Memory
// ============================================================================

/// Long-term memory entry: a summarized or indexed piece of knowledge.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LongTermMemoryEntry {
    /// Unique identifier for this entry.
    pub id: String,
    /// The world time when this was stored.
    pub created_at: WorldTime,
    /// Last accessed time.
    pub last_accessed: WorldTime,
    /// Access count (for LRU-like eviction).
    pub access_count: u64,
    /// The content of the memory.
    pub content: String,
    /// Tags for categorization and retrieval.
    pub tags: Vec<String>,
    /// Importance score.
    pub importance: f64,
}

impl LongTermMemoryEntry {
    /// Create a new long-term memory entry.
    pub fn new(id: impl Into<String>, created_at: WorldTime, content: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            created_at,
            last_accessed: created_at,
            access_count: 0,
            content: content.into(),
            tags: Vec::new(),
            importance: 0.5,
        }
    }

    /// Add a tag to this entry.
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    /// Set the importance score.
    pub fn with_importance(mut self, importance: f64) -> Self {
        self.importance = importance.clamp(0.0, 1.0);
        self
    }

    /// Mark this entry as accessed.
    pub fn mark_accessed(&mut self, time: WorldTime) {
        self.last_accessed = time;
        self.access_count += 1;
    }
}

/// Long-term memory store with basic retrieval.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct LongTermMemory {
    /// All stored entries, indexed by ID.
    pub(crate) entries: BTreeMap<String, LongTermMemoryEntry>,
    /// Maximum number of entries to keep.
    max_entries: Option<usize>,
    /// Counter for generating unique IDs.
    next_id: u64,
}

impl LongTermMemory {
    /// Create a new long-term memory store.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a new long-term memory store with a capacity limit.
    pub fn with_capacity(max_entries: usize) -> Self {
        Self {
            entries: BTreeMap::new(),
            max_entries: Some(max_entries),
            next_id: 0,
        }
    }

    /// Store a new entry and return its ID.
    pub fn store(&mut self, content: impl Into<String>, time: WorldTime) -> String {
        let id = format!("mem-{}", self.next_id);
        self.next_id += 1;

        let entry = LongTermMemoryEntry::new(&id, time, content);
        self.entries.insert(id.clone(), entry);
        self.evict_over_capacity();

        id
    }

    /// Store an entry with tags.
    pub fn store_with_tags(
        &mut self,
        content: impl Into<String>,
        time: WorldTime,
        tags: Vec<String>,
    ) -> String {
        let id = self.store(content, time);
        if let Some(entry) = self.entries.get_mut(&id) {
            entry.tags = tags;
        }
        id
    }

    /// Export all entries for external persistence.
    pub fn export_entries(&self) -> Vec<LongTermMemoryEntry> {
        self.entries.values().cloned().collect()
    }

    /// Restore all entries from an external persisted snapshot.
    pub fn restore_entries(&mut self, entries: Vec<LongTermMemoryEntry>) {
        self.entries.clear();
        for mut entry in entries {
            entry.importance = entry.importance.clamp(0.0, 1.0);
            if entry.last_accessed < entry.created_at {
                entry.last_accessed = entry.created_at;
            }
            self.entries.insert(entry.id.clone(), entry);
        }
        self.evict_over_capacity();
        self.next_id = self.next_id_from_entries();
    }

    /// Retrieve an entry by ID.
    pub fn get(&mut self, id: &str, time: WorldTime) -> Option<&LongTermMemoryEntry> {
        if let Some(entry) = self.entries.get_mut(id) {
            entry.mark_accessed(time);
        }
        self.entries.get(id)
    }

    /// Search entries by tag.
    pub fn search_by_tag(&self, tag: &str) -> Vec<&LongTermMemoryEntry> {
        self.entries
            .values()
            .filter(|e| e.tags.iter().any(|t| t == tag))
            .collect()
    }

    /// Search entries by content (simple substring match).
    pub fn search_by_content(&self, query: &str) -> Vec<&LongTermMemoryEntry> {
        let query_lower = query.to_lowercase();
        self.entries
            .values()
            .filter(|e| e.content.to_lowercase().contains(&query_lower))
            .collect()
    }

    /// Get the most important entries.
    pub fn top_by_importance(&self, n: usize) -> Vec<&LongTermMemoryEntry> {
        let mut top = Vec::with_capacity(n.min(self.entries.len()));
        if n == 0 {
            return top;
        }
        if n >= self.entries.len() {
            top.extend(self.entries.values());
            top.sort_by(|left, right| compare_importance_desc(left, right));
            return top;
        }
        for entry in self.entries.values() {
            insert_top_by_importance(&mut top, entry, n);
        }
        top
    }

    /// Get the most recently accessed entries.
    pub fn recently_accessed(&self, n: usize) -> Vec<&LongTermMemoryEntry> {
        let mut entries: Vec<_> = self.entries.values().collect();
        entries.sort_by(|a, b| b.last_accessed.cmp(&a.last_accessed));
        entries.into_iter().take(n).collect()
    }

    /// Get all entries.
    pub fn all(&self) -> impl Iterator<Item = &LongTermMemoryEntry> {
        self.entries.values()
    }

    /// Get the number of entries.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if the memory is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Remove an entry by ID.
    pub fn remove(&mut self, id: &str) -> Option<LongTermMemoryEntry> {
        self.entries.remove(id)
    }

    /// Clear all entries.
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    fn evict_over_capacity(&mut self) {
        if let Some(max) = self.max_entries {
            while self.entries.len() > max {
                if let Some(to_remove) = self
                    .entries
                    .iter()
                    .min_by(|a, b| {
                        a.1.importance
                            .partial_cmp(&b.1.importance)
                            .unwrap_or(std::cmp::Ordering::Equal)
                    })
                    .map(|(key, _)| key.clone())
                {
                    self.entries.remove(&to_remove);
                } else {
                    break;
                }
            }
        }
    }

    fn next_id_from_entries(&self) -> u64 {
        let mut next = self.entries.len() as u64;
        for entry_id in self.entries.keys() {
            let Some(raw) = entry_id.strip_prefix("mem-") else {
                continue;
            };
            if let Ok(parsed) = raw.parse::<u64>() {
                next = next.max(parsed.saturating_add(1));
            }
        }
        next
    }
}

fn insert_top_by_importance<'a>(
    top: &mut Vec<&'a LongTermMemoryEntry>,
    candidate: &'a LongTermMemoryEntry,
    limit: usize,
) {
    let insert_at = top
        .iter()
        .position(|entry| compare_importance_desc(candidate, entry) == Ordering::Less);
    match insert_at {
        Some(index) => top.insert(index, candidate),
        None if top.len() < limit => top.push(candidate),
        None => return,
    }
    if top.len() > limit {
        top.pop();
    }
}

fn compare_importance_desc(left: &LongTermMemoryEntry, right: &LongTermMemoryEntry) -> Ordering {
    right
        .importance
        .partial_cmp(&left.importance)
        .unwrap_or(Ordering::Equal)
}

// ============================================================================
// Agent Memory (combined)
// ============================================================================

/// Combined memory system for an agent.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentMemory {
    /// Short-term memory buffer.
    pub short_term: ShortTermMemory,
    /// Long-term memory store.
    pub long_term: LongTermMemory,
}

impl Default for AgentMemory {
    fn default() -> Self {
        Self {
            short_term: ShortTermMemory::default(),
            long_term: LongTermMemory::new(),
        }
    }
}

impl AgentMemory {
    /// Create a new agent memory with default settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a new agent memory with custom capacities.
    pub fn with_capacities(short_term_capacity: usize, long_term_capacity: usize) -> Self {
        Self {
            short_term: ShortTermMemory::new(short_term_capacity),
            long_term: LongTermMemory::with_capacity(long_term_capacity),
        }
    }

    /// Record an observation.
    pub fn record_observation(&mut self, time: WorldTime, summary: impl Into<String>) {
        self.short_term.add(MemoryEntry::observation(time, summary));
    }

    /// Record a decision.
    pub fn record_decision(&mut self, time: WorldTime, decision: AgentDecision) {
        self.short_term.add(MemoryEntry::decision(time, decision));
    }

    /// Record an action result.
    pub fn record_action_result(&mut self, time: WorldTime, action: Action, success: bool) {
        self.short_term
            .add(MemoryEntry::action_result(time, action, success));
    }

    /// Record an event.
    pub fn record_event(&mut self, time: WorldTime, description: impl Into<String>) {
        self.short_term.add(MemoryEntry::event(time, description));
    }

    /// Record a note.
    pub fn record_note(&mut self, time: WorldTime, content: impl Into<String>) {
        self.short_term.add(MemoryEntry::note(time, content));
    }

    /// Consolidate recent short-term memories into long-term storage.
    ///
    /// This is a simple consolidation that stores important recent memories.
    /// More sophisticated implementations could use summarization or embedding.
    pub fn consolidate(&mut self, time: WorldTime, importance_threshold: f64) {
        let important: Vec<_> = self
            .short_term
            .important(importance_threshold)
            .cloned()
            .collect();

        for entry in important {
            let content = match &entry.kind {
                MemoryEntryKind::Observation { summary } => summary.clone(),
                MemoryEntryKind::Decision { decision } => format!("Decision: {:?}", decision),
                MemoryEntryKind::ActionResult { action, success } => {
                    let status = if *success { "succeeded" } else { "failed" };
                    format!("Action {:?} {}", action, status)
                }
                MemoryEntryKind::Event { description } => description.clone(),
                MemoryEntryKind::Note { content } => content.clone(),
            };

            let id = self.long_term.store(&content, time);
            if let Some(stored) = self.long_term.entries.get_mut(&id) {
                stored.importance = entry.importance;
            }
        }
    }

    /// Get a summary of recent context for decision-making.
    pub fn context_summary(&self, max_recent: usize) -> String {
        self.short_term.summarize(max_recent)
    }

    /// Export long-term memory entries for persistence.
    pub fn export_long_term_entries(&self) -> Vec<LongTermMemoryEntry> {
        self.long_term.export_entries()
    }

    /// Restore long-term memory entries from persisted state.
    pub fn restore_long_term_entries(&mut self, entries: Vec<LongTermMemoryEntry>) {
        self.long_term.restore_entries(entries);
    }
}
