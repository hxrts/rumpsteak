// Message type definitions

use proc_macro2::{Ident, TokenStream};

/// Message type with optional payload
#[derive(Debug, Clone)]
pub struct MessageType {
    pub name: Ident,
    pub payload: Option<TokenStream>,
}

impl PartialEq for MessageType {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name && 
        self.payload.as_ref().map(|ts| ts.to_string()) == other.payload.as_ref().map(|ts| ts.to_string())
    }
}

impl Eq for MessageType {}

impl std::hash::Hash for MessageType {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.name.hash(state);
        if let Some(ref payload) = self.payload {
            payload.to_string().hash(state);
        }
    }
}

impl MessageType {
    /// Generate a Rust type identifier for this message
    pub fn to_ident(&self) -> Ident {
        self.name.clone()
    }
}

