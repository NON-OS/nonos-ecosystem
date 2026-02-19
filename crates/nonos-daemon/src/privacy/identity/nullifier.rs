use super::types::ScopedNullifier;
use std::collections::{HashSet, VecDeque};
use subtle::ConstantTimeEq;

const MAX_NULLIFIERS: usize = 1_000_000;

pub struct BoundedNullifierSet {
    set: HashSet<ScopedNullifier>,
    order: VecDeque<ScopedNullifier>,
    evicted: u64,
}

impl BoundedNullifierSet {
    pub fn new() -> Self {
        Self {
            set: HashSet::with_capacity(1024),
            order: VecDeque::with_capacity(1024),
            evicted: 0,
        }
    }

    pub fn contains(&self, nullifier: &[u8; 32], scope: &[u8; 32]) -> bool {
        let key = ScopedNullifier {
            nullifier: *nullifier,
            scope: *scope,
        };
        self.set.contains(&key)
    }

    pub fn contains_ct(&self, nullifier: &[u8; 32], scope: &[u8; 32]) -> bool {
        for existing in &self.set {
            let n_eq = existing.nullifier.ct_eq(nullifier);
            let s_eq = existing.scope.ct_eq(scope);
            if (n_eq & s_eq).into() {
                return true;
            }
        }
        false
    }

    pub fn insert(&mut self, nullifier: [u8; 32], scope: [u8; 32]) -> bool {
        let key = ScopedNullifier { nullifier, scope };

        if self.set.contains(&key) {
            return false;
        }

        while self.set.len() >= MAX_NULLIFIERS {
            if let Some(old) = self.order.pop_front() {
                self.set.remove(&old);
                self.evicted += 1;
            }
        }

        self.set.insert(key.clone());
        self.order.push_back(key);
        true
    }

    pub fn len(&self) -> usize {
        self.set.len()
    }

    pub fn evicted(&self) -> u64 {
        self.evicted
    }
}

impl Default for BoundedNullifierSet {
    fn default() -> Self {
        Self::new()
    }
}
