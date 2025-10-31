// Comprehensive tests for the choreographic DSL parser

use rumpsteak_choreography::compiler::parser::{parse_choreography_str, ParseError};

#[test]
fn test_parse_simple_protocol() {
    let input = r#"
choreography PingPong {
    roles: Alice, Bob
    
    Alice -> Bob: Ping
    Bob -> Alice: Pong
}
"#;

    let result = parse_choreography_str(input);
    assert!(result.is_ok(), "Failed to parse: {:?}", result.err());

    let choreo = result.unwrap();
    assert_eq!(choreo.name.to_string(), "PingPong");
    assert_eq!(choreo.roles.len(), 2);
}

#[test]
fn test_parse_three_party_protocol() {
    let input = r#"
choreography ThreeParty {
    roles: Alice, Bob, Carol
    
    Alice -> Bob: Hello
    Bob -> Carol: Forward
    Carol -> Alice: Response
}
"#;

    let result = parse_choreography_str(input);
    assert!(result.is_ok());

    let choreo = result.unwrap();
    assert_eq!(choreo.roles.len(), 3);
}

#[test]
fn test_parse_broadcast() {
    let input = r#"
choreography Broadcast {
    roles: Leader, Worker1, Worker2
    
    Leader ->* : Start
}
"#;

    let result = parse_choreography_str(input);
    assert!(
        result.is_ok(),
        "Failed to parse broadcast: {:?}",
        result.err()
    );
}

#[test]
fn test_parse_choice_two_branches() {
    let input = r#"
choreography Choice {
    roles: A, B
    
    A -> B: Propose
    
    choice B {
        accept: {
            B -> A: Accept
        }
        reject: {
            B -> A: Reject
        }
    }
}
"#;

    let result = parse_choreography_str(input);
    assert!(result.is_ok());
}

#[test]
fn test_parse_choice_three_branches() {
    let input = r#"
choreography ThreeWayChoice {
    roles: Client, Server
    
    choice Client {
        get: {
            Client -> Server: Get
        }
        post: {
            Client -> Server: Post
        }
        delete: {
            Client -> Server: Delete
        }
    }
}
"#;

    let result = parse_choreography_str(input);
    assert!(result.is_ok());
}

#[test]
fn test_parse_nested_choice() {
    let input = r#"
choreography NestedChoice {
    roles: A, B, C
    
    choice A {
        path1: {
            A -> B: First
            choice B {
                inner1: {
                    B -> C: InnerA
                }
                inner2: {
                    B -> C: InnerB
                }
            }
        }
        path2: {
            A -> C: Second
        }
    }
}
"#;

    let result = parse_choreography_str(input);
    assert!(result.is_ok());
}

#[test]
fn test_parse_loop_with_count() {
    let input = r#"
choreography LoopCount {
    roles: Client, Server
    
    loop (count: 5) {
        Client -> Server: Request
        Server -> Client: Response
    }
}
"#;

    let result = parse_choreography_str(input);
    assert!(result.is_ok());
}

#[test]
fn test_parse_loop_with_role_decides() {
    let input = r#"
choreography LoopRoleDecides {
    roles: Client, Server
    
    loop (decides: Client) {
        Client -> Server: Request
        Server -> Client: Response
    }
}
"#;

    let result = parse_choreography_str(input);
    assert!(result.is_ok());
}

#[test]
fn test_parse_loop_with_custom_condition() {
    let input = r#"
choreography LoopCustom {
    roles: A, B
    
    loop (custom: "has_more_data") {
        A -> B: Data
    }
}
"#;

    let result = parse_choreography_str(input);
    assert!(result.is_ok());
}

#[test]
fn test_parse_loop_without_condition() {
    let input = r#"
choreography InfiniteLoop {
    roles: A, B
    
    loop {
        A -> B: Tick
    }
}
"#;

    let result = parse_choreography_str(input);
    assert!(result.is_ok());
}

#[test]
fn test_parse_parallel() {
    let input = r#"
choreography Parallel {
    roles: A, B, C, D
    
    parallel {
        A -> B: Msg1
    |
        C -> D: Msg2
    }
}
"#;

    let result = parse_choreography_str(input);
    assert!(result.is_ok());
}

