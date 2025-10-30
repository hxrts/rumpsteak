// Demonstration of the choreography! macro
// This example shows the fully integrated choreography system

use rumpsteak_aura::try_session;
use futures::executor;
use std::error::Error;

// Define a simple protocol using the choreography! macro
// This macro generates:
// - Role structs (Alice, Bob) with proper routing
// - Message types (Greeting, Reply, Farewell)
// - Session types (AliceSession, BobSession) with correct send/receive ordering
// - A setup() function to initialize the roles
// - All necessary trait implementations
rumpsteak_aura::choreography! {
    protocol SimpleProtocol {
        roles: Alice, Bob;
        
        Alice -> Bob: Greeting(String);
        Bob -> Alice: Reply(String);
        Alice -> Bob: Farewell;
    }
}

// Example usage
fn main() {
    executor::block_on(async {
        let result = run_protocol().await;
        match result {
            Ok(()) => println!("\nProtocol completed successfully!"),
            Err(e) => eprintln!("Protocol error: {}", e),
        }
    });
}

async fn run_protocol() -> Result<(), Box<dyn Error>> {
    // Initialize roles using the generated setup function
    let Roles(mut alice, mut bob) = setup();
    
    // Alice's behavior
    let alice_task = async move {
        try_session(&mut alice, |s: AliceSession<'_, _>| async move {
            // Send greeting
            println!("Alice: Sending greeting");
            let s = s.send(Greeting("Hello Bob!".to_string())).await?;
            
            // Receive reply
            let (Reply(msg), s) = s.receive().await?;
            println!("Alice: Received reply: {}", msg);
            
            // Send farewell
            println!("Alice: Sending farewell");
            let s = s.send(Farewell).await?;
            
            Ok::<_, Box<dyn Error>>(((), s))
        })
        .await
    };
    
    // Bob's behavior
    let bob_task = async move {
        try_session(&mut bob, |s: BobSession<'_, _>| async move {
            // Receive greeting
            let (Greeting(msg), s) = s.receive().await?;
            println!("Bob: Received greeting: {}", msg);
            
            // Send reply
            println!("Bob: Sending reply");
            let s = s.send(Reply("Hi Alice, nice to hear from you!".to_string())).await?;
            
            // Receive farewell
            let (Farewell, s) = s.receive().await?;
            println!("Bob: Received farewell");
            
            Ok::<_, Box<dyn Error>>(((), s))
        })
        .await
    };
    
    // Run both roles concurrently
    futures::try_join!(alice_task, bob_task)?;
    Ok(())
}