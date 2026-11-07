//! File update activity outlives the visible page and is cleared by the actual operation.

use std::{
    collections::BTreeMap,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
    time::Instant,
};

// File data is shared by both panes and survives their visible lifetimes. This
// revision describes mutations and explicit refreshes: loading it never increments it.
static DATA_REVISION: AtomicU64 = AtomicU64::new(0);

pub(super) fn data_revision() -> u64 {
    DATA_REVISION.load(Ordering::Acquire)
}

pub(super) fn mark_changed() {
    DATA_REVISION
        .try_update(Ordering::Release, Ordering::Relaxed, |value| {
            value.checked_add(1)
        })
        .expect("file data revision exhausted");
}

#[derive(Clone, Default)]
pub(super) struct FileOperations(Arc<Mutex<BTreeMap<String, Instant>>>);

pub(super) struct UpdateBatch {
    pub names: Vec<String>,
    owner: FileOperations,
}

impl FileOperations {
    pub fn begin(&self, names: impl IntoIterator<Item = String>) -> UpdateBatch {
        let mut active = self.0.lock().unwrap();
        let mut accepted = Vec::new();
        for name in names {
            if let std::collections::btree_map::Entry::Vacant(entry) = active.entry(name.clone()) {
                entry.insert(Instant::now());
                accepted.push(name);
            }
        }
        UpdateBatch {
            names: accepted,
            owner: self.clone(),
        }
    }
    pub fn is_active(&self) -> bool {
        !self.0.lock().unwrap().is_empty()
    }
    pub fn frame_for(&self, name: &str) -> Option<usize> {
        self.0
            .lock()
            .unwrap()
            .get(name)
            .map(|started| (started.elapsed().as_millis() / 160 % 4) as usize)
    }
}

impl Drop for UpdateBatch {
    fn drop(&mut self) {
        let mut active = self.owner.0.lock().unwrap();
        for name in &self.names {
            active.remove(name);
        }
        if !self.names.is_empty() {
            mark_changed();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn completion_clears_activity_without_a_page_callback() {
        let owner = FileOperations::default();
        let work = owner.begin(["one".into(), "two".into()]);
        assert!(owner.is_active());
        assert!(owner.begin(["one".into()]).names.is_empty());
        let returning_view = owner.clone();
        drop(owner);
        drop(work);
        assert!(!returning_view.is_active());
        assert_eq!(returning_view.frame_for("one"), None);
    }
}