#[test]
fn test_parse_recursive() {
    let input = r#"
choreography Recursive {
    roles: A, B
    
    rec Loop {
        A -> B: Data
    }
}
"#;

    let result = parse_choreography_str(input);
    assert!(result.is_ok());
}

#[test]
fn test_parse_complex_protocol() {
    let input = r#"
choreography ComplexProtocol {
    roles: Buyer, Seller, Shipper
    
    Buyer -> Seller: Inquiry
    Seller -> Buyer: Quote
    
    choice Buyer {
        order: {
            Buyer -> Seller: Order
            Seller -> Shipper: ShipRequest
            Shipper -> Buyer: Tracking
            
            loop (decides: Buyer) {
                Buyer -> Shipper: StatusCheck
                Shipper -> Buyer: StatusUpdate
            }
            
            Shipper -> Buyer: Delivered
            Buyer -> Seller: Confirmation
        }
        cancel: {
            Buyer -> Seller: Cancel
        }
    }
}
"#;

    let result = parse_choreography_str(input);
    assert!(
        result.is_ok(),
        "Failed to parse complex protocol: {:?}",
        result.err()
    );
}

#[test]
fn test_parse_with_payload() {
    let input = r#"
choreography WithPayload {
    roles: A, B
    
    A -> B: Message(data: String)
}
"#;

    let result = parse_choreography_str(input);
    assert!(result.is_ok());
}

// Error cases

#[test]
fn test_error_undefined_role_in_send() {
    let input = r#"
choreography Invalid {
    roles: Alice
    
    Alice -> Bob: Hello
}
"#;

    let result = parse_choreography_str(input);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(err, ParseError::UndefinedRole { ref role, .. } if role == "Bob"));

    // Verify error message includes span information
    let err_str = err.to_string();
    assert!(err_str.contains("Undefined role"));
    assert!(err_str.contains("Bob"));
    assert!(err_str.contains("-->")); // Rust-style error format
}

#[test]
fn test_error_undefined_role_in_choice() {
    let input = r#"
choreography Invalid {
    roles: Alice
    
    choice Bob {
        opt: {
            Alice -> Alice: Self
        }
    }
}
"#;

    let result = parse_choreography_str(input);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(err, ParseError::UndefinedRole { ref role, .. } if role == "Bob"));

    // Verify error message includes span information
    let err_str = err.to_string();
    assert!(err_str.contains("Undefined role"));
    assert!(err_str.contains("Bob"));
}

#[test]
fn test_error_undefined_role_in_loop_decides() {
    let input = r#"
choreography Invalid {
    roles: Alice
    
    loop (decides: Bob) {
        Alice -> Alice: Msg
    }
}
"#;

    let result = parse_choreography_str(input);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(err, ParseError::UndefinedRole { ref role, .. } if role == "Bob"));

    // Verify error message includes span information
    let err_str = err.to_string();
    assert!(err_str.contains("Undefined role"));
    assert!(err_str.contains("Bob"));
}

#[test]
fn test_error_duplicate_role() {
    let input = r#"
choreography Invalid {
    roles: Alice, Bob, Alice
    
    Alice -> Bob: Hello
}
"#;

    let result = parse_choreography_str(input);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(err, ParseError::DuplicateRole { ref role, .. } if role == "Alice"));

    // Verify error message includes span information
    let err_str = err.to_string();
    assert!(err_str.contains("Duplicate role"));
    assert!(err_str.contains("Alice"));
}

#[test]
fn test_error_no_roles() {
    let input = r#"
choreography Invalid {
    Alice -> Bob: Hello
}
"#;

    let result = parse_choreography_str(input);
    assert!(result.is_err());
}

#[test]
fn test_error_invalid_syntax() {
    let input = r#"
choreography Invalid {
    roles: A, B
    
    A -> -> B: Hello
}
"#;

    let result = parse_choreography_str(input);
    assert!(result.is_err());
}

#[test]
fn test_parse_with_comments() {
    let input = r#"
// This is a comment
choreography CommentTest {
    roles: Alice, Bob  // Inline comment
    
    /* Multi-line
       comment */
    Alice -> Bob: Hello
}
"#;

    let result = parse_choreography_str(input);
    assert!(
        result.is_ok(),
        "Failed to parse with comments: {:?}",
        result.err()
    );
}

