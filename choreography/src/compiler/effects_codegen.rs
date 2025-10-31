// Code generation for effect-based choreographic protocols
//
// This module generates protocol implementations that use the effect handler
// abstraction instead of directly calling transport methods.

use crate::ast::{Choreography, Protocol, Role, MessageType};
use quote::{quote, format_ident};
use proc_macro2::TokenStream;
use std::collections::HashSet;

/// Generate effect-based protocol implementation
pub fn generate_effects_protocol(choreography: &Choreography) -> TokenStream {
    let protocol_name = &choreography.name;
    let roles = generate_role_enum(&choreography.roles);
    let messages = generate_message_types(&choreography.protocol);
    let role_functions = generate_role_functions(choreography);
    let endpoint_type = generate_endpoint_type(protocol_name);
    
    quote! {
        use rumpsteak::effects::{ChoreoHandler, Result, Label};
        use serde::{Serialize, Deserialize};
        
        #roles
        
        #endpoint_type
        
        #messages
        
        #role_functions
    }
}

fn generate_role_enum(roles: &[Role]) -> TokenStream {
    let role_names: Vec<_> = roles.iter().map(|r| &r.name).collect();
    
    quote! {
        #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
        pub enum Role {
            #(#role_names),*
        }
        
        impl rumpsteak::effects::RoleId for Role {}
    }
}

fn generate_endpoint_type(protocol_name: &proc_macro2::Ident) -> TokenStream {
    let ep_name = format_ident!("{}Endpoint", protocol_name);
    
    quote! {
        pub struct #ep_name {
            // Protocol-specific endpoint state
        }
        
        impl rumpsteak::effects::Endpoint for #ep_name {}
    }
}

fn generate_message_types(protocol: &Protocol) -> TokenStream {
    let mut message_types = HashSet::new();
    
    // Collect unique message types from protocol
    collect_message_types(protocol, &mut message_types);
    
    let message_structs: Vec<_> = message_types.into_iter().map(|msg_type| {
        let type_name = &msg_type.name;
        let content_type = if let Some(ref payload) = msg_type.payload {
            payload.clone()
        } else {
            infer_content_type(&msg_type.name.to_string())
        };
        
        quote! {
            #[derive(Clone, Debug, Serialize, Deserialize)]
            pub struct #type_name(pub #content_type);
        }
    }).collect();
    
    quote! {
        #(#message_structs)*
    }
}

fn collect_message_types(protocol: &Protocol, message_types: &mut HashSet<MessageType>) {
    match protocol {
        Protocol::Send { message, continuation, .. } => {
            message_types.insert(message.clone());
            collect_message_types(continuation, message_types);
        }
        Protocol::Broadcast { message, continuation, .. } => {
            message_types.insert(message.clone());
            collect_message_types(continuation, message_types);
        }
        Protocol::Choice { branches, .. } => {
            for branch in branches {
                collect_message_types(&branch.protocol, message_types);
            }
        }
        Protocol::Loop { body, .. } => {
            collect_message_types(body, message_types);
        }
        Protocol::Parallel { protocols } => {
            for p in protocols {
                collect_message_types(p, message_types);
            }
        }
        Protocol::Rec { body, .. } => {
            collect_message_types(body, message_types);
        }
        Protocol::Var(_) | Protocol::End => {}
    }
}

fn generate_role_functions(choreography: &Choreography) -> TokenStream {
    choreography.roles.iter().map(|role| {
        let role_name_str = role.name.to_string().to_lowercase();
        let fn_name = format_ident!("run_{}", role_name_str);
        let _role_name = &role.name;
        let protocol_name = &choreography.name;
        let endpoint_type = format_ident!("{}Endpoint", protocol_name);
        
        let body = generate_role_body(&choreography.protocol, role);
        
        quote! {
            pub async fn #fn_name<H: ChoreoHandler<Role = Role, Endpoint = #endpoint_type>>(
                handler: &mut H,
                endpoint: &mut #endpoint_type,
            ) -> Result<()> {
                #body
                Ok(())
            }
        }
    }).collect()
}

fn generate_role_body(protocol: &Protocol, role: &Role) -> TokenStream {
    generate_protocol_steps(protocol, role)
}

