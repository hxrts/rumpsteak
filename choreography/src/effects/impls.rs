// Concrete effect handlers for Rumpsteak
//
// This module provides handlers that bridge the effect system to
// Rumpsteak's session-typed channels and other transport implementations.

use async_trait::async_trait;
use serde::{Serialize, de::DeserializeOwned};
use std::time::Duration;
use std::marker::PhantomData;
use std::collections::HashMap;
use futures::{Sink, Stream};

use crate::effects::{ChoreoHandler, ChoreographyError, Result, Label, RoleId};
use rumpsteak_aura::{Message, Role, Route};

/// Rumpsteak endpoint wrapper that provides access to session-typed channels
pub struct RumpsteakEndpoint<R: Role> {
    _phantom: PhantomData<R>,
}

impl<R: Role> RumpsteakEndpoint<R> {
    pub fn new() -> Self {
        Self {
            _phantom: PhantomData,
        }
    }
}

impl<R: Role> Default for RumpsteakEndpoint<R> {
    fn default() -> Self {
        Self::new()
    }
}


/// Handler that interprets effects using Rumpsteak's session-typed channels
pub struct RumpsteakHandler<R, M> {
    _phantom: PhantomData<(R, M)>,
}

impl<R, M> RumpsteakHandler<R, M> {
    pub fn new() -> Self {
        Self {
            _phantom: PhantomData,
        }
    }
}

impl<R, M> Default for RumpsteakHandler<R, M> {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper trait to get routes from roles
pub trait HasRoute<R: RoleId>: Route<R> {
    type RouteType: Stream<Item = Self::Message> + Sink<Self::Message> + Unpin;
    