#[test]
fn test_parse_whitespace_variations() {
    let input = r#"
choreography   WhitespaceTest   {
    roles:Alice,Bob
    Alice->Bob:Hello
}
"#;

    let result = parse_choreography_str(input);
    assert!(result.is_ok());
}

#[test]
fn test_empty_protocol_body() {
    let input = r#"
choreography Empty {
    roles: A, B
}
"#;

    let result = parse_choreography_str(input);
    assert!(result.is_ok());
}

#[test]
fn test_parse_role_names_with_underscores() {
    let input = r#"
choreography UnderscoreRoles {
    roles: Alice_Client, Bob_Server
    
    Alice_Client -> Bob_Server: Request
}
"#;

    let result = parse_choreography_str(input);
    assert!(result.is_ok());
}

#[test]
fn test_parse_role_names_with_numbers() {
    let input = r#"
choreography NumericRoles {
    roles: Worker1, Worker2, Worker3
    
    Worker1 -> Worker2: Data
    Worker2 -> Worker3: Forward
}
"#;

    let result = parse_choreography_str(input);
    assert!(result.is_ok());
}

#[test]
fn test_parse_sequence_of_sends() {
    let input = r#"
choreography Sequence {
    roles: A, B, C, D
    
    A -> B: Msg1
    B -> C: Msg2
    C -> D: Msg3
    D -> A: Msg4
}
"#;

    let result = parse_choreography_str(input);
    assert!(result.is_ok());
}

#[test]
fn test_integration_with_projection() {
    use rumpsteak_choreography::compiler::projection;

    let input = r#"
choreography TwoParty {
    roles: Client, Server
    
    Client -> Server: Request
    Server -> Client: Response
}
"#;

    let choreo = parse_choreography_str(input).expect("Failed to parse");

    // Test that we can project this choreography
    for role in &choreo.roles {
        let result = projection::project(&choreo, role);
        assert!(
            result.is_ok(),
            "Failed to project for role {}: {:?}",
            role.name,
            result.err()
        );
    }
}

#[test]
fn test_integration_validation() {
    let input = r#"
choreography ValidProtocol {
    roles: A, B
    
    A -> B: Hello
    B -> A: World
}
"#;

    let choreo = parse_choreography_str(input).expect("Failed to parse");

    // Test that the choreography validates
    let validation_result = choreo.validate();
    assert!(
        validation_result.is_ok(),
        "Validation failed: {:?}",
        validation_result.err()
    );
}

#[test]
fn test_error_message_quality() {
    // This test verifies that error messages include helpful span information
    let input = r#"
choreography Example {
    roles: Alice, Bob
    
    Alice -> Charlie: Hello
}
"#;

    let result = parse_choreography_str(input);
    assert!(result.is_err());

    let err = result.unwrap_err();
    let err_msg = err.to_string();

    // Should include error type
    assert!(err_msg.contains("Undefined role"));

    // Should include the offending role name
    assert!(err_msg.contains("Charlie"));

    // Should include location information
    assert!(err_msg.contains("-->"));

    // Should include line and column numbers
    assert!(err_msg.contains("5:"));

    // Should include code snippet
    assert!(err_msg.contains("Alice -> Charlie: Hello"));

    // Should have visual indicator (underline)
    assert!(err_msg.contains("^"));
}

#[test]
fn test_error_span_precision() {
    // Test that the span precisely identifies the error location
    let input = r#"
choreography Test {
    roles: Alice, Bob
    
    Alice -> UnknownRole: Message
    Bob -> Alice: Response
}
"#;

    let result = parse_choreography_str(input);
    assert!(result.is_err());

    let err = result.unwrap_err();
    let err_msg = err.to_string();

    // Should point to the specific undefined role
    assert!(err_msg.contains("UnknownRole"));

    // Should show the line with the error
    assert!(err_msg.contains("Alice -> UnknownRole: Message"));
}

// Protocol composition tests

#[test]
fn test_parse_protocol_composition_simple() {
    let input = r#"
choreography CompositionExample {
    roles: Alice, Bob
    
    protocol Handshake {
        Alice -> Bob: Hello
        Bob -> Alice: Hi
    }
    
    call Handshake
    Alice -> Bob: Data
}
"#;

    let result = parse_choreography_str(input);
    assert!(
        result.is_ok(),
        "Failed to parse protocol composition: {:?}",
        result.err()
    );

    let choreo = result.unwrap();
    assert_eq!(choreo.name.to_string(), "CompositionExample");
}

