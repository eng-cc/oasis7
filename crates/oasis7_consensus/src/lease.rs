// Lease-based single-writer coordination for sequencer roles.

use super::util::blake3_hex;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LeaseState {
    pub holder_id: String,
    pub lease_id: String,
    pub acquired_at_ms: i64,
    pub expires_at_ms: i64,
    pub term: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LeaseDecision {
    pub granted: bool,
    pub lease: Option<LeaseState>,
    pub reason: Option<String>,
}

#[derive(Debug, Clone)]
pub struct LeaseManager {
    lease: Option<LeaseState>,
    next_term: u64,
}

impl Default for LeaseManager {
    fn default() -> Self {
        Self {
            lease: None,
            next_term: 1,
        }
    }
}

impl LeaseManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn current(&self) -> Option<&LeaseState> {
        self.lease.as_ref()
    }

    pub fn is_active(&self, now_ms: i64) -> bool {
        self.lease
            .as_ref()
            .map(|lease| lease.expires_at_ms > now_ms)
            .unwrap_or(false)
    }

    pub fn try_acquire(&mut self, holder_id: &str, now_ms: i64, ttl_ms: i64) -> LeaseDecision {
        if ttl_ms <= 0 {
            return LeaseDecision {
                granted: false,
                lease: self.lease.clone(),
                reason: Some("ttl must be positive".to_string()),
            };
        }

        if let Some(lease) = &self.lease {
            if lease.expires_at_ms > now_ms {
                return LeaseDecision {
                    granted: false,
                    lease: self.lease.clone(),
                    reason: Some("lease already held".to_string()),
                };
            }
        }

        let term = self.next_term;
        let next_term = match checked_lease_term_increment(term) {
            Ok(next_term) => next_term,
            Err(reason) => {
                return LeaseDecision {
                    granted: false,
                    lease: self.lease.clone(),
                    reason: Some(reason),
                };
            }
        };
        let expires_at_ms = match checked_lease_expiry(now_ms, ttl_ms) {
            Ok(expires_at_ms) => expires_at_ms,
            Err(reason) => {
                return LeaseDecision {
                    granted: false,
                    lease: self.lease.clone(),
                    reason: Some(reason),
                };
            }
        };
        let lease_id = lease_id_for(holder_id, now_ms, term);
        let lease = LeaseState {
            holder_id: holder_id.to_string(),
            lease_id,
            acquired_at_ms: now_ms,
            expires_at_ms,
            term,
        };
        self.next_term = next_term;
        self.lease = Some(lease.clone());

        LeaseDecision {
            granted: true,
            lease: Some(lease),
            reason: None,
        }
    }

    pub fn renew(&mut self, lease_id: &str, now_ms: i64, ttl_ms: i64) -> LeaseDecision {
        let Some(lease) = &mut self.lease else {
            return LeaseDecision {
                granted: false,
                lease: None,
                reason: Some("no active lease".to_string()),
            };
        };
        if lease.lease_id != lease_id {
            return LeaseDecision {
                granted: false,
                lease: Some(lease.clone()),
                reason: Some("lease_id mismatch".to_string()),
            };
        }
        if lease.expires_at_ms <= now_ms {
            return LeaseDecision {
                granted: false,
                lease: Some(lease.clone()),
                reason: Some("lease expired".to_string()),
            };
        }
        if ttl_ms <= 0 {
            return LeaseDecision {
                granted: false,
                lease: Some(lease.clone()),
                reason: Some("ttl must be positive".to_string()),
            };
        }

        let expires_at_ms = match checked_lease_expiry(now_ms, ttl_ms) {
            Ok(expires_at_ms) => expires_at_ms,
            Err(reason) => {
                return LeaseDecision {
                    granted: false,
                    lease: Some(lease.clone()),
                    reason: Some(reason),
                };
            }
        };
        lease.expires_at_ms = expires_at_ms;
        LeaseDecision {
            granted: true,
            lease: Some(lease.clone()),
            reason: None,
        }
    }

    pub fn release(&mut self, lease_id: &str) -> bool {
        if let Some(lease) = &self.lease {
            if lease.lease_id == lease_id {
                self.lease = None;
                return true;
            }
        }
        false
    }

    pub fn expire_if_needed(&mut self, now_ms: i64) -> Option<LeaseState> {
        if let Some(lease) = &self.lease {
            if lease.expires_at_ms <= now_ms {
                return self.lease.take();
            }
        }
        None
    }
}

#[derive(Debug, Clone, Default)]
pub struct ScopedLeaseManager {
    leases: HashMap<String, LeaseManager>,
}

impl ScopedLeaseManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn current(&self, scope: &str) -> Option<&LeaseState> {
        self.leases.get(scope).and_then(LeaseManager::current)
    }

    pub fn try_acquire(
        &mut self,
        scope: &str,
        holder_id: &str,
        now_ms: i64,
        ttl_ms: i64,
    ) -> LeaseDecision {
        if scope.trim().is_empty() {
            return LeaseDecision {
                granted: false,
                lease: None,
                reason: Some("scope cannot be empty".to_string()),
            };
        }
        self.leases
            .entry(scope.to_string())
            .or_default()
            .try_acquire(holder_id, now_ms, ttl_ms)
    }

    pub fn renew(
        &mut self,
        scope: &str,
        lease_id: &str,
        now_ms: i64,
        ttl_ms: i64,
    ) -> LeaseDecision {
        let Some(manager) = self.leases.get_mut(scope) else {
            return LeaseDecision {
                granted: false,
                lease: None,
                reason: Some("scope has no active lease".to_string()),
            };
        };
        manager.renew(lease_id, now_ms, ttl_ms)
    }

    pub fn release(&mut self, scope: &str, lease_id: &str) -> bool {
        let Some(manager) = self.leases.get_mut(scope) else {
            return false;
        };
        let released = manager.release(lease_id);
        if released && manager.current().is_none() {
            self.leases.remove(scope);
        }
        released
    }

    pub fn expire_if_needed(&mut self, scope: &str, now_ms: i64) -> Option<LeaseState> {
        let expired = self.leases.get_mut(scope)?.expire_if_needed(now_ms);
        if expired.is_some()
            && self
                .leases
                .get(scope)
                .is_some_and(|manager| manager.current().is_none())
        {
            self.leases.remove(scope);
        }
        expired
    }

    pub fn scope_count(&self) -> usize {
        self.leases.len()
    }
}

