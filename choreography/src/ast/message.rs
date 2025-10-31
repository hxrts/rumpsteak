//! Message type definitions for choreographic protocols

use proc_macro2::{Ident, TokenStream};

/// Message type with optional payload
///
/// Represents a message that can be sent between roles in a choreography.
/// Messages have a name and an optional payload type.
///
/// # Examples
///
/// ```ignore
/// use quote::{format_ident, quote};
/// use rumpsteak_choreography::MessageType;
///
/// // Simple message without payload
/// let ping = MessageType {
///     name: format_ident!("Ping"),
///     payload: None,
/// };
///
/// // Message with payload
/// let request = MessageType {
///     name: format_ident!("Request"),
///     payload: Some(quote! { String }),
/// };
/// ```
#[derive(Debug, Clone)]
pub struct MessageType {
    /// The name identifier of the message
    pub name: Ident,
    /// Optional payload type (as token stream)
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

