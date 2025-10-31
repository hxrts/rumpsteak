//! Role definitions for choreographic protocols

use proc_macro2::{Ident, TokenStream};

/// A role (participant) in the choreography
///
/// Roles represent the different participants in a distributed protocol.
/// They can be simple (e.g., `Client`, `Server`) or parameterized
/// (e.g., `Worker[0]`, `Worker[N]` where the parameter can be a constant or variable).
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
/// // Parameterized role with concrete index
/// let worker = Role::indexed(format_ident!("Worker"), 0);
/// ```
#[derive(Debug, Clone)]
pub struct Role {
    /// The name identifier of the role
    pub name: Ident,
    /// Optional index for parameterized roles (e.g., Worker with index 0)
    pub index: Option<usize>,
    /// Optional parameter expression for symbolic indices (e.g., Worker[N])
    pub param: Option<TokenStream>,
    /// Size of the role array (for Worker[N], this would be N)
    pub array_size: Option<TokenStream>,
}

// Manual implementations for PartialEq, Eq, and Hash
// TokenStream doesn't implement these traits, so we compare based on string representation
impl PartialEq for Role {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
            && self.index == other.index
            && self.param.as_ref().map(|t| t.to_string())
                == other.param.as_ref().map(|t| t.to_string())
            && self.array_size.as_ref().map(|t| t.to_string())
                == other.array_size.as_ref().map(|t| t.to_string())
    }
}

impl Eq for Role {}

impl std::hash::Hash for Role {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.name.hash(state);
        self.index.hash(state);
        if let Some(param) = &self.param {
            param.to_string().hash(state);
        }
        if let Some(size) = &self.array_size {
            size.to_string().hash(state);
        }
    }
}

impl Role {
    /// Create a new simple role with the given name
    pub fn new(name: Ident) -> Self {
        Role {
            name,
            index: None,
            param: None,
            array_size: None,
        }
    }

    /// Create a new indexed role (e.g., Worker with index 0)
    pub fn indexed(name: Ident, index: usize) -> Self {
        Role {
            name,
            index: Some(index),
            param: None,
            array_size: None,
        }
    }

    /// Create a parameterized role with symbolic parameter (e.g., Worker[N])
    pub fn parameterized(name: Ident, param: TokenStream) -> Self {
        Role {
            name,
            index: None,
            param: Some(param.clone()),
            array_size: Some(param),
        }
    }

    /// Check if this role has an index
    pub fn is_indexed(&self) -> bool {
        self.index.is_some()
    }

    /// Generate a Rust identifier for this role
    pub fn to_ident(&self) -> Ident {
        self.name.clone()
    }

    /// Check if this role is parameterized (has either index or param)
    pub fn is_parameterized(&self) -> bool {
        self.index.is_some() || self.param.is_some()
    }

    /// Check if this is a role array (declared with size like Worker[N])
    pub fn is_array(&self) -> bool {
        self.array_size.is_some()
    }
}
