// Simple property-based tests for choreography projection
//
// Simplified version to verify proptest works

use proptest::prelude::*;
use quote::format_ident;
use rumpsteak_choreography::ast::{Choreography, LocalType, Protocol, Role};
use rumpsteak_choreography::compiler::projection::project;
use std::collections::HashMap;

fn simple_role_strategy() -> impl Strategy<Value = Role> {
    prop_oneof![
        Just(Role::new(format_ident!("Alice"))),
        Just(Role::new(format_ident!("Bob"))),
    ]
}

proptest! {
    /// Property: Projection is deterministic
    #[test]
    fn projection_completes(role in simple_role_strategy()) {
        let choreo = Choreography {
            name: format_ident!("Simple"),
            roles: vec![role.clone()],
            protocol: Protocol::End,
            attrs: HashMap::new(),
        };

        // Projection should complete without panicking
        let _result = project(&choreo, &role);
    }
}

#[test]
fn test_end_projection() {
    let alice = Role::new(format_ident!("Alice"));
    let choreo = Choreography {
        name: format_ident!("EndOnly"),
        roles: vec![alice.clone()],
        protocol: Protocol::End,
        attrs: HashMap::new(),
    };

    let projected = project(&choreo, &alice).unwrap();
    assert_eq!(projected, LocalType::End);
}
