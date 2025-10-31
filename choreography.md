# Rumpsteak Choreographic Programming Refactor Proposal

## Overview

This proposal outlines a refactoring of Rumpsteak to enable writing global choreographic protocols declaratively. The goal is enable specifying distributed protocols from a global viewpoint, with automatic projection to local session types for each participant.

## Proposed Architecture

### 1. Choreographic DSL

The choreographic DSL provides a declarative syntax for specifying global protocols using a `#[choreography]` macro. This high-level syntax allows developers to express multi-party protocols naturally, focusing on the communication patterns rather than individual role implementations.

```rust
// New macro for defining global choreographies
#[choreography]
protocol Adder {
    roles: Client, Server;
    
    Client -> Server: Hello(i32);
    
    loop {
        choice Client {
            Add => {
                Client -> Server: Add(i32);
                Client -> Server: Add(i32);
                Server -> Client: Sum(i32);
            }
            Bye => {
                Client -> Server: Bye;
                Server -> Client: Bye;
                break;
            }
        }
    }
}
```

### 2. Choreography Language Features

The choreography language supports a rich set of features for expressing complex protocol patterns. These primitives cover the full spectrum of distributed communication patterns including message passing, conditional branching, iteration, parallel composition, and recursion.

#### a. Message Passing
```rust
A -> B: MessageType(payload);
```

#### b. Choice/Branching
```rust
choice A {
    Label1 => { ... }
    Label2 => { ... }
}
```

#### c. Loops
```rust
loop {
    // protocol steps
    if condition { break; }
}
```

#### d. Parallel Composition
```rust
par {
    A -> B: Msg1;
} and {
    C -> D: Msg2;
}
```

#### e. Recursion
```rust
rec X {
    A -> B: Message;
    choice A {
        Continue => X;
        Stop => { /* end */ }
    }
}
```

### 3. Implementation Structure

The choreographic programming system is now implemented as a separate crate (`rumpsteak-choreography`) that provides a clean layer on top of the core Rumpsteak session types. The system is organized into distinct modules that separate concerns and enable clean architecture.

```rust
// Crate structure
rumpsteak-choreography/
├── src/
│   ├── lib.rs          // Public API
│   ├── ast/            // AST and language constructs
│   │   ├── mod.rs      // Core types: Choreography, Protocol, Role
│   │   ├── protocol.rs // Protocol-specific extensions
│   │   └── role.rs     // Role and message type utilities
│   ├── compiler/       // Compilation pipeline
│   │   ├── mod.rs
│   │   ├── parser.rs   // Parse choreography DSL
│   │   ├── analysis.rs // Static analysis
│   │   ├── projection.rs // Project to local types
│   │   ├── codegen.rs  // Generate session types
│   │   └── effects_codegen.rs // Generate effect-based code
│   └── effects/        // Effect system
│       ├── mod.rs
│       ├── handler.rs  // Core effect traits
│       ├── middleware.rs // Composable middleware (Trace, Metrics, Retry)
│       └── impls.rs    // Handler implementations (Rumpsteak, InMemory, Recording)
```

### 4. Key Components

Each component below plays a role in transforming global choreographies into executable session-typed code while preserving correctness guarantees.

#### a. Choreography AST (`rumpsteak-choreography/src/ast/mod.rs`)
```rust
#[derive(Debug, Clone)]
pub struct Choreography {
    pub name: Ident,
    pub roles: Vec<Role>,
    pub protocol: Protocol,
}

#[derive(Debug, Clone)]
pub enum Protocol {
    Send {
        from: Role,
        to: Role,
        message: MessageType,
        continuation: Box<Protocol>,
    },
    Choice {
        role: Role,
        branches: Vec<(Label, Protocol)>,
    },
    Loop {
        body: Box<Protocol>,
    },
    Parallel {
        protocols: Vec<Protocol>,
    },
    Rec {
        name: Ident,
        body: Box<Protocol>,
    },
    End,
}
```

#### b. Projection Algorithm (`rumpsteak-choreography/src/compiler/projection.rs`)
```rust
pub fn project(choreography: &Choreography, role: &Role) -> LocalType {
    match &choreography.protocol {
        Protocol::Send { from, to, message, continuation } => {
            if role == from {
                LocalType::Send {
                    to: to.clone(),
                    message: message.clone(),
                    continuation: Box::new(project_continuation(continuation, role)),
                }
            } else if role == to {
                LocalType::Receive {
                    from: from.clone(),
                    message: message.clone(),
                    continuation: Box::new(project_continuation(continuation, role)),
                }
            } else {
                project_continuation(continuation, role)
            }
        }
        // ... other cases
    }
}
```

#### c. Code Generation (`rumpsteak-choreography/src/compiler/codegen.rs`)
```rust
pub fn generate_session_types(choreography: &Choreography) -> TokenStream {
    let mut types = TokenStream::new();
    
    for role in &choreography.roles {
        let local_type = project(choreography, role);
        let session_type = generate_type(local_type);
        types.extend(session_type);
    }
    
    types
}
```

