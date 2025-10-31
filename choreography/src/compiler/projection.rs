// Projection from global choreographies to local session types

use crate::ast::{Choreography, Protocol, Role, LocalType, MessageType, Branch};
use std::collections::HashMap;

/// Project a choreography to a local session type for a specific role
pub fn project(choreography: &Choreography, role: &Role) -> Result<LocalType, ProjectionError> {
    let mut context = ProjectionContext::new(choreography, role);
    context.project_protocol(&choreography.protocol)
}

/// Errors that can occur during projection
#[derive(Debug, thiserror::Error)]
pub enum ProjectionError {
    #[error("Cannot project choice for non-participant role")]
    NonParticipantChoice,
    
    #[error("Parallel composition not supported for role {0}")]
    UnsupportedParallel(String),
    
    #[error("Inconsistent projections in parallel branches")]
    InconsistentParallel,
    
    #[error("Recursive variable {0} not in scope")]
    UnboundVariable(String),
}

/// Context for projection algorithm
struct ProjectionContext<'a> {
    #[allow(dead_code)]
    choreography: &'a Choreography,
    role: &'a Role,
    #[allow(dead_code)]
    rec_env: HashMap<String, LocalType>,
}

impl<'a> ProjectionContext<'a> {
    fn new(choreography: &'a Choreography, role: &'a Role) -> Self {
        ProjectionContext {
            choreography,
            role,
            rec_env: HashMap::new(),
        }
    }
    
    fn project_protocol(&mut self, protocol: &Protocol) -> Result<LocalType, ProjectionError> {
        match protocol {
            Protocol::Send { from, to, message, continuation } => {
                self.project_send(from, to, message, continuation)
            }
            
            Protocol::Broadcast { from, to_all, message, continuation } => {
                self.project_broadcast(from, to_all, message, continuation)
            }
            
            Protocol::Choice { role: choice_role, branches } => {
                self.project_choice(choice_role, branches)
            }
            
            Protocol::Loop { body, .. } => {
                // Simple loop projection - more sophisticated analysis needed for conditions
                self.project_protocol(body)
            }
            
            Protocol::Parallel { protocols } => {
                self.project_parallel(protocols)
            }
            
            Protocol::Rec { label, body } => {
                self.project_rec(label, body)
            }
            
            Protocol::Var(label) => {
                self.project_var(label)
            }
            
            Protocol::End => Ok(LocalType::End),
        }
    }
    
    fn project_send(
        &mut self,
        from: &Role,
        to: &Role,
        message: &MessageType,
        continuation: &Protocol,
    ) -> Result<LocalType, ProjectionError> {
        if self.role == from {
            // We are the sender
            Ok(LocalType::Send {
                to: to.clone(),
                message: message.clone(),
                continuation: Box::new(self.project_protocol(continuation)?),
            })
        } else if self.role == to {
            // We are the receiver
            Ok(LocalType::Receive {
                from: from.clone(),
                message: message.clone(),
                continuation: Box::new(self.project_protocol(continuation)?),
            })
        } else {
            // We are not involved, skip to continuation
            self.project_protocol(continuation)
        }
    }
    
    fn project_broadcast(
        &mut self,
        from: &Role,
        to_all: &[Role],
        message: &MessageType,
        continuation: &Protocol,
    ) -> Result<LocalType, ProjectionError> {
        if self.role == from {
            // We are broadcasting - need to send to each recipient
            let mut current = self.project_protocol(continuation)?;
            
            // Build sends in reverse order so they nest correctly
            for to in to_all.iter().rev() {
                current = LocalType::Send {
                    to: to.clone(),
                    message: message.clone(),
                    continuation: Box::new(current),
                };
            }
            
            Ok(current)
        } else if to_all.contains(self.role) {
            // We are receiving the broadcast
            Ok(LocalType::Receive {
                from: from.clone(),
                message: message.clone(),
                continuation: Box::new(self.project_protocol(continuation)?),
            })
        } else {
            // Not involved in broadcast
            self.project_protocol(continuation)
        }
    }
    