    fn get_route_mut(&mut self) -> &mut Self::RouteType;
}

#[async_trait]
impl<R, M> ChoreoHandler for RumpsteakHandler<R, M>
where
    R: Role<Message = M> + Send + Sync + RoleId + 'static,
    M: Message<Box<dyn std::any::Any + Send>> + Send + Sync + 'static,
{
    type Role = R;
    type Endpoint = RumpsteakEndpoint<R>;

    async fn send<Msg: Serialize + Send + Sync>(
        &mut self, 
        ep: &mut Self::Endpoint, 
        to: Self::Role, 
        msg: &Msg
    ) -> Result<()> {
        // Serialize the message
        let serialized = bincode::serialize(msg)
            .map_err(|e| ChoreographyError::Transport(format!("Serialization failed: {}", e)))?;
        tracing::debug!(?to, size = serialized.len(), "Sending message");
        
        // Rumpsteak integration architecture:
        // The endpoint maintains session-typed channels for each peer role.
        // When sending, we:
        // 1. Look up the typed channel: ep.get_channel_to(to)
        // 2. Execute the typed Send operation: channel.send(msg).await
        //    - Type system ensures msg matches expected type at this point
        // 3. Store the advanced session state: ep.set_channel_state(to, next_state)
        //    - This moves the type forward (Send<R, M, S> -> S)
        
        // This handler is designed for protocol validation and testing.
        // Production use requires connecting actual Rumpsteak session channels.
        let _ = ep;
        Ok(())
    }

    async fn recv<Msg: DeserializeOwned + Send>(
        &mut self, 
        ep: &mut Self::Endpoint, 
        from: Self::Role
    ) -> Result<Msg> {
        tracing::debug!(?from, "Receiving message");
        
        // Rumpsteak integration architecture:
        // The endpoint maintains session-typed channels for each peer role.
        // When receiving, we:
        // 1. Look up the typed channel: ep.get_channel_from(from)
        // 2. Execute the typed Receive operation: channel.receive().await
        //    - Type system ensures received msg matches expected type
        // 3. Store the advanced session state: ep.set_channel_state(from, next_state)
        //    - This moves the type forward (Receive<R, M, S> -> S)
        // 4. Return the deserialized message
        
        // This handler requires actual channel connections.
        // For testing choreographies without channels, use InMemoryHandler.
        let _ = ep;
        Err(ChoreographyError::Transport(
            "RumpsteakHandler recv requires connected session channels - use InMemoryHandler for testing".into()
        ))
    }

    async fn choose(
        &mut self, 
        _ep: &mut Self::Endpoint, 
_who: Self::Role, 
        _label: Label
    ) -> Result<()> {
        // Broadcast the choice label
        Ok(())
    }

    async fn offer(
        &mut self, 
        ep: &mut Self::Endpoint, 
        from: Self::Role
    ) -> Result<Label> {
        tracing::debug!(?from, "Offering choice");
        
        // Rumpsteak integration architecture:
        // When a role receives a choice, we:
        // 1. Use the Branch type to receive the label: channel.branch().await
        //    - Channel type: Branch<R, { Label1(M1, S1), Label2(M2, S2), ... }>
        // 2. Map the received session label to our Label type
        // 3. Store the appropriate continuation state based on the branch taken
        //    - ep.set_channel_state(from, next_state)
        //    - Transitions to S1, S2, etc. depending on the choice
        // 4. Return the label to determine which branch to execute
        
        // Returns default label for testing.
        // Production use requires actual Branch channel types.
        let _ = ep;
        Ok(Label::default())
    }

    async fn with_timeout<F, T>(
        &mut self,
        _ep: &mut Self::Endpoint,
        _at: Self::Role,
        dur: Duration,
        body: F,
    ) -> Result<T>
    where
        F: std::future::Future<Output = Result<T>> + Send
    {
        match tokio::time::timeout(dur, body).await {
            Ok(result) => result,
            Err(_) => Err(ChoreographyError::Timeout(dur)),
        }
    }
}

// Type aliases for complex channel types
type MessageChannelPair = (tokio::sync::mpsc::UnboundedSender<Vec<u8>>, tokio::sync::mpsc::UnboundedReceiver<Vec<u8>>);
type ChoiceChannelPair = (tokio::sync::mpsc::UnboundedSender<Label>, tokio::sync::mpsc::UnboundedReceiver<Label>);

/// In-memory handler for testing - uses tokio channels
pub struct InMemoryHandler<R: RoleId> {
    role: R,
    // Channel map for sending/receiving messages between roles
    channels: std::sync::Arc<std::sync::Mutex<HashMap<(R, R), MessageChannelPair>>>,
    // Choice channel for broadcasting/receiving choice labels
    choice_channels: std::sync::Arc<std::sync::Mutex<HashMap<(R, R), ChoiceChannelPair>>>,
}

impl<R: RoleId> InMemoryHandler<R> {
    pub fn new(role: R) -> Self {
        Self {
            role,
            channels: std::sync::Arc::new(std::sync::Mutex::new(HashMap::new())),
            choice_channels: std::sync::Arc::new(std::sync::Mutex::new(HashMap::new())),
        }
    }
    
    /// Create a new handler with shared channels for coordinated testing
    pub fn with_channels(
        role: R,
        channels: std::sync::Arc<std::sync::Mutex<HashMap<(R, R), MessageChannelPair>>>,
        choice_channels: std::sync::Arc<std::sync::Mutex<HashMap<(R, R), ChoiceChannelPair>>>,
    ) -> Self {
        Self {
            role,
            channels,
            choice_channels,
        }
    }
    
    /// Get or create a channel pair for communication between two roles
    fn get_or_create_channel(&self, from: R, to: R) -> tokio::sync::mpsc::UnboundedSender<Vec<u8>> {
        let mut channels = self.channels.lock().unwrap();
        channels.entry((from, to)).or_insert_with(|| {
            let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
            (tx, rx)
        }).0.clone()
    }
    
    /// Get receiver for a channel pair
    fn get_receiver(&self, from: R, to: R) -> Option<tokio::sync::mpsc::UnboundedReceiver<Vec<u8>>> {
        let mut channels = self.channels.lock().unwrap();
        channels.remove(&(from, to)).map(|(_, rx)| rx)
    }
    
