//! Local knowledge base for the lux agent.
//!
//! Provides RAG-style retrieval over local documentation:
//! man pages, package metadata, common fixes, hardware compatibility.
//!
//! TODO: Implement vector search with SQLite + embeddings.

/// Placeholder for knowledge base.
/// Will be implemented with local vector search.
pub struct KnowledgeBase;

impl KnowledgeBase {
    pub fn new() -> Self {
        Self
    }
}

impl Default for KnowledgeBase {
    fn default() -> Self {
        Self::new()
    }
}
