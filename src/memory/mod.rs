pub mod repository;
pub mod embedding;
pub mod ingestion;
pub mod retrieval;

pub use repository::MemoryRepository;
#[allow(unused_imports)] // SCAFFOLD — re-exports for future modules
pub use embedding::MemoryEmbeddingStore;
#[allow(unused_imports)]
pub use ingestion::MemoryIngestion;
#[allow(unused_imports)]
pub use retrieval::MemoryRetriever;
