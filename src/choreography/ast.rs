// Abstract Syntax Tree for choreographic protocols

use proc_macro2::{Ident, TokenStream};
use std::collections::HashMap;

/// A complete choreographic protocol specification
#[derive(Debug, Clone)]
pub struct Choreography {
    /// Protocol name
    pub name: Ident,
    /// Participating roles
    pub roles: Vec<Role>,
    /// The protocol specification
    pub protocol: Protocol,
    /// Metadata and attributes
    pub attrs: HashMap<String, String>,
}

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
}

/// Message type with optional payload
#[derive(Debug, Clone)]
pub struct MessageType {
    pub name: Ident,
    pub payload: Option<TokenStream>,
}

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

/// Local session type after projection
#[derive(Debug, Clone)]
pub enum LocalType {
    /// Send a message
    Send {
        to: Role,
        message: MessageType,
        continuation: Box<LocalType>,
    },
    
    /// Receive a message
    Receive {
        from: Role,
        message: MessageType,
        continuation: Box<LocalType>,
    },
    
    /// Make a choice (select)
    Select {
        to: Role,
        branches: Vec<(Ident, LocalType)>,
    },
    
    /// Receive a choice (branch)
    Branch {
        from: Role,
        branches: Vec<(Ident, LocalType)>,
    },
    
    /// Recursive type
    Rec {
        label: Ident,
        body: Box<LocalType>,
    },
    
    /// Variable (reference to recursive type)
    Var(Ident),
    
    /// Type termination
    End,
}

impl LocalType {
    /// Check if this type is well-formed
    pub fn is_well_formed(&self) -> bool {
        self.check_well_formed(&mut vec![])
    }
    
    fn check_well_formed(&self, rec_vars: &mut Vec<Ident>) -> bool {
        match self {
            LocalType::Send { continuation, .. } => continuation.check_well_formed(rec_vars),
            LocalType::Receive { continuation, .. } => continuation.check_well_formed(rec_vars),
            LocalType::Select { branches, .. } => {
                branches.iter().all(|(_, ty)| ty.check_well_formed(rec_vars))
            }
            LocalType::Branch { branches, .. } => {
                branches.iter().all(|(_, ty)| ty.check_well_formed(rec_vars))
            }
            LocalType::Rec { label, body } => {
                rec_vars.push(label.clone());
                let result = body.check_well_formed(rec_vars);
                rec_vars.pop();
                result
            }
            LocalType::Var(label) => rec_vars.contains(label),
            LocalType::End => true,
        }
    }
}

/// Choreography validation errors
#[derive(Debug, Clone, thiserror::Error)]
pub enum ValidationError {
    #[error("Role {0} not declared in choreography")]
    UndefinedRole(String),
    
    #[error("Recursive variable {0} not bound")]
    UnboundVariable(String),
    
    #[error("Choice role {0} must be sender in all branches")]
    InvalidChoice(String),
    
    #[error("Deadlock detected in protocol")]
    Deadlock,
    
    #[error("Role {0} is not used in protocol")]
    UnusedRole(String),
}

impl Choreography {
    /// Validate the choreography for correctness
    pub fn validate(&self) -> Result<(), ValidationError> {
        // Check all roles are used
        for role in &self.roles {
            if !self.protocol.mentions_role(role) {
                return Err(ValidationError::UnusedRole(
                    role.name.to_string()
                ));
            }
        }
        
        // Check protocol is well-formed
        self.protocol.validate(&self.roles)?;
        
        Ok(())
    }
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
    
    fn validate(&self, roles: &[Role]) -> Result<(), ValidationError> {
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