#[test]
fn test_parse_protocol_composition_multiple_defs() {
    let input = r#"
choreography MultipleProtocols {
    roles: A, B, C
    
    protocol Step1 {
        A -> B: Start
    }
    
    protocol Step2 {
        B -> C: Continue
    }
    
    protocol Step3 {
        C -> A: Finish
    }
    
    call Step1
    call Step2
    call Step3
}
"#;

    let result = parse_choreography_str(input);
    assert!(
        result.is_ok(),
        "Failed to parse multiple protocols: {:?}",
        result.err()
    );
}

#[test]
fn test_parse_protocol_composition_nested_calls() {
    let input = r#"
choreography NestedCalls {
    roles: Alice, Bob
    
    protocol Inner {
        Alice -> Bob: Data1
        Bob -> Alice: Ack1
    }
    
    protocol Outer {
        call Inner
        Alice -> Bob: Data2
    }
    
    call Outer
}
"#;

    let result = parse_choreography_str(input);
    assert!(
        result.is_ok(),
        "Failed to parse nested calls: {:?}",
        result.err()
    );
}

#[test]
fn test_parse_protocol_composition_in_choice() {
    let input = r#"
choreography CallInChoice {
    roles: Client, Server
    
    protocol Success {
        Server -> Client: Success
    }
    
    protocol Failure {
        Server -> Client: Failure
    }
    
    Client -> Server: Request
    
    choice Server {
        ok: {
            call Success
        }
        error: {
            call Failure
        }
    }
}
"#;

    let result = parse_choreography_str(input);
    assert!(
        result.is_ok(),
        "Failed to parse call in choice: {:?}",
        result.err()
    );
}

#[test]
fn test_parse_protocol_composition_in_loop() {
    let input = r#"
choreography CallInLoop {
    roles: A, B
    
    protocol Exchange {
        A -> B: Request
        B -> A: Response
    }
    
    loop (count: 3) {
        call Exchange
    }
}
"#;

    let result = parse_choreography_str(input);
    assert!(
        result.is_ok(),
        "Failed to parse call in loop: {:?}",
        result.err()
    );
}

#[test]
fn test_error_undefined_protocol_call() {
    let input = r#"
choreography UndefinedCall {
    roles: A, B
    
    call NonExistent
}
"#;

    let result = parse_choreography_str(input);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(
        err,
        rumpsteak_choreography::compiler::parser::ParseError::UndefinedProtocol { .. }
    ));

    let err_msg = err.to_string();
    assert!(err_msg.contains("Undefined protocol"));
    assert!(err_msg.contains("NonExistent"));
}

#[test]
fn test_error_duplicate_protocol_def() {
    let input = r#"
choreography DuplicateDef {
    roles: A, B
    
    protocol MyProtocol {
        A -> B: First
    }
    
    protocol MyProtocol {
        A -> B: Second
    }
}
"#;

    let result = parse_choreography_str(input);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(
        err,
        rumpsteak_choreography::compiler::parser::ParseError::DuplicateProtocol { .. }
    ));

    let err_msg = err.to_string();
    assert!(err_msg.contains("Duplicate protocol"));
    assert!(err_msg.contains("MyProtocol"));
}

// Guard tests

#[test]
fn test_parse_choice_with_guard() {
    let input = r#"
choreography GuardExample {
    roles: Client, Server
    
    choice Client {
        buy when (balance > price): {
            Client -> Server: Purchase
        }
        cancel: {
            Client -> Server: Cancel
        }
    }
}
"#;

    let result = parse_choreography_str(input);
    assert!(
        result.is_ok(),
        "Failed to parse choice with guard: {:?}",
        result.err()
    );
}

#[test]
fn test_parse_choice_with_multiple_guards() {
    let input = r#"
choreography MultiGuards {
    roles: A, B
    
    choice A {
        option1 when (x > 0): {
            A -> B: Msg1
        }
        option2 when (x < 0): {
            A -> B: Msg2
        }
        option3: {
            A -> B: Msg3
        }
    }
}
"#;

    let result = parse_choreography_str(input);
    assert!(
        result.is_ok(),
        "Failed to parse multiple guards: {:?}",
        result.err()
    );
}

