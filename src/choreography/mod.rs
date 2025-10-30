// Choreographic programming module for Rumpsteak
// Enables writing distributed protocols from a global viewpoint

pub mod ast;
pub mod parser;
pub mod projection;
pub mod codegen;
pub mod analysis;

pub use ast::{Choreography, Protocol, Role, MessageType};
pub use projection::project;
pub use codegen::generate_session_type;

// Note: The choreography macro would be defined in rumpsteak-macros crate
// For now, this is a placeholder for the future implementation
// pub use rumpsteak_macros::choreography;

#[cfg(test)]
mod tests;