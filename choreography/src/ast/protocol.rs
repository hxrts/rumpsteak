// Protocol AST definitions

use super::*;
use proc_macro2::{Ident, TokenStream};

/// Protocol specification using choreographic constructs
#[derive(Debug, Clone)]
pub enum Protocol {
    /// Message send: A -> B: Message
    Send {
        from: Role,
        to: Role,
        message: MessageType,
        continuation: Box<Protocol>,
    },
    
    /// Broadcast: A -> *: Message
    Broadcast {
        from: Role,
        to_all: Vec<Role>,
        message: MessageType,
        continuation: Box<Protocol>,
    },
    
    /// Choice made by a role
    Choice {
        role: Role,
        branches: Vec<Branch>,
    },
    
    /// Loop construct
    Loop {
        condition: Option<Condition>,
        body: Box<Protocol>,
    },
    
    /// Parallel composition
    Parallel {
        protocols: Vec<Protocol>,
    },
    
    /// Recursive protocol with label
    Rec {
        label: Ident,
        body: Box<Protocol>,
    },
    
    /// Reference to recursive label
    Var(Ident),
    
    /// Protocol termination
    End,
}

/// A branch in a choice
#[derive(Debug, Clone)]
pub struct Branch {
    pub label: Ident,
    pub protocol: Protocol,
}

/// Loop condition
#[derive(Debug, Clone)]
pub enum Condition {
    /// Loop while a role decides
    RoleDecides(Role),
    /// Fixed iteration count
    Count(usize),
    /// Custom condition
    Custom(TokenStream),
}

impl Protocol {
    pub fn mentions_role(&self, role: &Role) -> bool {
        match self {
            Protocol::Send { from, to, continuation, .. } => {
                from == role || to == role || continuation.mentions_role(role)
            }
            Protocol::Broadcast { from, to_all, continuation, .. } => {
                from == role || to_all.contains(role) || continuation.mentions_role(role)
            }
            Protocol::Choice { role: r, branches } => {
                r == role || branches.iter().any(|b| b.protocol.mentions_role(role))
            }
            Protocol::Loop { body, .. } => body.mentions_role(role),
            Protocol::Parallel { protocols } => {
                protocols.iter().any(|p| p.mentions_role(role))
            }
            Protocol::Rec { body, .. } => body.mentions_role(role),
            Protocol::Var(_) | Protocol::End => false,
        }
    }
    
    pub(crate) fn validate(&self, roles: &[Role]) -> Result<(), ValidationError> {
        match self {
            Protocol::Send { from, to, continuation, .. } => {
                if !roles.contains(from) {
                    return Err(ValidationError::UndefinedRole(from.name.to_string()));
                }
                if !roles.contains(to) {
                    return Err(ValidationError::UndefinedRole(to.name.to_string()));
                }
                continuation.validate(roles)
            }
            Protocol::Broadcast { from, to_all, continuation, .. } => {
                if !roles.contains(from) {
                    return Err(ValidationError::UndefinedRole(from.name.to_string()));
                }
                for to in to_all {
                    if !roles.contains(to) {
                        return Err(ValidationError::UndefinedRole(to.name.to_string()));
                    }
                }
                continuation.validate(roles)
            }
            Protocol::Choice { role, branches } => {
                if !roles.contains(role) {
                    return Err(ValidationError::UndefinedRole(role.name.to_string()));
                }
                // Validate each branch starts with the choosing role sending
                for branch in branches {
                    if let Protocol::Send { from, .. } = &branch.protocol {
                        if from != role {
                            return Err(ValidationError::InvalidChoice(role.name.to_string()));
                        }
                    } else {
                        return Err(ValidationError::InvalidChoice(role.name.to_string()));
                    }
                }
                Ok(())
            }
            Protocol::Loop { body, .. } => body.validate(roles),
            Protocol::Parallel { protocols } => {
                for p in protocols {
                    p.validate(roles)?;
                }
                Ok(())
            }
            Protocol::Rec { body, .. } => body.validate(roles),
            Protocol::Var(_) | Protocol::End => Ok(()),
        }
    }
}

