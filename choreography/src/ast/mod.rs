//! Abstract Syntax Tree for choreographic protocols
//!
//! This module defines the core AST types used to represent choreographic protocols,
//! including global protocols, local (projected) types, roles, and messages.

/// Choreography definitions (global protocols with metadata)
pub mod choreography;

/// Local types resulting from projection
pub mod local_type;

/// Message type definitions
pub mod message;

/// Protocol combinators (global protocol constructs)
pub mod protocol;

/// Role definitions
pub mod role;

/// Validation errors and utilities
pub mod validation;

pub use choreography::*;
pub use local_type::*;
pub use message::*;
pub use protocol::*;
pub use role::*;
pub use validation::*;
