//! Effect handler system for choreographic programming
//! 
//! This module provides the effect handler abstraction that decouples protocol
//! logic from transport implementation, enabling testable and composable protocols.

pub mod handler;
pub mod middleware;
pub mod impls;

pub use handler::*;