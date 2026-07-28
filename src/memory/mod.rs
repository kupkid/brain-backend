pub mod repository;
pub mod embedding;
pub mod ingestion;
pub mod retrieval;
pub mod heuristic;

pub use repository::MemoryRepository;
#[allow(unused_imports)]
pub use embedding::MemoryEmbeddingStore;
pub use ingestion::{MemoryIngestion, IngestResult, IngestParams, compute_content_hash};
pub use retrieval::MemoryRetriever;
pub use heuristic::{check_content, validate_layer_for_project};
