use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};

pub const MAX_CACHE_ENTRIES: usize = 1024;

pub struct BoundedCache<V> {
    capacity: usize,
    entries: HashMap<String, Arc<V>>,
    insertion_order: VecDeque<String>,
}

impl<V> BoundedCache<V> {
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            entries: HashMap::new(),
            insertion_order: VecDeque::new(),
        }
    }

    pub fn get_cloned(&self, key: &str) -> Option<Arc<V>> {
        self.entries.get(key).cloned()
    }

    pub fn insert(&mut self, key: String, value: Arc<V>) {
        if self.capacity == 0 {
            self.entries.clear();
            self.insertion_order.clear();
            return;
        }
        if self.entries.contains_key(&key) {
            self.entries.insert(key, value);
            return;
        }
        while self.entries.len() >= self.capacity {
            if let Some(oldest_key) = self.insertion_order.pop_front() {
                self.entries.remove(&oldest_key);
            } else {
                break;
            }
        }
        self.insertion_order.push_back(key.clone());
        self.entries.insert(key, value);
    }
}

pub type RegexCache = Mutex<BoundedCache<regex::Regex>>;