fn checked_lease_term_increment(value: u64) -> Result<u64, String> {
    value
        .checked_add(1)
        .ok_or_else(|| format!("lease term overflow at {value}"))
}

fn checked_lease_expiry(now_ms: i64, ttl_ms: i64) -> Result<i64, String> {
    now_ms
        .checked_add(ttl_ms)
        .ok_or_else(|| format!("lease expires_at overflow: now_ms={now_ms}, ttl_ms={ttl_ms}"))
}

fn lease_id_for(holder_id: &str, now_ms: i64, term: u64) -> String {
    let payload = format!("{holder_id}:{now_ms}:{term}");
    blake3_hex(payload.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn acquire_and_release_lease() {
        let mut manager = LeaseManager::new();
        let decision = manager.try_acquire("seq-1", 100, 50);
        assert!(decision.granted);
        let lease_id = decision.lease.as_ref().unwrap().lease_id.clone();
        assert!(manager.release(&lease_id));
        assert!(manager.current().is_none());
    }

    #[test]
    fn deny_acquire_when_active() {
        let mut manager = LeaseManager::new();
        let first = manager.try_acquire("seq-1", 100, 50);
        assert!(first.granted);
        let second = manager.try_acquire("seq-2", 120, 50);
        assert!(!second.granted);
    }

    #[test]
    fn allow_acquire_after_expiry() {
        let mut manager = LeaseManager::new();
        let first = manager.try_acquire("seq-1", 100, 10);
        assert!(first.granted);
        manager.expire_if_needed(200);
        let second = manager.try_acquire("seq-2", 200, 10);
        assert!(second.granted);
    }

    #[test]
    fn renew_extends_lease() {
        let mut manager = LeaseManager::new();
        let first = manager.try_acquire("seq-1", 100, 10);
        let lease = first.lease.unwrap();
        let renewed = manager.renew(&lease.lease_id, 105, 20);
        assert!(renewed.granted);
        assert!(manager.is_active(115));
    }

    #[test]
    fn try_acquire_rejects_term_overflow_without_mutation() {
        let mut manager = LeaseManager::new();
        manager.next_term = u64::MAX;

        let decision = manager.try_acquire("seq-1", 100, 10);
        assert!(!decision.granted);
        assert!(
            decision
                .reason
                .as_ref()
                .is_some_and(|reason| reason.contains("term overflow"))
        );
        assert!(manager.current().is_none());
        assert_eq!(manager.next_term, u64::MAX);
    }

    #[test]
    fn try_acquire_rejects_expiry_overflow_without_mutation() {
        let mut manager = LeaseManager::new();

        let decision = manager.try_acquire("seq-1", i64::MAX - 1, 10);
        assert!(!decision.granted);
        assert!(
            decision
                .reason
                .as_ref()
                .is_some_and(|reason| reason.contains("expires_at overflow"))
        );
        assert!(manager.current().is_none());
        assert_eq!(manager.next_term, 1);
    }

    #[test]
    fn renew_rejects_expiry_overflow_without_mutation() {
        let mut manager = LeaseManager::new();
        let first = manager.try_acquire("seq-1", i64::MAX - 20, 10);
        let lease = first.lease.expect("lease");
        let previous_expiry = lease.expires_at_ms;

        let renewed = manager.renew(&lease.lease_id, i64::MAX - 15, 20);
        assert!(!renewed.granted);
        assert!(
            renewed
                .reason
                .as_ref()
                .is_some_and(|reason| reason.contains("expires_at overflow"))
        );
        assert_eq!(
            manager.current().expect("active lease").expires_at_ms,
            previous_expiry
        );
    }

    #[test]
    fn scoped_manager_allows_parallel_zone_leases() {
        let mut scoped = ScopedLeaseManager::new();
        let zone_a = scoped.try_acquire("zone-a", "seq-1", 100, 20);
        let zone_b = scoped.try_acquire("zone-b", "seq-2", 100, 20);
        assert!(zone_a.granted);
        assert!(zone_b.granted);
        assert_eq!(scoped.scope_count(), 2);
    }

    #[test]
    fn scoped_manager_blocks_conflict_within_same_zone() {
        let mut scoped = ScopedLeaseManager::new();
        let first = scoped.try_acquire("zone-a", "seq-1", 100, 20);
        assert!(first.granted);
        let second = scoped.try_acquire("zone-a", "seq-2", 110, 20);
        assert!(!second.granted);
        let lease_id = first.lease.expect("lease").lease_id;
        assert!(scoped.release("zone-a", &lease_id));
        let retry = scoped.try_acquire("zone-a", "seq-2", 120, 20);
        assert!(retry.granted);
    }
}