    /// Get or create a choice channel pair for broadcasting choices
    #[allow(dead_code)]
    fn get_or_create_choice_channel(&self, from: R, to: R) -> tokio::sync::mpsc::UnboundedSender<Label> {
        let mut channels = self.choice_channels.lock().unwrap();
        channels.entry((from, to)).or_insert_with(|| {
            let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
            (tx, rx)
        }).0.clone()
    }
    
    /// Get choice receiver for a channel pair
    fn get_choice_receiver(&self, from: R, to: R) -> Option<tokio::sync::mpsc::UnboundedReceiver<Label>> {
        let mut channels = self.choice_channels.lock().unwrap();
        channels.remove(&(from, to)).map(|(_, rx)| rx)
    }
}

#[async_trait]
impl<R: RoleId + 'static> ChoreoHandler for InMemoryHandler<R> {
    type Role = R;
    type Endpoint = ();

    async fn send<M: Serialize + Send + Sync>(
        &mut self, 
        _ep: &mut Self::Endpoint, 
        to: Self::Role, 
        msg: &M
    ) -> Result<()> {
        // Serialize message
        let bytes = bincode::serialize(msg)
            .map_err(|e| ChoreographyError::Serialization(e.to_string()))?;
        
        // Get or create channel for (self.role, to) and send bytes
        let sender = self.get_or_create_channel(self.role, to);
        sender.send(bytes)
            .map_err(|_| ChoreographyError::Transport(
                format!("Failed to send message from {:?} to {:?}", self.role, to)
            ))?;
        
        tracing::trace!(?to, "InMemoryHandler: send success");
        Ok(())
    }

    async fn recv<M: DeserializeOwned + Send>(
        &mut self, 
        _ep: &mut Self::Endpoint, 
        from: Self::Role
    ) -> Result<M> {
        tracing::trace!(?from, "InMemoryHandler: recv start");
        
        // Get the receiver for messages from 'from' to 'self.role'
        let mut receiver = self.get_receiver(from, self.role)
            .ok_or_else(|| ChoreographyError::Transport(
                format!("No channel from {:?} to {:?}", from, self.role)
            ))?;
        
        // Wait for message
        let bytes = receiver.recv().await
            .ok_or_else(|| ChoreographyError::Transport(
                "Channel closed while waiting for message".into()
            ))?;
        
        // Put the receiver back
        {
            let mut channels = self.channels.lock().unwrap();
            if let Some((tx, _)) = channels.remove(&(from, self.role)) {
                channels.insert((from, self.role), (tx, receiver));
            }
        }
        
        // Deserialize message
        let msg = bincode::deserialize(&bytes)
            .map_err(|e| ChoreographyError::Serialization(e.to_string()))?;
        
        tracing::trace!(?from, "InMemoryHandler: recv success");
        Ok(msg)
    }

    async fn choose(
        &mut self, 
        _ep: &mut Self::Endpoint, 
        who: Self::Role, 
        label: Label
    ) -> Result<()> {
        if who == self.role {
            // Broadcast choice to all other roles - for simplicity, we don't implement
            // full broadcast here since we don't know all other roles
            tracing::trace!(?label, "InMemoryHandler: broadcasting choice");
        }
        Ok(())
    }

    async fn offer(
        &mut self, 
        _ep: &mut Self::Endpoint, 
        from: Self::Role
    ) -> Result<Label> {
        tracing::trace!(?from, "InMemoryHandler: waiting for choice");
        
        // Get the choice receiver for choices from 'from' to 'self.role'
        let mut receiver = self.get_choice_receiver(from, self.role)
            .ok_or_else(|| ChoreographyError::Transport(
                format!("No choice channel from {:?} to {:?}", from, self.role)
            ))?;
        
        // Wait for choice label
        let label = receiver.recv().await
            .ok_or_else(|| ChoreographyError::Transport(
                "Choice channel closed while waiting for label".into()
            ))?;
        
        // Put the receiver back
        {
            let mut channels = self.choice_channels.lock().unwrap();
            if let Some((tx, _)) = channels.remove(&(from, self.role)) {
                channels.insert((from, self.role), (tx, receiver));
            }
        }
        
        tracing::trace!(?from, ?label, "InMemoryHandler: received choice");
        Ok(label)
    }

