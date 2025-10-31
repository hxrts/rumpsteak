# Using Rumpsteak Handlers

## Overview

The `RumpsteakHandler` provides a production-ready implementation of choreographic effects using session-typed channels. This guide covers everything you need to know to use it effectively in your applications.

## Quick Start

### Basic Two-Party Protocol

```rust
use rumpsteak_choreography::effects::{
    ChoreoHandler,
    handlers::rumpsteak::{RumpsteakHandler, RumpsteakEndpoint, SimpleChannel},
};

// Define roles
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Role { Alice, Bob }

impl rumpsteak_aura::Role for Role {
    type Message = MyMessage;
    fn seal(&mut self) {}
    fn is_sealed(&self) -> bool { false }
}

// Define messages
#[derive(Debug, Clone, Serialize, Deserialize)]
struct MyMessage {
    content: String,
}

impl rumpsteak_aura::Message<Box<dyn std::any::Any + Send>> for MyMessage {
    fn upcast(msg: Box<dyn std::any::Any + Send>) -> Self {
        *msg.downcast::<MyMessage>().unwrap()
    }
    fn downcast(self) -> Result<Box<dyn std::any::Any + Send>, Self> {
        Ok(Box::new(self))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create endpoints
    let mut alice_ep = RumpsteakEndpoint::new(Role::Alice);
    let mut bob_ep = RumpsteakEndpoint::new(Role::Bob);
    
    // Setup channels
    let (alice_ch, bob_ch) = SimpleChannel::pair();
    alice_ep.register_channel(Role::Bob, alice_ch);
    bob_ep.register_channel(Role::Alice, bob_ch);
    
    // Create handlers
    let mut alice_handler = RumpsteakHandler::<Role, MyMessage>::new();
    let mut bob_handler = RumpsteakHandler::<Role, MyMessage>::new();
    
    // Send and receive
    let msg = MyMessage { content: "Hello!".to_string() };
    alice_handler.send(&mut alice_ep, Role::Bob, &msg).await?;
    
    let received: MyMessage = bob_handler.recv(&mut bob_ep, Role::Alice).await?;
    println!("Received: {}", received.content);
    
    Ok(())
}
```

---

## Core Concepts

### Roles

Roles represent participants in the choreography. They must implement:
- `rumpsteak_aura::Role`
- `Clone`, `Copy`, `Debug`, `PartialEq`, `Eq`, `Hash`

### Messages

Messages are the data exchanged between roles. They must:
- Implement `Serialize` and `Deserialize` (via serde)
- Implement `rumpsteak_aura::Message`
- Be `Send` and `Sync`

### Endpoints

`RumpsteakEndpoint<R>` manages the channels and session state for a role:
- One endpoint per role in the protocol
- Contains channels to all peers
- Tracks session metadata (operation counts, state descriptions)

### Channels

`SimpleChannel` provides bidirectional async message passing:
- Created in pairs: `SimpleChannel::pair()`
- Uses mpsc unbounded channels internally
- Automatically serializes/deserializes messages

### Handlers

`RumpsteakHandler<R, M>` interprets choreographic effects:
- Stateless (can be shared across operations)
- Implements `ChoreoHandler` trait
- Provides send, recv, choose, offer operations

---

## API Reference

### RumpsteakEndpoint

```rust
impl<R: Role + Eq + Hash + Clone> RumpsteakEndpoint<R>
```

#### Constructor
```rust
pub fn new(local_role: R) -> Self
```
Create a new endpoint for a role.

#### Channel Management
```rust
pub fn register_channel<T>(&mut self, peer: R, channel: T)
```
Register a channel with a peer role.

```rust
pub fn has_channel(&self, peer: &R) -> bool
```
Check if a channel exists for a peer.

```rust
pub fn close_channel(&mut self, peer: &R) -> bool
```
Close a specific channel.

```rust
pub fn close_all_channels(&mut self) -> usize
```
Close all channels and return count.

```rust
pub fn active_channel_count(&self) -> usize
```
Get number of active channels.

```rust
pub fn is_all_closed(&self) -> bool
```
Check if all channels are closed.

#### Metadata Access
```rust
pub fn get_metadata(&self, peer: &R) -> Option<&SessionMetadata>
```
Get session metadata for a peer.

```rust
pub fn all_metadata(&self) -> Vec<(R, &SessionMetadata)>
```
Get metadata for all sessions.

### RumpsteakHandler

```rust
impl<R, M> RumpsteakHandler<R, M>
```

#### Constructor
```rust
pub fn new() -> Self
```
Create a new handler.

#### ChoreoHandler Implementation

```rust
async fn send<Msg>(&mut self, ep: &mut Endpoint, to: Role, msg: &Msg) -> Result<()>
where Msg: Serialize + Send + Sync
```
Send a message to a role.

```rust
async fn recv<Msg>(&mut self, ep: &mut Endpoint, from: Role) -> Result<Msg>
where Msg: DeserializeOwned + Send
```
Receive a message from a role.

```rust
async fn choose(&mut self, ep: &mut Endpoint, who: Role, label: Label) -> Result<()>
```
Make a choice (internal choice).

```rust
async fn offer(&mut self, ep: &mut Endpoint, from: Role) -> Result<Label>
```
Offer a choice (external choice).

