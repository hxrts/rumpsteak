//! Choreography compilation pipeline
//! 
//! This module contains the compilation pipeline that transforms choreographic
//! specifications into executable code.

pub mod parser;
pub mod analysis;
pub mod projection;
pub mod codegen;
pub mod effects_codegen;

pub use parser::*;
pub use analysis::*;
pub use projection::*;
pub use codegen::*;
pub use effects_codegen::*;