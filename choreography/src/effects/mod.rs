//! Effect handler system for choreographic programming
//! 
//! This module provides the effect handler abstraction that decouples protocol
//! logic from transport implementation, enabling testable and composable protocols.
//!
//! The system uses a free algebra approach where choreographic programs are
//! represented as data structures that can be analyzed, transformed, and interpreted.

pub mod handler;
pub mod middleware;
pub mod impls;
pub mod algebra;
pub mod interpreter;

pub use handler::*;
pub use algebra::*;
pub use interpreter::{interpret, ChoreoHandlerExt};