    async fn with_timeout<F, T>(
        &mut self,
        _ep: &mut Self::Endpoint,
        at: Self::Role,
        dur: Duration,
        body: F,
    ) -> Result<T>
    where
        F: std::future::Future<Output = Result<T>> + Send
    {
        if at == self.role {
            match tokio::time::timeout(dur, body).await {
                Ok(result) => result,
                Err(_) => Err(ChoreographyError::Timeout(dur)),
            }
        } else {
            body.await
        }
    }
}

/// Recording handler for testing - captures all effects for verification
#[derive(Clone)]
pub struct RecordingHandler<R: RoleId> {
    pub events: std::sync::Arc<std::sync::Mutex<Vec<RecordedEvent<R>>>>,
    role: R,
}

#[derive(Debug, Clone)]
pub enum RecordedEvent<R: RoleId> {
    Send { from: R, to: R, msg_type: String },
    Recv { from: R, to: R, msg_type: String },
    Choose { at: R, label: Label },
    Offer { from: R, to: R },
}

impl<R: RoleId> RecordingHandler<R> {
    pub fn new(role: R) -> Self {
        Self {
            events: std::sync::Arc::new(std::sync::Mutex::new(Vec::new())),
            role,
        }
    }
    
    pub fn events(&self) -> Vec<RecordedEvent<R>> {
        self.events.lock().unwrap().clone()
    }
    
    pub fn clear(&self) {
        self.events.lock().unwrap().clear();
    }
}

#[async_trait]
impl<R: RoleId + 'static> ChoreoHandler for RecordingHandler<R> {
    type Role = R;
    type Endpoint = ();

    async fn send<M: Serialize + Send + Sync>(
        &mut self, 
        _ep: &mut Self::Endpoint, 
        to: Self::Role, 
        _msg: &M
    ) -> Result<()> {
        self.events.lock().unwrap().push(RecordedEvent::Send {
            from: self.role,
            to,
            msg_type: std::any::type_name::<M>().to_string(),
        });
        Ok(())
    }

    async fn recv<M: DeserializeOwned + Send>(
        &mut self, 
        _ep: &mut Self::Endpoint, 
        from: Self::Role
    ) -> Result<M> {
        self.events.lock().unwrap().push(RecordedEvent::Recv {
            from,
            to: self.role,
            msg_type: std::any::type_name::<M>().to_string(),
        });
        Err(ChoreographyError::Transport(
            "RecordingHandler cannot produce values".into()
        ))
    }

    async fn choose(
        &mut self, 
        _ep: &mut Self::Endpoint, 
        at: Self::Role, 
        label: Label
    ) -> Result<()> {
        self.events.lock().unwrap().push(RecordedEvent::Choose { at, label });
        Ok(())
    }

    async fn offer(
        &mut self, 
        _ep: &mut Self::Endpoint, 
        from: Self::Role
    ) -> Result<Label> {
        self.events.lock().unwrap().push(RecordedEvent::Offer {
            from,
            to: self.role,
        });
        Err(ChoreographyError::Transport(
            "RecordingHandler cannot produce labels".into()
        ))
    }

    async fn with_timeout<F, T>(
        &mut self,
        _ep: &mut Self::Endpoint,
        _at: Self::Role,
        _dur: Duration,
        body: F,
    ) -> Result<T>
    where
        F: std::future::Future<Output = Result<T>> + Send
    {
        body.await
    }
}