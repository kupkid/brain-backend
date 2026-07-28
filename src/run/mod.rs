pub mod state;
pub mod events;
pub mod repository;

#[allow(unused_imports)] // SCAFFOLD — re-exports for future modules
pub use state::RunStateMachine;
#[allow(unused_imports)]
pub use events::EventStore;
pub use repository::RunRepository;