    fn project_choice(
        &mut self,
        choice_role: &Role,
        branches: &[Branch],
    ) -> Result<LocalType, ProjectionError> {
        if self.role == choice_role {
            // We make the choice - project as Select
            let mut local_branches = Vec::new();
            
            for branch in branches {
                // Skip the initial send (it's implied by the choice)
                let inner_protocol = match &branch.protocol {
                    Protocol::Send { continuation, .. } => continuation,
                    _ => unreachable!("Choice branch should start with send"),
                };
                
                let local_type = self.project_protocol(inner_protocol)?;
                local_branches.push((branch.label.clone(), local_type));
            }
            
            // Find the recipient (from first branch's send)
            let recipient = match &branches[0].protocol {
                Protocol::Send { to, .. } => to.clone(),
                _ => unreachable!(),
            };
            
            Ok(LocalType::Select {
                to: recipient,
                branches: local_branches,
            })
        } else {
            // Check if we receive the choice
            let mut receives_choice = false;
            let mut sender = None;
            
            for branch in branches {
                if let Protocol::Send { from, to, .. } = &branch.protocol {
                    if self.role == to {
                        receives_choice = true;
                        sender = Some(from.clone());
                        break;
                    }
                }
            }
            
            if receives_choice {
                // We receive the choice - project as Branch
                let sender = sender.unwrap();
                let mut local_branches = Vec::new();
                
                for branch in branches {
                    let local_type = self.project_protocol(&branch.protocol)?;
                    local_branches.push((branch.label.clone(), local_type));
                }
                
                Ok(LocalType::Branch {
                    from: sender,
                    branches: local_branches,
                })
            } else {
                // Not involved in the choice - merge continuations
                self.merge_choice_continuations(branches)
            }
        }
    }
    
    fn project_parallel(
        &mut self,
        protocols: &[Protocol],
    ) -> Result<LocalType, ProjectionError> {
        // For now, simple approach: if role appears in only one branch, project that
        // More sophisticated merging needed for general case
        
        let mut projections = Vec::new();
        for protocol in protocols {
            if protocol.mentions_role(self.role) {
                projections.push(self.project_protocol(protocol)?);
            }
        }
        
        match projections.len() {
            0 => Ok(LocalType::End),
            1 => Ok(projections.into_iter().next().unwrap()),
            _ => Err(ProjectionError::UnsupportedParallel(
                self.role.name.to_string()
            )),
        }
    }
    
    fn project_rec(
        &mut self,
        label: &proc_macro2::Ident,
        body: &Protocol,
    ) -> Result<LocalType, ProjectionError> {
        let body_projection = self.project_protocol(body)?;
        
        // Only include Rec if the body actually uses this role
        if body_projection == LocalType::End {
            Ok(LocalType::End)
        } else {
            Ok(LocalType::Rec {
                label: label.clone(),
                body: Box::new(body_projection),
            })
        }
    }
    
    fn project_var(
        &mut self,
        label: &proc_macro2::Ident,
    ) -> Result<LocalType, ProjectionError> {
        Ok(LocalType::Var(label.clone()))
    }
    
    fn merge_choice_continuations(
        &mut self,
        branches: &[Branch],
    ) -> Result<LocalType, ProjectionError> {
        // Project each branch and find where we rejoin
        let mut projections = Vec::new();
        
        for branch in branches {
            projections.push(self.project_protocol(&branch.protocol)?);
        }
        
        // Simple merge: if all projections are the same, use that
        // More sophisticated merging needed for general case
        if projections.windows(2).all(|w| w[0] == w[1]) {
            Ok(projections.into_iter().next().unwrap())
        } else {
            // For now, just take the first non-End projection
            Ok(projections.into_iter()
                .find(|p| p != &LocalType::End)
                .unwrap_or(LocalType::End))
        }
    }
}


// Helper to compare LocalTypes for equality
impl PartialEq for LocalType {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (LocalType::End, LocalType::End) => true,
            (LocalType::Var(a), LocalType::Var(b)) => a == b,
            (
                LocalType::Send { to: to1, message: msg1, continuation: cont1 },
                LocalType::Send { to: to2, message: msg2, continuation: cont2 }
            ) => to1 == to2 && msg1.name == msg2.name && cont1 == cont2,
            (
                LocalType::Receive { from: from1, message: msg1, continuation: cont1 },
                LocalType::Receive { from: from2, message: msg2, continuation: cont2 }
            ) => from1 == from2 && msg1.name == msg2.name && cont1 == cont2,
            _ => false,
        }
    }
}

impl Eq for LocalType {}