#[test]
fn test_parse_guard_with_complex_expression() {
    let input = r#"
choreography ComplexGuard {
    roles: Client, Server
    
    choice Client {
        proceed when (balance >= price && is_authenticated): {
            Client -> Server: Action
        }
        reject: {
            Client -> Server: Reject
        }
    }
}
"#;

    let result = parse_choreography_str(input);
    assert!(
        result.is_ok(),
        "Failed to parse complex guard: {:?}",
        result.err()
    );
}

#[test]
fn test_parse_guard_in_nested_choice() {
    let input = r#"
choreography NestedGuard {
    roles: A, B, C
    
    choice A {
        outer when (condition1): {
            A -> B: Start
            choice B {
                inner when (condition2): {
                    B -> C: Inner
                }
                fallback: {
                    B -> C: Fallback
                }
            }
        }
        skip: {
            A -> C: Skip
        }
    }
}
"#;

    let result = parse_choreography_str(input);
    assert!(
        result.is_ok(),
        "Failed to parse nested guard: {:?}",
        result.err()
    );
}

// ============================================================================
// Annotation Tests (Performance Hints & Verification)
// ============================================================================

#[test]
fn test_parse_simple_annotation() {
    let input = r#"
@optimize
choreography Simple {
    roles: A, B
    
    A -> B: Msg
}
"#;

    let result = parse_choreography_str(input);
    assert!(
        result.is_ok(),
        "Failed to parse simple annotation: {:?}",
        result.err()
    );

    let choreo = result.unwrap();
    assert!(choreo.attrs.contains_key("optimize"));
    assert_eq!(choreo.attrs.get("optimize").unwrap(), "true");
}

#[test]
fn test_parse_annotation_with_args() {
    let input = r#"
@optimize(inline, buffer_size=1024)
choreography WithArgs {
    roles: A, B
    
    A -> B: Msg
}
"#;

    let result = parse_choreography_str(input);
    assert!(
        result.is_ok(),
        "Failed to parse annotation with args: {:?}",
        result.err()
    );

    let choreo = result.unwrap();
    assert!(choreo.attrs.contains_key("optimize"));
    let value = choreo.attrs.get("optimize").unwrap();
    assert!(value.contains("inline"));
    assert!(value.contains("buffer_size=1024"));
}

#[test]
fn test_parse_multiple_annotations() {
    let input = r#"
@optimize(inline)
@verify(deadlock_free)
@parallel
choreography MultiAnnotated {
    roles: A, B, C
    
    A -> B: Msg1
    B -> C: Msg2
}
"#;

    let result = parse_choreography_str(input);
    assert!(
        result.is_ok(),
        "Failed to parse multiple annotations: {:?}",
        result.err()
    );

    let choreo = result.unwrap();
    assert!(choreo.attrs.contains_key("optimize"));
    assert!(choreo.attrs.contains_key("verify"));
    assert!(choreo.attrs.contains_key("parallel"));
}

#[test]
fn test_parse_statement_annotations() {
    let input = r#"
choreography StatementAnnotations {
    roles: A, B
    
    @critical
    A -> B: ImportantMsg
    
    @buffered(size=10)
    B -> A: Response
}
"#;

    let result = parse_choreography_str(input);
    assert!(
        result.is_ok(),
        "Failed to parse statement annotations: {:?}",
        result.err()
    );
}

#[test]
fn test_parse_verification_annotation() {
    let input = r#"
@verify(deadlock_free, liveness)
choreography Verified {
    roles: A, B
    
    A -> B: Start
    B -> A: Ack
}
"#;

    let result = parse_choreography_str(input);
    assert!(
        result.is_ok(),
        "Failed to parse verification annotation: {:?}",
        result.err()
    );

    let choreo = result.unwrap();
    assert!(choreo.attrs.contains_key("verify"));
    let value = choreo.attrs.get("verify").unwrap();
    assert!(value.contains("deadlock_free"));
    assert!(value.contains("liveness"));
}

// ============================================================================
// Type Annotation Tests
// ============================================================================

