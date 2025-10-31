//! Effect handler system for choreographic programming
//!
//! This module provides the effect handler abstraction that decouples protocol
//! logic from transport implementation, enabling testable and composable protocols.
//!
//! The system uses a free algebra approach where choreographic programs are
//! represented as data structures that can be analyzed, transformed, and interpreted.

pub mod algebra;
pub mod handler;
pub mod handlers;
pub mod interpreter;
pub mod middleware;

// Re-export core effect system types explicitly
pub use algebra::{
    Effect, InterpretResult, InterpreterState, Program, ProgramError, ProgramMessage,
};
pub use handler::{
    ChoreoHandler, ChoreoHandlerExt, ChoreographyError, Endpoint, Label, NoOpHandler, Result,
    RoleId,
};
pub use interpreter::interpret;

// Re-export handler implementations for convenience
pub use handlers::{InMemoryHandler, RecordedEvent, RecordingHandler};

#[cfg(not(target_arch = "wasm32"))]
pub use handlers::{HasRoute, RumpsteakEndpoint, RumpsteakHandler};

// Re-export middleware for convenience
pub use middleware::{Metrics, Retry, Trace};

#[cfg(feature = "test-utils")]
pub use middleware::FaultInjection;