### 5. Integration with Existing Rumpsteak

The choreographic layer is designed to be compatible with Rumpsteak's existing session type infrastructure. Rather than replacing the current system, choreographies compile down to standard Rumpsteak session types, allowing seamless interoperation with existing code.

The choreography system generates standard Rumpsteak session types:

```rust
// Input: Choreographic protocol
#[choreography]
protocol Simple {
    roles: A, B;
    A -> B: Hello(String);
    B -> A: World(String);
}

// Generated output (automatic):
#[session]
type A_Protocol = Send<B, Hello, Receive<B, World, End>>;

#[session]
type B_Protocol = Receive<A, Hello, Send<A, World, End>>;
```

### 6. Additional Features

Beyond basic choreographic primitives, the system supports features for expressing protocol patterns. This enables parameterized protocols, runtime refinements, and reusable choreographic functions that can be composed into larger protocols.

#### a. Parameterized Choreographies
```rust
#[choreography]
protocol MultiParty<const N: usize> {
    roles: Leader, Follower[N];
    
    for i in 0..N {
        Leader -> Follower[i]: Task(i);
        Follower[i] -> Leader: Result(i);
    }
}
```

#### b. Assertions and Refinements
```rust
#[choreography]
protocol SecureTransfer {
    roles: Client, Server;
    
    Client -> Server: Request(amount: u64)
        where { amount > 0 && amount <= 1000 };
    
    Server -> Client: Response(approved: bool);
}
```

#### c. Choreographic Functions
```rust
#[choreography]
fn broadcast<T>(sender: Role, receivers: Vec<Role>, msg: T) {
    for receiver in receivers {
        sender -> receiver: msg.clone();
    }
}

#[choreography]
protocol UsesBroadcast {
    roles: Master, Worker1, Worker2, Worker3;
    
    broadcast(Master, vec![Worker1, Worker2, Worker3], StartTask);
}
```

### 7. Static Analysis

The choreography system includes static analysis capabilities to verify protocol properties before code generation. These analyses catch common distributed system errors at compile time, providing stronger correctness guarantees than traditional approaches.

#### a. Deadlock Freedom
- Automatically verify absence of circular dependencies
- Check for proper session termination

#### b. Progress Guarantee
- Ensure all branches lead to termination or recursion
- Verify no infinite loops without communication

#### c. Role Coverage
- Ensure all roles participate meaningfully
- Detect unused roles

### 8. Effect Handler Architecture

The choreographic system now includes an effect handler abstraction that decouples protocol logic from transport implementation. This clean separation enables testable, composable, and runtime-agnostic protocol implementations while maintaining the choreographic programming model.

#### a. Effect Handler Trait (`rumpsteak-choreography/src/effects/handler.rs`)
```rust
#[async_trait]
pub trait ChoreoHandler: Send {
    type Role: RoleId;
    type Endpoint: Endpoint;

    async fn send<M: Serialize + Send + Sync>(
        &mut self, ep: &mut Self::Endpoint, to: Self::Role, msg: &M
    ) -> Result<()>;

    async fn recv<M: DeserializeOwned + Send>(
        &mut self, ep: &mut Self::Endpoint, from: Self::Role
    ) -> Result<M>;

    async fn choose(
        &mut self, ep: &mut Self::Endpoint, who: Self::Role, label: Label
    ) -> Result<()>;

    async fn offer(
        &mut self, ep: &mut Self::Endpoint, from: Self::Role
    ) -> Result<Label>;
}
```

#### b. Composable Middleware (`rumpsteak-choreography/src/effects/middleware.rs`)
```rust
// Tracing middleware for observability
pub struct Trace<H> {
    inner: H,
    prefix: String,
}

// Metrics collection middleware
pub struct Metrics<H> {
    inner: H,
    send_count: AtomicU64,
    recv_count: AtomicU64,
}

// Retry middleware with exponential backoff
pub struct Retry<H> {
    inner: H,
    max_retries: usize,
    base_delay: Duration,
}
```

#### c. Multiple Transport Implementations (`rumpsteak-choreography/src/effects/impls.rs`)
```rust
// Production: Rumpsteak session-typed channels
pub struct RumpsteakHandler<R, M> { ... }

// Testing: In-memory channels
pub struct InMemoryHandler<R: RoleId> { ... }

// Verification: Event recording for testing
pub struct RecordingHandler<R: RoleId> { ... }

// Stub: No-op for pure logic testing
pub struct NoOpHandler<R: RoleId> { ... }
```

#### d. Generated Protocol Code (`rumpsteak-choreography/src/compiler/effects_codegen.rs`)
```rust
// Generated from choreography - uses effect handlers
pub async fn run_client<H: ChoreoHandler<Role = Role, Endpoint = ClientEndpoint>>(
    handler: &mut H,
    endpoint: &mut ClientEndpoint,
) -> Result<()> {
    // Send request
    handler.send(endpoint, Role::Server, &Request("hello".into())).await?;
    
    // Receive response
    let response: Response = handler.recv(endpoint, Role::Server).await?;
    
    Ok(())
}
```
