# Getting Started

## Installation

Add Rumpsteak to your project (using the Aura fork):

```toml
[dependencies]
rumpsteak-aura = { git = "https://github.com/aura-project/rumpsteak-aura" }
rumpsteak-choreography = { git = "https://github.com/aura-project/rumpsteak-aura" }
```

For WASM support, add the wasm feature:

```toml
rumpsteak-choreography = { git = "https://github.com/aura-project/rumpsteak-aura", features = ["wasm"] }
```

## Creating a Choreography

This example shows a simple ping-pong protocol between two roles.

Define the choreography using the macro:

```rust
use rumpsteak_choreography::choreography;

choreography! {
    PingPong {
        roles: Alice, Bob
        Alice -> Bob: Ping
        Bob -> Alice: Pong
    }
}
```

The macro generates role types, message types, and session types automatically.

Run the protocol using the effect system:

```rust
use rumpsteak_choreography::{InMemoryHandler, Program, interpret};

let mut handler = InMemoryHandler::new(Role::Alice);
let program = Program::new()
    .send(Role::Bob, Message::Ping)
    .recv::<Message>(Role::Bob)
    .end();

let mut endpoint = ();
let result = interpret(&mut handler, &mut endpoint, program).await?;
```

The `InMemoryHandler` provides local message passing for testing. See `06_rumpsteak_handler.md` for production handlers.

## Core Concepts

### Choreographies

A choreography specifies a distributed protocol from a global viewpoint. Each role sees only their local behavior after projection.

### Roles

Roles are participants in the protocol. They send and receive messages according to their projected session type.

### Messages

Messages are data exchanged between roles. They must implement serde's `Serialize` and `Deserialize`.

### Effect Handlers

Handlers interpret choreographic effects into actual communication. Different handlers provide different transports (in-memory, session-typed channels, WebSockets).

### Projection

The system projects global choreographies into local session types. Each role gets a type-safe API for their part of the protocol.
