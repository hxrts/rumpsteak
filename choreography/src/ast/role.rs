//! Role definitions for choreographic protocols

use proc_macro2::Ident;

/// A role (participant) in the choreography
///
/// Roles represent the different participants in a distributed protocol.
/// They can be simple (e.g., `Client`, `Server`) or parameterized
/// (e.g., `Worker[0]`, `Worker[1]` where the number is the index).
///
/// # Examples
///
/// ```ignore
/// use quote::format_ident;
/// use rumpsteak_choreography::Role;
///
/// // Simple role
/// let client = Role::new(format_ident!("Client"));
///
/// // Parameterized role
/// let worker = Role::indexed(format_ident!("Worker"), 0);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Role {
    /// The name identifier of the role
    pub name: Ident,
    /// Optional index for parameterized roles (e.g., Worker with index 0)
    pub index: Option<usize>,
}

impl Role {
    /// Create a new simple role with the given name
    pub fn new(name: Ident) -> Self {
        Role { name, index: None }
    }

    /// Create a new indexed role (e.g., Worker with index 0)
    pub fn indexed(name: Ident, index: usize) -> Self {
        Role { name, index: Some(index) }
    }

    /// Check if this role has an index
    pub fn is_indexed(&self) -> bool {
        self.index.is_some()
    }

    /// Generate a Rust identifier for this role
    pub fn to_ident(&self) -> Ident {
        self.name.clone()
    }
    
    /// Check if this role is parameterized (same as `is_indexed`)
    pub fn is_parameterized(&self) -> bool {
        self.index.is_some()
    }
}
