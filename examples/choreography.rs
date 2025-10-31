// Demonstration of effect-based choreography
// This example shows how the effect handler abstraction provides
// clean separation between protocol logic and transport implementation

use rumpsteak_choreography::{
    ChoreoHandler, Result,
    Trace, Metrics, InMemoryHandler, RecordingHandler
};
use serde::{Serialize, Deserialize};
use futures::executor;

// Define protocol roles
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
enum Role {
    Alice,
    Bob,
}

// Define message types
#[derive(Clone, Debug, Serialize, Deserialize)]
struct Greeting(String);

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Reply(String);

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Farewell;

// Define protocol endpoint (runtime-specific state)
struct SimpleProtocolEndpoint;

// Alice's role implementation using effect handler
async fn run_alice<H: ChoreoHandler<Role = Role>>(
    handler: &mut H,
    endpoint: &mut H::Endpoint,
) -> Result<String> {
    // Send greeting
    println!("Alice: Sending greeting");
    handler.send(endpoint, Role::Bob, &Greeting("Hello Bob!".to_string())).await?;
    
    // Receive reply
    let Reply(msg) = handler.recv(endpoint, Role::Bob).await?;
    println!("Alice: Received reply: {}", msg);
    
    // Send farewell
    println!("Alice: Sending farewell");
    handler.send(endpoint, Role::Bob, &Farewell).await?;
    
    Ok(msg)
}

// Bob's role implementation using effect handler
async fn run_bob<H: ChoreoHandler<Role = Role>>(
    handler: &mut H,
    endpoint: &mut H::Endpoint,
) -> Result<()> {
    // Receive greeting
    let Greeting(msg) = handler.recv(endpoint, Role::Alice).await?;
    println!("Bob: Received greeting: {}", msg);
    
    // Send reply
    println!("Bob: Sending reply");
    let reply = Reply("Hi Alice, nice to hear from you!".to_string());
    handler.send(endpoint, Role::Alice, &reply).await?;
    
    // Receive farewell
    let Farewell = handler.recv(endpoint, Role::Alice).await?;
    println!("Bob: Received farewell");
    
    Ok(())
}

// Example with different handler implementations
fn main() {
    executor::block_on(async {
        println!("=== Running with Recording Handler ===");
        run_with_recording().await;
        
        println!("\n=== Running with Traced Handler ===");
        run_with_tracing().await;
        
        println!("\n=== Running with Metrics ===");
        run_with_metrics().await;
    });
}

async fn run_with_recording() {
    // Use recording handler to capture protocol events
    let mut alice_handler = RecordingHandler::new(Role::Alice);
    let mut bob_handler = RecordingHandler::new(Role::Bob);
    
    let mut alice_ep = (); // RecordingHandler uses () as endpoint
    let mut bob_ep = ();   // RecordingHandler uses () as endpoint
    
    // Run Alice's side
    let alice_result = run_alice(&mut alice_handler, &mut alice_ep).await;
    
    // Run Bob's side (would normally be concurrent)
    let bob_result = run_bob(&mut bob_handler, &mut bob_ep).await;
    
    // Examine captured events
    println!("\nAlice's events:");
    for event in alice_handler.events() {
        println!("  {:?}", event);
    }
    
    println!("\nBob's events:");
    for event in bob_handler.events() {
        println!("  {:?}", event);
    }
}

async fn run_with_tracing() {
    // Use tracing middleware for debugging
    let inner_handler = InMemoryHandler::new(Role::Alice);
    let mut handler = Trace::with_prefix(inner_handler, "alice");
    let mut endpoint = (); // InMemoryHandler uses () as endpoint
    
    // This would log all operations with tracing
    let result = run_alice(&mut handler, &mut endpoint).await;
    match result {
        Ok(reply) => println!("Alice completed with reply: {}", reply),
        Err(e) => eprintln!("Alice failed: {}", e),
    }
}

async fn run_with_metrics() {
    // Use metrics middleware to collect statistics
    let inner_handler = InMemoryHandler::new(Role::Bob);
    let mut handler = Metrics::new(inner_handler);
    let mut endpoint = (); // InMemoryHandler uses () as endpoint
    
    // Run protocol
    let result = run_bob(&mut handler, &mut endpoint).await;
    
    // Print metrics
    println!("\nMetrics:");
    println!("  Sends: {}", handler.send_count());
    println!("  Receives: {}", handler.recv_count());
    println!("  Errors: {}", handler.error_count());
}

// Example of protocol-agnostic testing
#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_protocol_flow() {
        // Test the protocol logic independent of transport
        let mut handler = RecordingHandler::new(Role::Alice);
        let mut endpoint = (); // RecordingHandler uses () as endpoint
        
        let result = run_alice(&mut handler, &mut endpoint).await;
        
        // Verify the protocol follows expected communication pattern
        let events = handler.events();
        assert_eq!(events.len(), 3); // 2 sends, 1 receive
    }
}