```rust
async fn with_timeout<F, T>(&mut self, ep: &mut Endpoint, at: Role, dur: Duration, body: F) -> Result<T>
where F: Future<Output = Result<T>> + Send
```
Execute operation with timeout.

### SimpleChannel

```rust
pub struct SimpleChannel
```

#### Constructor
```rust
pub fn pair() -> (Self, Self)
```
Create a connected pair of channels.

#### Operations
```rust
pub async fn send(&mut self, msg: Vec<u8>) -> Result<(), String>
```
Send raw bytes.

```rust
pub async fn recv(&mut self) -> Result<Vec<u8>, String>
```
Receive raw bytes.

### SessionMetadata

```rust
pub struct SessionMetadata {
    pub state_description: String,
    pub is_complete: bool,
    pub operation_count: usize,
}
```

Tracks session progression:
- `state_description`: Human-readable current state
- `is_complete`: Whether session has completed
- `operation_count`: Number of operations performed

---

## Usage Patterns

### Pattern 1: Request-Response

```rust
// Client side
let request = Request { query: "data".to_string() };
handler.send(&mut endpoint, Role::Server, &request).await?;
let response: Response = handler.recv(&mut endpoint, Role::Server).await?;
```

### Pattern 2: Choice with Branches

```rust
// Sender
let decision = if condition {
    Label("accept")
} else {
    Label("reject")
};
handler.choose(&mut endpoint, Role::Other, decision).await?;

// Receiver
let choice = handler.offer(&mut endpoint, Role::Other).await?;
match choice.0 {
    "accept" => {
        // Handle accept branch
    }
    "reject" => {
        // Handle reject branch
    }
    _ => unreachable!(),
}
```

### Pattern 3: Sequential Messages

```rust
for item in items {
    handler.send(&mut endpoint, Role::Peer, &item).await?;
    let ack: Ack = handler.recv(&mut endpoint, Role::Peer).await?;
}
```

### Pattern 4: Multi-Party Coordination

```rust
// Coordinator
let offer: Offer = handler.recv(&mut endpoint, Role::Buyer).await?;
handler.send(&mut endpoint, Role::Seller, &offer).await?;

let response: Response = handler.recv(&mut endpoint, Role::Seller).await?;
handler.send(&mut endpoint, Role::Buyer, &response).await?;
```

### Pattern 5: Timeout Protection

```rust
let result = handler.with_timeout(
    &mut endpoint,
    Role::Self_,
    Duration::from_secs(5),
    async {
        handler.recv(&mut endpoint, Role::Peer).await
    }
).await;

match result {
    Ok(msg) => {
        // Process message
    }
    Err(ChoreographyError::Timeout(_)) => {
        // Handle timeout
    }
    Err(e) => {
        // Handle other errors
    }
}
```

## Best Practices

### 1. Resource Management

**DO**:
```rust
// Close channels explicitly when done
endpoint.close_all_channels();
```

**DO**:
```rust
// Use Drop to ensure cleanup
{
    let mut endpoint = RumpsteakEndpoint::new(role);
    // ... use endpoint ...
} // Drop ensures cleanup
```

**DON'T**:
```rust
// Don't forget to clean up resources
let mut endpoint = RumpsteakEndpoint::new(role);
// ... use endpoint ...
// Forgot to close!
```

### 2. Error Handling

**DO**:
```rust
match handler.send(&mut ep, role, &msg).await {
    Ok(()) => { /* success */ }
    Err(ChoreographyError::Transport(e)) => {
        // Handle transport error
        tracing::error!("Send failed: {}", e);
    }
    Err(e) => {
        // Handle other errors
    }
}
```

**DON'T**:
```rust
// Don't ignore errors
handler.send(&mut ep, role, &msg).await.unwrap();
```

### 3. Channel Setup

**DO**:
```rust
// Setup all channels before starting protocol
let (ch1, ch2) = SimpleChannel::pair();
alice_ep.register_channel(Role::Bob, ch1);
bob_ep.register_channel(Role::Alice, ch2);

// Then start protocol
protocol_run().await?;
```

**DON'T**:
```rust
// Don't register channels mid-protocol
handler.send(&mut ep, role, &msg).await?; // Might not have channel!
ep.register_channel(role, channel); // Too late!
```

### 4. Metadata Usage

**DO**:
```rust
// Use metadata for debugging and monitoring
if let Some(meta) = endpoint.get_metadata(&peer) {
    tracing::info!(
        peer = ?peer,
        operations = meta.operation_count,
        state = %meta.state_description,
        "Session status"
    );
}
```

### 5. Testing

**DO**:
```rust
#[tokio::test]
async fn test_protocol() {
    // Setup test environment
    let mut alice_ep = RumpsteakEndpoint::new(Role::Alice);
    let mut bob_ep = RumpsteakEndpoint::new(Role::Bob);
    
    let (alice_ch, bob_ch) = SimpleChannel::pair();
    alice_ep.register_channel(Role::Bob, alice_ch);
    bob_ep.register_channel(Role::Alice, bob_ch);
    
    // Test protocol
    let msg = TestMessage { data: vec![1, 2, 3] };
    handler.send(&mut alice_ep, Role::Bob, &msg).await.unwrap();
    
    let received: TestMessage = handler.recv(&mut bob_ep, Role::Alice).await.unwrap();
    assert_eq!(received.data, vec![1, 2, 3]);
}
```
