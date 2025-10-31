// Role definitions

use proc_macro2::Ident;

/// A role (participant) in the choreography
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Role {
    pub name: Ident,
    /// For parameterized roles like Worker[N]
    pub index: Option<usize>,
}

impl Role {
    pub fn new(name: Ident) -> Self {
        Role { name, index: None }
    }

    pub fn indexed(name: Ident, index: usize) -> Self {
        Role { name, index: Some(index) }
    }

    pub fn is_indexed(&self) -> bool {
        self.index.is_some()
    }

    /// Generate a Rust identifier for this role
    pub fn to_ident(&self) -> Ident {
        self.name.clone()
    }
    
    /// Check if this role is parameterized
    pub fn is_parameterized(&self) -> bool {
        self.index.is_some()
    }
}
