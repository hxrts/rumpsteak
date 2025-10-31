// Code generation from projected local types to Rumpsteak session types

use crate::ast::{LocalType, Role, MessageType};
use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};

/// Generate Rumpsteak session type definitions from a local type
pub fn generate_session_type(
    role: &Role,
    local_type: &LocalType,
    protocol_name: &str,
) -> TokenStream {
    let type_name = format_ident!("{}_{}", role.name, protocol_name);
    let inner_type = generate_type_expr(local_type);
    
    quote! {
        #[session]
        type #type_name = #inner_type;
    }
}

/// Generate the type expression for a local type
fn generate_type_expr(local_type: &LocalType) -> TokenStream {
    match local_type {
        LocalType::Send { to, message, continuation } => {
            let to_name = &to.name;
            let msg_name = &message.name;
            let cont = generate_type_expr(continuation);
            
            quote! {
                Send<#to_name, #msg_name, #cont>
            }
        }
        
        LocalType::Receive { from, message, continuation } => {
            let from_name = &from.name;
            let msg_name = &message.name;
            let cont = generate_type_expr(continuation);
            
            quote! {
                Receive<#from_name, #msg_name, #cont>
            }
        }
        
        LocalType::Select { to, branches } => {
            let to_name = &to.name;
            let choice_type = generate_choice_enum(branches, true);
            
            quote! {
                Select<#to_name, #choice_type>
            }
        }
        
        LocalType::Branch { from, branches } => {
            let from_name = &from.name;
            let choice_type = generate_choice_enum(branches, false);
            
            quote! {
                Branch<#from_name, #choice_type>
            }
        }
        
        LocalType::Rec { label: _label, body } => {
            // Generate a recursive type using the label as the type name
            // This prevents infinite expansion by creating a named recursive type
            let body_expr = generate_type_expr(body);
            quote! {
                // Recursive type
                #body_expr
            }
        }
        
        LocalType::Var(label) => {
            // Reference to recursive type variable
            // Refers back to the enclosing Rec label
            // Inlined reference for code generation
            quote! { #label }
        }
        
        LocalType::End => {
            quote! { End }
        }
    }
}

/// Generate a choice enum for Select/Branch
fn generate_choice_enum(
    branches: &[(Ident, LocalType)],
    _is_select: bool,
) -> TokenStream {
    let enum_name = format_ident!("Choice{}", 
        branches.iter().map(|(l, _)| l.to_string()).collect::<String>()
    );
    
    let variants: Vec<TokenStream> = branches.iter().map(|(label, local_type)| {
        let continuation = generate_type_expr(local_type);
        quote! {
            #label(#label, #continuation)
        }
    }).collect();
    
    quote! {
        {
            #[session]
            enum #enum_name {
                #(#variants),*
            }
            #enum_name
        }
    }
}

/// Generate complete Rumpsteak code from a choreography
pub fn generate_choreography_code(
    name: &str,
    roles: &[Role],
    local_types: &[(Role, LocalType)],
) -> TokenStream {
    let role_struct_defs = generate_role_structs(roles);
    let session_type_defs = local_types.iter().map(|(role, local_type)| {
        generate_session_type(role, local_type, name)
    });
    
    quote! {
        #role_struct_defs
        #(#session_type_defs)*
    }
}

/// Generate role struct definitions
fn generate_role_structs(roles: &[Role]) -> TokenStream {
    let _n = roles.len();
    let role_names: Vec<&Ident> = roles.iter().map(|r| &r.name).collect();
    
    // Generate Roles tuple struct
    let roles_struct = quote! {
        #[derive(Roles)]
        struct Roles(#(#role_names),*);
    };
    
    // Generate individual role structs with routes
    let role_structs = roles.iter().enumerate().map(|(i, role)| {
        let role_name = &role.name;
        let other_roles: Vec<_> = roles.iter()
            .enumerate()
            .filter(|(j, _)| i != *j)
            .map(|(_, r)| &r.name)
            .collect();
        
        if other_roles.is_empty() {
            // Single role (unusual but possible)
            quote! {
                #[derive(Role)]
                #[message(Label)]
                struct #role_name;
            }
        } else {
            let routes = other_roles.iter().map(|other| {
                quote! {
                    #[route(#other)] Channel
                }
            });
            
            quote! {
                #[derive(Role)]
                #[message(Label)]
                struct #role_name(#(#routes),*);
            }
        }
    });
    
    quote! {
        #roles_struct
        #(#role_structs)*
    }
}

/// Generate implementation functions for each role
pub fn generate_role_implementations(
    role: &Role,
    local_type: &LocalType,
    protocol_name: &str,
) -> TokenStream {
    let role_name = &role.name;
    let fn_name = format_ident!("{}_protocol", role_name.to_string().to_lowercase());
    let session_type = format_ident!("{}_{}", role_name, protocol_name);
    
    let impl_body = generate_implementation_body(local_type);
    
    quote! {
        async fn #fn_name(role: &mut #role_name) -> Result<()> {
            try_session(role, |s: #session_type<'_, _>| async move {
                #impl_body
                Ok(((), s))
            }).await
        }
    }
}

/// Generate the implementation body for a local type
fn generate_implementation_body(local_type: &LocalType) -> TokenStream {
    match local_type {
        LocalType::Send { message, continuation, .. } => {
            let msg_name = &message.name;
            let cont_impl = generate_implementation_body(continuation);
            
            quote! {
                let s = s.send(#msg_name(/* ... */)).await?;
                #cont_impl
            }
        }
        
        LocalType::Receive { message, continuation, .. } => {
            let msg_name = &message.name;
            let cont_impl = generate_implementation_body(continuation);
            
            quote! {
                let (#msg_name(value), s) = s.receive().await?;
                #cont_impl
            }
        }
        
        LocalType::Select { branches, .. } => {
            // Generate match on user choice
            let first_branch = &branches[0];
            let label = &first_branch.0;
            let cont_impl = generate_implementation_body(&first_branch.1);
            
            quote! {
                let s = s.select(#label(/* ... */)).await?;
                #cont_impl
            }
        }
        
        LocalType::Branch { branches, .. } => {
            let match_arms = branches.iter().map(|(label, local_type)| {
                let impl_body = generate_implementation_body(local_type);
                quote! {
                    Choice::#label(value, s) => {
                        #impl_body
                    }
                }
            });
            
            quote! {
                let s = match s.branch().await? {
                    #(#match_arms)*
                };
            }
        }
        
        LocalType::End => quote! {},
        
        _ => quote! { /* recursive types need special handling */ },
    }
}

/// Generate helper functions and types for the choreography
pub fn generate_helpers(_name: &str, messages: &[MessageType]) -> TokenStream {
    let message_enum = if !messages.is_empty() {
        let variants = messages.iter().map(|msg| {
            let name = &msg.name;
            quote! { #name(#name) }
        });
        
        quote! {
            #[derive(Message)]
            enum Label {
                #(#variants),*
            }
        }
    } else {
        quote! {}
    };
    
    let message_structs = messages.iter().map(|msg| {
        let name = &msg.name;
        if let Some(payload) = &msg.payload {
            quote! { struct #name #payload; }
        } else {
            quote! { struct #name; }
        }
    });
    
    quote! {
        #message_enum
        #(#message_structs)*
        
        type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
        type Channel = Bidirectional<UnboundedSender<Label>, UnboundedReceiver<Label>>;
    }
}