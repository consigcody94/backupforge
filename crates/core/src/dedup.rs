use backupforge_common::{types::ChunkId, Result, Error};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};

/// In-memory deduplication index
/// Tracks which chunks have already been stored
#[derive(Clone)]
pub struct DedupIndex {
    chunks: Arc<RwLock<HashSet<String>>>,
    chunk_refs: Arc<RwLock<HashMap<String, u64>>>, // chunk_id -> reference count
}

impl DedupIndex {
    pub fn new() -> Self {
        Self {
            chunks: Arc::new(RwLock::new(HashSet::new())),
            chunk_refs: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Check if a chunk already exists
    pub fn contains(&self, chunk_id: &ChunkId) -> bool {
        self.chunks.read().unwrap().contains(&chunk_id.0)
    }

    /// Add a chunk to the index
    pub fn insert(&self, chunk_id: ChunkId) {
        let mut chunks = self.chunks.write().unwrap();
        let mut refs = self.chunk_refs.write().unwrap();

        chunks.insert(chunk_id.0.clone());
        *refs.entry(chunk_id.0).or_insert(0) += 1;
    }

    /// Remove a chunk reference (decrement ref count)
    pub fn remove(&self, chunk_id: &ChunkId) -> bool {
        let mut refs = self.chunk_refs.write().unwrap();

        if let Some(count) = refs.get_mut(&chunk_id.0) {
            *count = count.saturating_sub(1);

            if *count == 0 {
                refs.remove(&chunk_id.0);
                let mut chunks = self.chunks.write().unwrap();
                chunks.remove(&chunk_id.0);
                return true; // Chunk can be deleted from storage
            }
        }

        false
    }

    /// Get number of unique chunks
    pub fn len(&self) -> usize {
        self.chunks.read().unwrap().len()
    }

    /// Check if index is empty
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Get reference count for a chunk
    pub fn get_ref_count(&self, chunk_id: &ChunkId) -> u64 {
        self.chunk_refs
            .read()
            .unwrap()
            .get(&chunk_id.0)
            .copied()
            .unwrap_or(0)
    }
}

impl Default for DedupIndex {
    fn default() -> Self {
        Self::new()
    }
}

/// Deduplication store that combines index and storage
pub struct DedupStore {
    index: DedupIndex,
}

impl DedupStore {
    pub fn new() -> Self {
        Self {
            index: DedupIndex::new(),
        }
    }

    /// Check if we need to store this chunk or if it's a duplicate
    pub fn is_duplicate(&self, chunk_id: &ChunkId) -> bool {
        self.index.contains(chunk_id)
    }

    /// Register a new chunk
    pub fn register_chunk(&self, chunk_id: ChunkId) {
        self.index.insert(chunk_id);
    }

    /// Unregister a chunk (for garbage collection)
    pub fn unregister_chunk(&self, chunk_id: &ChunkId) -> bool {
        self.index.remove(chunk_id)
    }

    /// Get statistics
    pub fn stats(&self) -> DedupStats {
        DedupStats {
            total_chunks: self.index.len(),
        }
    }

    pub fn index(&self) -> &DedupIndex {
        &self.index
    }
}

impl Default for DedupStore {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct DedupStats {
    pub total_chunks: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dedup_index() {
        let index = DedupIndex::new();
        let chunk_id = ChunkId("test123".to_string());

        assert!(!index.contains(&chunk_id));

        index.insert(chunk_id.clone());
        assert!(index.contains(&chunk_id));
        assert_eq!(index.get_ref_count(&chunk_id), 1);

        // Insert same chunk again
        index.insert(chunk_id.clone());
        assert_eq!(index.get_ref_count(&chunk_id), 2);

        // Remove once
        assert!(!index.remove(&chunk_id));
        assert_eq!(index.get_ref_count(&chunk_id), 1);

        // Remove again (should actually remove)
        assert!(index.remove(&chunk_id));
        assert!(!index.contains(&chunk_id));
    }

    #[test]
    fn test_dedup_store() {
        let store = DedupStore::new();
        let chunk_id = ChunkId("chunk1".to_string());

        assert!(!store.is_duplicate(&chunk_id));

        store.register_chunk(chunk_id.clone());
        assert!(store.is_duplicate(&chunk_id));

        let stats = store.stats();
        assert_eq!(stats.total_chunks, 1);
    }
}
