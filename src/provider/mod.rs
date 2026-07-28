pub mod embedding;
pub mod llm;

#[allow(unused_imports)] // SCAFFOLD — re-exports for future modules
pub use embedding::EmbeddingProvider;
#[allow(unused_imports)]
pub use llm::LlmProvider;
