//! Choreographic Programming for Rumpsteak
//! 
//! This crate provides a choreographic programming layer on top of Rumpsteak's
//! session types, enabling global protocol specification with automatic projection.
//! 
//! The choreographic approach allows you to write distributed protocols from a
//! global viewpoint, with automatic generation of local session types for each
//! participant. This includes an effect handler system that decouples protocol
//! logic from transport implementation.

pub mod ast;
pub mod compiler;
pub mod effects;

// Re-export main APIs
pub use ast::{Choreography, Protocol, Role, MessageType};
pub use compiler::{generate_effects_protocol};
pub use effects::{
    ChoreoHandler, ChoreoHandlerExt, ChoreographyError, Result, Label, RoleId, Endpoint
};
pub use effects::middleware::{Trace, Metrics, Retry};
pub use effects::impls::{RumpsteakHandler, InMemoryHandler, RecordingHandler};
pub use effects::{NoOpHandler};

// Re-export macros from rumpsteak-macros
pub use rumpsteak_macros::choreography;