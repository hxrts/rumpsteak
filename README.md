# :meat_on_bone: Rumpsteak

[![Actions](https://github.com/zakcutner/rumpsteak/workflows/Check/badge.svg)](https://github.com/zakcutner/rumpsteak/actions)
[![Crate](https://img.shields.io/crates/v/rumpsteak)](https://crates.io/crates/rumpsteak)
[![Docs](https://docs.rs/rumpsteak/badge.svg)](https://docs.rs/rumpsteak)
[![License](https://img.shields.io/crates/l/rumpsteak)](LICENSE)

Rumpsteak is a Rust framework for _safely_ and _efficiently_ implementing
[message-passing](https://doc.rust-lang.org/book/ch16-02-message-passing.html)
[asynchronous](https://rust-lang.github.io/async-book/) programs. It uses
multiparty session types to statically guarantee the absence of communication errors such as deadlocks and asynchronous subtyping to allow optimizing communications.

Multiparty session types (MPST) verify the safety of message-passing protocols, as described in [A Very Gentle Introduction to Multiparty Session Types](http://mrg.doc.ic.ac.uk/publications/a-very-gentle-introduction-to-multiparty-session-types/main.pdf).
Asynchronous subtyping, introduced for MPST in [Precise Subtyping for
Asynchronous Multiparty Sessions](http://mrg.doc.ic.ac.uk/publications/precise-subtyping-for-asynchronous-multiparty-sessions/main.pdf),
verifies the reordering of messages to create more optimized implementations than are usually possible with MPST.

## Features

- Deadlock-free communication with session types.
- Integrates with `async`/`await` code.
- Supports any number of participants.
- Choreographic programming with DSL parser and automatic projection.
- Effect handler system with multiple implementations (in-memory, distributed).
- Production-ready RumpsteakHandler with session state tracking.
- Middleware support (tracing, retry, metrics, fault injection).
- WebAssembly (WASM) support for browser-based protocols.

## Usage

This is the Aura fork of Rumpsteak with enhanced choreographic programming support.

```toml
[dependencies]
rumpsteak-aura = { git = "https://github.com/aura-project/rumpsteak-aura" }
```

For choreographic programming:
```toml
[dependencies]
rumpsteak-choreography = { git = "https://github.com/aura-project/rumpsteak-aura" }
```

## Example

```rust
use futures::{
    channel::mpsc::{UnboundedReceiver, UnboundedSender},
    executor, try_join,
};
use rumpsteak::{
    channel::Bidirectional, session, try_session, End, Message, Receive, Role, Roles, Send,
};
use std::{error::Error, result};

type Result<T> = result::Result<T, Box<dyn Error>>;

type Channel = Bidirectional<UnboundedSender<Label>, UnboundedReceiver<Label>>;

#[derive(Roles)]
struct Roles(C, S);

#[derive(Role)]
#[message(Label)]
struct C(#[route(S)] Channel);

#[derive(Role)]
#[message(Label)]
struct S(#[route(C)] Channel);

#[derive(Message)]
enum Label {
    Add(Add),
    Sum(Sum),
}

struct Add(i32);
struct Sum(i32);

#[session]
type Client = Send<S, Add, Send<S, Add, Receive<S, Sum, End>>>;

#[session]
type Server = Receive<C, Add, Receive<C, Add, Send<C, Sum, End>>>;

async fn client(role: &mut C, x: i32, y: i32) -> Result<i32> {
    try_session(role, |s: Client<'_, _>| async {
        let s = s.send(Add(x)).await?;
        let s = s.send(Add(y)).await?;
        let (Sum(z), s) = s.receive().await?;
        Ok((z, s))
    })
    .await
}

async fn server(role: &mut S) -> Result<()> {
    try_session(role, |s: Server<'_, _>| async {
        let (Add(x), s) = s.receive().await?;
        let (Add(y), s) = s.receive().await?;
        let s = s.send(Sum(x + y)).await?;
        Ok(((), s))
    })
    .await
}

fn main() {
    let Roles(mut c, mut s) = Roles::default();
    executor::block_on(async {
        let (output, _) = try_join!(client(&mut c, 1, 2), server(&mut s)).unwrap();
        assert_eq!(output, 3);
    });
}
```

## Structure

#### `caching/`

HTTP cache case study backed by Redis.

#### `choreography/`

Choreographic programming layer enabling global protocol specification with automatic projection to local session types. Includes:
- DSL Parser: Pest-based parser for `.choreography` files with protocol composition, guards, annotations, and parameterized roles
- Effect Handler System: Transport-agnostic protocol implementations with middleware support
- Multiple Handlers: `InMemoryHandler` for testing, `RumpsteakHandler` for production distributed execution
- Session State Tracking: Metadata tracking for debugging and monitoring
- Complete Documentation: Comprehensive guides in `docs/` directory
- WebAssembly Support: Works in browser environments

*This is the primary extension of the original version with significant enhancements.*

#### `examples/`

Many examples of using Rumpsteak from popular protocols. *Updated to use new APIs*. Includes `wasm-ping-pong/` demonstrating browser-based protocols.

#### `fsm/`

Finite state machine support for session types, including DOT parsing and subtyping verification.

#### `macros/`

Crate for procedural macros used within Rumpsteak's API.

## WebAssembly Support

Rumpsteak now supports compilation to WebAssembly! The core session types and choreography system can run in browser environments. See `examples/wasm-ping-pong/` for a complete example and `docs/WASM_IMPLEMENTATION_SUMMARY.md` for implementation details.

Key features:
- Core session types compile to WASM
- Effect handlers work in browser
- Platform-agnostic runtime abstraction
- Example with browser deployment
- Supports custom network transports - Implement `ChoreoHandler` with WebSockets, WebRTC, etc.

Quick start:
```bash
cd examples/wasm-ping-pong
./build.sh  # or: wasm-pack build --target web
# Serve and open in browser
```

Custom network handlers: See `docs/07_wasm_guide.md` for implementing WebSocket/WebRTC handlers for real distributed protocols in WASM.

## License

Licensed under the MIT license. See the [LICENSE](LICENSE) file for details.
