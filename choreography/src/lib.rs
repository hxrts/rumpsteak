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
    ChoreoHandler, ChoreoHandlerExt, ChoreographyError, Result, Label, RoleId, Endpoint,
    Program, Effect, interpret, InterpretResult, InterpreterState, ProgramMessage
};
pub use effects::middleware::{Trace, Metrics, Retry};
pub use effects::impls::{RumpsteakHandler, InMemoryHandler, RecordingHandler};
pub use effects::{NoOpHandler};

// Re-export macros from rumpsteak-macros
pub use rumpsteak_macros::choreography;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_structure() {
        // Test that main re-exports are available
        let _choreography: Option<Choreography> = None;
        let _protocol: Option<Protocol> = None;
        let _role: Option<Role> = None;
        let _message_type: Option<MessageType> = None;
        
        // Test effect system is available
        let _program: Option<Program<(), ()>> = None;
        let _result: Option<Result<()>> = None;
        let _label: Option<Label> = None;
    }

    #[test]
    fn test_free_algebra_integration() {
        use std::time::Duration;
        
        // Test that Program can be built using the free algebra API
        let program = Program::new()
            .send((), ())
            .recv::<()>(())
            .choose((), Label("test"))
            .offer(())
            .with_timeout((), Duration::from_millis(100), Program::new().end())
            .parallel(vec![Program::new().end()])
            .end();
            
        // Basic analysis should work
        assert_eq!(program.send_count(), 1);
        assert_eq!(program.recv_count(), 1);
        assert!(program.has_timeouts());
        assert!(program.has_parallel());
    }
}