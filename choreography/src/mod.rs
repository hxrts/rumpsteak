// Choreographic programming module for Rumpsteak
// Enables writing distributed protocols from a global viewpoint

pub mod ast;
pub mod parser;
pub mod projection;
pub mod codegen;
pub mod analysis;
pub mod effects_codegen;

pub use ast::{Choreography, Protocol, Role, MessageType};
pub use projection::project;
pub use codegen::generate_session_type;
pub use effects_codegen::generate_effects_protocol;

// Note: The choreography macro would be defined in rumpsteak-macros crate
// For now, this is a placeholder for the future implementation
// pub use rumpsteak_macros::choreography;

// TODO: Add tests module when tests are implemented
// #[cfg(test)]
// mod tests;