#[test]
fn test_parse_message_with_type() {
    let input = r#"
choreography TypedMessages {
    roles: A, B
    
    A -> B: Request<String>
    B -> A: Response<i32>
}
"#;

    let result = parse_choreography_str(input);
    assert!(
        result.is_ok(),
        "Failed to parse typed messages: {:?}",
        result.err()
    );
}

#[test]
fn test_parse_message_with_multiple_types() {
    let input = r#"
choreography MultiTyped {
    roles: A, B
    
    A -> B: Data<String, i32, bool>
}
"#;

    let result = parse_choreography_str(input);
    assert!(
        result.is_ok(),
        "Failed to parse multi-typed message: {:?}",
        result.err()
    );
}

#[test]
fn test_parse_message_with_generic_types() {
    let input = r#"
choreography Generics {
    roles: A, B
    
    A -> B: Container<Vec<String>>
    B -> A: Result<i32, Error>
}
"#;

    let result = parse_choreography_str(input);
    assert!(
        result.is_ok(),
        "Failed to parse generic types: {:?}",
        result.err()
    );
}

#[test]
fn test_parse_message_with_type_and_payload() {
    let input = r#"
choreography TypedWithPayload {
    roles: A, B
    
    A -> B: Request<String>(data)
    B -> A: Response<i32>(result)
}
"#;

    let result = parse_choreography_str(input);
    assert!(
        result.is_ok(),
        "Failed to parse typed message with payload: {:?}",
        result.err()
    );
}

#[test]
fn test_parse_message_with_path_types() {
    let input = r#"
choreography PathTypes {
    roles: A, B
    
    A -> B: Data<std::string::String>
    B -> A: Result<std::vec::Vec<i32>>
}
"#;

    let result = parse_choreography_str(input);
    assert!(
        result.is_ok(),
        "Failed to parse path types: {:?}",
        result.err()
    );
}

// ============================================================================
// Parameterized Roles Tests
// ============================================================================

#[test]
fn test_parse_parameterized_role() {
    let input = r#"
choreography WorkerPool {
    roles: Master, Worker[N]
    
    Master -> Worker[0]: Task
}
"#;

    let result = parse_choreography_str(input);
    assert!(
        result.is_ok(),
        "Failed to parse parameterized role: {:?}",
        result.err()
    );

    let choreo = result.unwrap();
    assert_eq!(choreo.roles.len(), 2);
    assert!(choreo.roles[1].is_parameterized());
}

#[test]
fn test_parse_concrete_indexed_role() {
    let input = r#"
choreography IndexedWorkers {
    roles: Master, Worker[3]
    
    Master -> Worker[0]: Task1
    Master -> Worker[1]: Task2
    Master -> Worker[2]: Task3
}
"#;

    let result = parse_choreography_str(input);
    assert!(
        result.is_ok(),
        "Failed to parse indexed roles: {:?}",
        result.err()
    );
}

#[test]
fn test_parse_multiple_parameterized_roles() {
    let input = r#"
choreography MultiParam {
    roles: Coordinator, Worker[N], Monitor[M]
    
    Coordinator -> Worker[i]: Start
    Worker[i] -> Monitor[j]: Report
}
"#;

    let result = parse_choreography_str(input);
    assert!(
        result.is_ok(),
        "Failed to parse multiple parameterized roles: {:?}",
        result.err()
    );

    let choreo = result.unwrap();
    assert_eq!(choreo.roles.len(), 3);
}

#[test]
fn test_parse_parameterized_role_in_choice() {
    let input = r#"
choreography ParameterizedChoice {
    roles: Master, Worker[N]
    
    choice Master {
        assign: {
            Master -> Worker[i]: Task
        }
        skip: {
            Master -> Worker[0]: Skip
        }
    }
}
"#;

    let result = parse_choreography_str(input);
    assert!(
        result.is_ok(),
        "Failed to parse parameterized role in choice: {:?}",
        result.err()
    );
}

#[test]
fn test_parse_parameterized_role_loop() {
    let input = r#"
choreography ParameterizedLoop {
    roles: Master, Worker[N]
    
    loop (count: N) {
        Master -> Worker[i]: Work
        Worker[i] -> Master: Result
    }
}
"#;

    let result = parse_choreography_str(input);
    assert!(
        result.is_ok(),
        "Failed to parse parameterized role in loop: {:?}",
        result.err()
    );
}
