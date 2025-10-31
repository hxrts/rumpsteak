// Effect Handler Implementations
//
// This module contains concrete implementations of the ChoreoHandler trait
// for different execution environments:
//
// - in_memory: Fast tokio-based handler for testing
// - recording: Captures effects for verification
// - rumpsteak: Session-typed Rumpsteak integration (placeholder)

pub mod in_memory;
pub mod recording;
#[cfg(not(target_arch = "wasm32"))]
pub mod rumpsteak;

// Re-export handler types for convenience
pub use in_memory::InMemoryHandler;
pub use recording::{RecordedEvent, RecordingHandler};
#[cfg(not(target_arch = "wasm32"))]
pub use rumpsteak::{HasRoute, RumpsteakEndpoint, RumpsteakHandler};