/// Generate code for protocol steps from the perspective of a specific role
fn generate_protocol_steps(protocol: &Protocol, role: &Role) -> TokenStream {
    match protocol {
        Protocol::End => {
            quote! {
                tracing::debug!("Protocol completed");
                Ok(())
            }
        }
        Protocol::Send { from, to, message, continuation } => {
            if from == role {
                // This role is sending
                let message_type = &message.name;
                let to_ident = &to.name;
                let continuation_code = generate_protocol_steps(continuation, role);
                
                quote! {
                    // Send message
                    let msg = #message_type::default(); // TODO: Proper message construction
                    handler.send(endpoint, Role::#to_ident, &msg).await?;
                    
                    #continuation_code
                }
            } else if to == role {
                // This role is receiving
                let message_type = &message.name;
                let from_ident = &from.name;
                let continuation_code = generate_protocol_steps(continuation, role);
                
                quote! {
                    // Receive message
                    let _msg: #message_type = handler.recv(endpoint, Role::#from_ident).await?;
                    
                    #continuation_code
                }
            } else {
                // This role is not involved in this step
                generate_protocol_steps(continuation, role)
            }
        }
        Protocol::Choice { role: choice_role, branches } => {
            if choice_role == role {
                // This role is making the choice
                let branch_code: Vec<TokenStream> = branches.iter().map(|branch| {
                    let label = &branch.label;
                    let body = generate_protocol_steps(&branch.protocol, role);
                    quote! {
                        #label => {
                            // Make choice
                            handler.choose(endpoint, role.clone(), Label::#label).await?;
                            #body
                        }
                    }
                }).collect();
                
                quote! {
                    // Make choice based on local decision
                    let choice = decide_choice(); // TODO: Implement choice logic
                    match choice {
                        #(#branch_code)*
                        _ => Err(ChoreographyError::InvalidChoice("Unknown choice".into()))?,
                    }
                }
            } else {
                // This role is offering/waiting for choice
                let branch_code: Vec<TokenStream> = branches.iter().map(|branch| {
                    let label = &branch.label;
                    let body = generate_protocol_steps(&branch.protocol, role);
                    quote! {
                        Label::#label => {
                            #body
                        }
                    }
                }).collect();
                
                let choice_role_name = &choice_role.name;
                quote! {
                    // Wait for choice from the choosing role
                    let offered_choice = handler.offer(endpoint, Role::#choice_role_name).await?;
                    match offered_choice {
                        #(#branch_code)*
                        _ => Err(ChoreographyError::InvalidChoice("Unexpected choice".into()))?,
                    }
                }
            }
        }
        Protocol::Loop { body, condition: _ } => {
            let body_code = generate_protocol_steps(body, role);
            quote! {
                // Loop until termination condition
                loop {
                    #body_code
                    // TODO: Add proper loop termination logic
                    break;
                }
            }
        }
        Protocol::Parallel { protocols } => {
            let parallel_code: Vec<TokenStream> = protocols.iter().map(|p| {
                generate_protocol_steps(p, role)
            }).collect();
            
            quote! {
                // Execute protocols in parallel
                // TODO: Implement proper parallel execution
                #(#parallel_code)*
            }
        }
        Protocol::Rec { label: _, body } => {
            // For simplicity, treat recursion as a simple body for now
            generate_protocol_steps(body, role)
        }
        Protocol::Broadcast { from, to_all: _, message, continuation } => {
            if from == role {
                // This role is broadcasting
                let message_type = &message.name;
                let continuation_code = generate_protocol_steps(continuation, role);
                
                quote! {
                    // Broadcast message to all roles
                    // TODO: Implement proper broadcast to all recipients
                    let msg = #message_type::default();
                    // For now, just skip broadcast implementation
                    tracing::debug!("Broadcast not fully implemented");
                    
                    #continuation_code
                }
            } else {
                // This role might be receiving the broadcast
                let message_type = &message.name;
                let from_ident = &from.name;
                let continuation_code = generate_protocol_steps(continuation, role);
                
                quote! {
                    // Receive broadcast message
                    let _msg: #message_type = handler.recv(endpoint, Role::#from_ident).await?;
                    
                    #continuation_code
                }
            }
        }
        Protocol::Var(_label) => {
            // Variable reference - would be resolved in a real implementation
            quote! {
                // Variable reference - would jump to recursive label
                tracing::debug!("Variable reference not implemented");
            }
        }
    }
}

fn infer_content_type(message_type: &str) -> TokenStream {
    // Simple heuristic - can be improved
    match message_type {
        s if s.contains("Request") => quote! { String },
        s if s.contains("Response") => quote! { String },
        s if s.contains("Data") => quote! { Vec<u8> },
        s if s.contains("Count") => quote! { u64 },
        s if s.contains("Flag") => quote! { bool },
        _ => quote! { String },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{Protocol, Role};
    
    #[test]
    fn test_generate_simple_protocol() {
        let choreography = Choreography {
            name: format_ident!("SimpleProtocol"),
            roles: vec![
                Role::new(format_ident!("Client")),
                Role::new(format_ident!("Server")),
            ],
            protocol: Protocol::End,
            attrs: std::collections::HashMap::new(),
        };
        
        let code = generate_effects_protocol(&choreography);
        let code_str = code.to_string();
        
        assert!(code_str.contains("enum Role"));
        assert!(code_str.contains("Client"));
        assert!(code_str.contains("Server"));
        assert!(code_str.contains("run_client"));
        assert!(code_str.contains("run_server"));
    }
}