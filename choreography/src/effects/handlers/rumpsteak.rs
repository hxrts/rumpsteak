// Rumpsteak session-typed effect handler
//
// Implements integration with Rumpsteak's session-typed channels for choreographic effects.
use async_trait::async_trait;
use futures::channel::mpsc;
use futures::{Sink, SinkExt, Stream, StreamExt};
use serde::{de::DeserializeOwned, Serialize};
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::marker::PhantomData;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

use crate::effects::{ChoreoHandler, ChoreographyError, Label, Result, RoleId};
use rumpsteak_aura::{Message, Role, Route};

/// Simple bidirectional channel for basic message passing
///
/// This is a simplified channel type for Phase 1 implementation.
/// Phase 2 will integrate with full Rumpsteak session types.
#[derive(Clone)]
pub struct SimpleChannel {
    /// Sender for outgoing messages
    sender: mpsc::UnboundedSender<Vec<u8>>,
    /// Receiver for incoming messages (wrapped in Arc<Mutex> for cloning)
    receiver: Arc<Mutex<mpsc::UnboundedReceiver<Vec<u8>>>>,
}

impl SimpleChannel {
    /// Create a pair of connected channels
    pub fn pair() -> (Self, Self) {
        let (tx1, rx1) = mpsc::unbounded();
        let (tx2, rx2) = mpsc::unbounded();

        (
            SimpleChannel {
                sender: tx1,
                receiver: Arc::new(Mutex::new(rx2)),
            },
            SimpleChannel {
                sender: tx2,
                receiver: Arc::new(Mutex::new(rx1)),
            },
        )
    }

    /// Send a message
    pub async fn send(&mut self, msg: Vec<u8>) -> std::result::Result<(), String> {
        self.sender
            .send(msg)
            .await
            .map_err(|e| format!("Send failed: {}", e))
    }

    /// Receive a message
    pub async fn recv(&mut self) -> std::result::Result<Vec<u8>, String> {
        let mut receiver = self.receiver.lock().await;
        receiver
            .next()
            .await
            .ok_or_else(|| "Channel closed".to_string())
    }
}

/// Session state wrapper for tracking session type progression
///
/// This wraps a session-typed channel (Send<>, Receive<>, etc.) and tracks
/// its current state for proper progression through the protocol.
///
/// Phase 2 Note: Full Rumpsteak session type integration would require:
/// - Storing heterogeneous session types (Send<A,M1,S1>, Receive<B,M2,S2>, etc.)
/// - Progressing types through operations (Send<R,M,S> -> S)
/// - Managing lifetime parameters
/// - Type-safe branch selection for choices
///
/// Current implementation uses SimpleChannel as a baseline.
pub struct SessionState {
    /// The underlying channel (can be SimpleChannel or Rumpsteak session type)
    channel: Box<dyn Any + Send + Sync>,
    /// Type identifier for safe downcasting
    type_id: TypeId,
    /// Optional metadata about the session state
    metadata: SessionMetadata,
}

/// Metadata about a session state
#[derive(Debug, Clone)]
pub struct SessionMetadata {
    /// Human-readable description of current state
    pub state_description: String,
    /// Whether this session has completed
    pub is_complete: bool,
    /// Number of operations performed on this session
    pub operation_count: usize,
}

impl Default for SessionMetadata {
    fn default() -> Self {
        Self {
            state_description: "Initial".to_string(),
            is_complete: false,
            operation_count: 0,
        }
    }
}

impl SessionState {
    /// Create a new session state from a channel
    pub fn new<T: Any + Send + Sync + 'static>(channel: T) -> Self {
        Self {
            type_id: TypeId::of::<T>(),
            channel: Box::new(channel),
            metadata: SessionMetadata::default(),
        }
    }

    /// Create with metadata
    pub fn with_metadata<T: Any + Send + Sync + 'static>(
        channel: T,
        metadata: SessionMetadata,
    ) -> Self {
        Self {
            type_id: TypeId::of::<T>(),
            channel: Box::new(channel),
            metadata,
        }
    }

    /// Get the type ID
    pub fn type_id(&self) -> TypeId {
        self.type_id
    }

    /// Check if this matches a specific type
    pub fn is_type<T: Any + 'static>(&self) -> bool {
        self.type_id == TypeId::of::<T>()
    }

    /// Try to downcast to a specific type
    pub fn downcast<T: Any + 'static>(self) -> std::result::Result<T, Box<dyn Any + Send + Sync>> {
        if self.type_id == TypeId::of::<T>() {
            self.channel.downcast::<T>().map(|b| *b)
        } else {
            Err(self.channel)
        }
    }

    /// Get metadata
    pub fn metadata(&self) -> &SessionMetadata {
        &self.metadata
    }

    /// Update metadata
    pub fn update_metadata(&mut self, f: impl FnOnce(&mut SessionMetadata)) {
        f(&mut self.metadata);
    }

    /// Mark an operation performed
    pub fn mark_operation(&mut self, description: &str) {
        self.metadata.operation_count += 1;
        self.metadata.state_description = description.to_string();
    }

    /// Mark session as complete
    pub fn mark_complete(&mut self) {
        self.metadata.is_complete = true;
        self.metadata.state_description = "Complete".to_string();
    }
}

/// Type-erased channel wrapper for heterogeneous session type storage
///
/// Wraps a session-typed channel in a way that can be stored in a HashMap
/// while maintaining type safety through downcasting.
struct ChannelBox {
    /// The actual channel, type-erased as Box<dyn Any>
    inner: Box<dyn Any + Send + Sync>,
    /// TypeId for safe downcasting
    #[allow(dead_code)]
    type_id: TypeId,
}

impl ChannelBox {
    /// Create a new channel box from a typed channel
    fn new<T: Any + Send + Sync + 'static>(channel: T) -> Self {
        Self {
            type_id: TypeId::of::<T>(),
            inner: Box::new(channel),
        }
    }

    /// Attempt to downcast to a specific channel type
    #[allow(dead_code)]
    fn downcast_ref<T: Any + 'static>(&self) -> Option<&T> {
        if self.type_id == TypeId::of::<T>() {
            self.inner.downcast_ref::<T>()
        } else {
            None
        }
    }

    /// Attempt to downcast to a mutable reference
    #[allow(dead_code)]
    fn downcast_mut<T: Any + 'static>(&mut self) -> Option<&mut T> {
        if self.type_id == TypeId::of::<T>() {
            self.inner.downcast_mut::<T>()
        } else {
            None
        }
    }
}

/// Bundle of session-typed channels indexed by role
///
/// Stores heterogeneous session types in a type-safe manner using type erasure.
/// Each role maps to a channel with its current session state.
///
/// Phase 2 Enhancement: Now tracks session state metadata for each channel,
/// enabling visibility into session progression and state.
pub struct SessionChannelBundle<RoleKey>
where
    RoleKey: Eq + std::hash::Hash + Clone,
{
    /// Map from role to type-erased channel
    channels: HashMap<RoleKey, ChannelBox>,
    /// Map from role to session metadata
    session_metadata: HashMap<RoleKey, SessionMetadata>,
}

impl<RoleKey> SessionChannelBundle<RoleKey>
where
    RoleKey: Eq + std::hash::Hash + Clone,
{
    /// Create a new empty channel bundle
    pub fn new() -> Self {
        Self {
            channels: HashMap::new(),
            session_metadata: HashMap::new(),
        }
    }

    /// Register a session-typed channel for a role
    ///
    /// The channel type T should be a Rumpsteak session type like:
    /// - Send<'q, Q, R, L, S>
    /// - Receive<'q, Q, R, L, S>
    /// - End
    pub fn register<T: Any + Send + Sync + 'static>(&mut self, role: RoleKey, channel: T) {
        self.channels.insert(role.clone(), ChannelBox::new(channel));
        self.session_metadata
            .insert(role, SessionMetadata::default());
    }

    /// Register with metadata
    pub fn register_with_metadata<T: Any + Send + Sync + 'static>(
        &mut self,
        role: RoleKey,
        channel: T,
        metadata: SessionMetadata,
    ) {
        self.channels.insert(role.clone(), ChannelBox::new(channel));
        self.session_metadata.insert(role, metadata);
    }

    /// Take a channel for a role, removing it from the bundle
    ///
    /// This is used to perform session type operations which consume and return new states.
    /// After taking a channel, use `put_channel` to store the new state.
    pub fn take_channel(&mut self, role: &RoleKey) -> Option<Box<dyn Any + Send + Sync>> {
        self.channels.remove(role).map(|b| b.inner)
    }

    /// Put a channel back for a role
    ///
    /// Used after session operations to store the new session state.
    pub fn put_channel<T: Any + Send + Sync + 'static>(&mut self, role: RoleKey, channel: T) {
        self.channels.insert(role, ChannelBox::new(channel));
    }

    /// Get metadata for a role's session
    pub fn get_metadata(&self, role: &RoleKey) -> Option<&SessionMetadata> {
        self.session_metadata.get(role)
    }

    /// Update metadata for a role's session
    pub fn update_metadata(&mut self, role: &RoleKey, f: impl FnOnce(&mut SessionMetadata)) {
        if let Some(metadata) = self.session_metadata.get_mut(role) {
            f(metadata);
        }
    }

    /// Mark an operation performed on a role's session
    pub fn mark_operation(&mut self, role: &RoleKey, description: &str) {
        self.update_metadata(role, |m| {
            m.operation_count += 1;
            m.state_description = description.to_string();
        });
    }

    /// Check if a channel is registered for a role
    pub fn has_channel(&self, role: &RoleKey) -> bool {
        self.channels.contains_key(role)
    }

    /// Remove a channel for a role
    pub fn remove(&mut self, role: &RoleKey) -> bool {
        self.session_metadata.remove(role);
        self.channels.remove(role).is_some()
    }

    /// Get all session metadata (for debugging/monitoring)
    pub fn all_metadata(&self) -> Vec<(RoleKey, &SessionMetadata)> {
        self.session_metadata
            .iter()
            .map(|(k, v)| (k.clone(), v))
            .collect()
    }
}

// Note: SessionChannelBundle does not implement Clone because channels
// cannot be safely cloned. Each endpoint should have its own bundle.

impl<RoleKey> Default for SessionChannelBundle<RoleKey>
where
    RoleKey: Eq + std::hash::Hash + Clone,
{
    fn default() -> Self {
        Self::new()
    }
}

/// Rumpsteak endpoint wrapper that provides access to session-typed channels
///
/// Manages session-typed channels for communication with other roles.
/// Each endpoint contains a bundle of channels, one for each connected peer.
pub struct RumpsteakEndpoint<R>
where
    R: Role + Eq + std::hash::Hash + Clone,
{
    /// Bundle of session-typed channels
    channels: SessionChannelBundle<R>,
    /// The local role this endpoint represents
    local_role: R,
}

impl<R> RumpsteakEndpoint<R>
where
    R: Role + Eq + std::hash::Hash + Clone,
{
    /// Create a new endpoint for a role
    pub fn new(local_role: R) -> Self {
        Self {
            channels: SessionChannelBundle::new(),
            local_role,
        }
    }

    /// Register a session-typed channel with a peer role
    ///
    /// # Example
    /// ```ignore
    /// let mut endpoint = RumpsteakEndpoint::new(alice);
    /// endpoint.register_channel(bob, send_channel);
    /// ```
    pub fn register_channel<T: Any + Send + Sync + 'static>(&mut self, peer: R, channel: T) {
        self.channels.register(peer, channel);
    }

    /// Take a channel for a peer, removing it from the endpoint
    ///
    /// Used to perform session type operations that consume channels.
    /// After the operation, use `put_channel` to store the new state.
    pub fn take_channel(&mut self, peer: &R) -> Option<Box<dyn Any + Send + Sync>> {
        self.channels.take_channel(peer)
    }

    /// Put a channel back for a peer
    ///
    /// Used after session operations to store the new session state.
    pub fn put_channel<T: Any + Send + Sync + 'static>(&mut self, peer: R, channel: T) {
        self.channels.put_channel(peer, channel);
    }

    /// Check if a channel is registered for a peer
    pub fn has_channel(&self, peer: &R) -> bool {
        self.channels.has_channel(peer)
    }

    /// Remove a channel for a peer
    pub fn close_channel(&mut self, peer: &R) -> bool {
        tracing::debug!("Closing channel");
        self.channels.remove(peer)
    }

    /// Close all channels gracefully
    ///
    /// This should be called when shutting down the endpoint to ensure
    /// all resources are properly released.
    pub fn close_all_channels(&mut self) -> usize {
        let all_peers: Vec<R> = self
            .channels
            .all_metadata()
            .into_iter()
            .map(|(peer, _)| peer)
            .collect();

        let count = all_peers.len();
        for peer in all_peers {
            self.close_channel(&peer);
        }

        tracing::info!(closed = count, "Closed all channels");
        count
    }

    /// Check if all channels are closed
    pub fn is_all_closed(&self) -> bool {
        self.channels.all_metadata().is_empty()
    }

    /// Get count of active channels
    pub fn active_channel_count(&self) -> usize {
        self.channels.all_metadata().len()
    }

    /// Get the local role
    pub fn local_role(&self) -> &R {
        &self.local_role
    }

    /// Mark an operation performed on a peer's session
    pub fn mark_operation(&mut self, peer: &R, operation: &str) {
        self.channels.mark_operation(peer, operation);
    }

    /// Get metadata for a peer's session
    pub fn get_metadata(&self, peer: &R) -> Option<&SessionMetadata> {
        self.channels.get_metadata(peer)
    }

    /// Get all session metadata (for debugging/monitoring)
    pub fn all_metadata(&self) -> Vec<(R, &SessionMetadata)> {
        self.channels.all_metadata()
    }
}

// Note: RumpsteakEndpoint does not implement Clone because the channel
// bundle contains unique channels that cannot be safely cloned.

impl<R> Drop for RumpsteakEndpoint<R>
where
    R: Role + Eq + std::hash::Hash + Clone,
{
    fn drop(&mut self) {
        let active_count = self.active_channel_count();
        if active_count > 0 {
            tracing::warn!(
                active_channels = active_count,
                "RumpsteakEndpoint dropped with active channels - closing them"
            );
            self.close_all_channels();
        }
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
        msg: &Msg,
    ) -> Result<()> {
        // Serialize the message
        let serialized = bincode::serialize(msg)
            .map_err(|e| ChoreographyError::Transport(format!("Serialization failed: {}", e)))?;
        tracing::debug!(?to, size = serialized.len(), "Sending message");

        // Take the channel for this peer
        let channel_box = ep.take_channel(&to).ok_or_else(|| {
            ChoreographyError::Transport(format!("No channel registered for role: {:?}", to))
        })?;

        // Downcast to SimpleChannel
        let mut channel = *channel_box.downcast::<SimpleChannel>().map_err(|_| {
            ChoreographyError::Transport(
                "Failed to downcast channel - wrong channel type".to_string(),
            )
        })?;

        // Send the serialized message
        channel
            .send(serialized)
            .await
            .map_err(|e| ChoreographyError::Transport(format!("Send failed: {}", e)))?;

        // Put the channel back and mark operation
        ep.put_channel(to, channel);
        ep.channels.mark_operation(&to, "Send");

        Ok(())
    }

    async fn recv<Msg: DeserializeOwned + Send>(
        &mut self,
        ep: &mut Self::Endpoint,
        from: Self::Role,
    ) -> Result<Msg> {
        tracing::debug!(?from, "Receiving message");

        // Take the channel for this peer
        let channel_box = ep.take_channel(&from).ok_or_else(|| {
            ChoreographyError::Transport(format!("No channel registered for role: {:?}", from))
        })?;

        // Downcast to SimpleChannel
        let mut channel = *channel_box.downcast::<SimpleChannel>().map_err(|_| {
            ChoreographyError::Transport(
                "Failed to downcast channel - wrong channel type".to_string(),
            )
        })?;

        // Receive the serialized message
        let serialized = channel
            .recv()
            .await
            .map_err(|e| ChoreographyError::Transport(format!("Receive failed: {}", e)))?;

        tracing::debug!(?from, size = serialized.len(), "Received message");

        // Deserialize the message
        let msg: Msg = bincode::deserialize(&serialized)
            .map_err(|e| ChoreographyError::Transport(format!("Deserialization failed: {}", e)))?;

        // Put the channel back and mark operation
        ep.put_channel(from, channel);
        ep.channels.mark_operation(&from, "Recv");

        Ok(msg)
    }

    async fn choose(
        &mut self,
        ep: &mut Self::Endpoint,
        who: Self::Role,
        label: Label,
    ) -> Result<()> {
        tracing::debug!(?who, ?label, "Choosing branch");

        // Take the channel for this peer
        let channel_box = ep.take_channel(&who).ok_or_else(|| {
            ChoreographyError::Transport(format!("No channel registered for role: {:?}", who))
        })?;

        // Downcast to SimpleChannel
        let mut channel = *channel_box.downcast::<SimpleChannel>().map_err(|_| {
            ChoreographyError::Transport(
                "Failed to downcast channel - wrong channel type".to_string(),
            )
        })?;

        // Serialize and send the label
        let serialized = bincode::serialize(&label.0).map_err(|e| {
            ChoreographyError::Transport(format!("Label serialization failed: {}", e))
        })?;

        channel
            .send(serialized)
            .await
            .map_err(|e| ChoreographyError::Transport(format!("Choice send failed: {}", e)))?;

        // Put the channel back and mark operation
        ep.put_channel(who, channel);
        ep.mark_operation(&who, "Choose");

        Ok(())
    }

    async fn offer(&mut self, ep: &mut Self::Endpoint, from: Self::Role) -> Result<Label> {
        tracing::debug!(?from, "Offering choice");

        // Take the channel for this peer
        let channel_box = ep.take_channel(&from).ok_or_else(|| {
            ChoreographyError::Transport(format!("No channel registered for role: {:?}", from))
        })?;

        // Downcast to SimpleChannel
        let mut channel = *channel_box.downcast::<SimpleChannel>().map_err(|_| {
            ChoreographyError::Transport(
                "Failed to downcast channel - wrong channel type".to_string(),
            )
        })?;

        // Receive the serialized label
        let serialized = channel
            .recv()
            .await
            .map_err(|e| ChoreographyError::Transport(format!("Choice receive failed: {}", e)))?;

        // Deserialize the label
        let label_string: String = bincode::deserialize(&serialized).map_err(|e| {
            ChoreographyError::Transport(format!("Label deserialization failed: {}", e))
        })?;

        tracing::debug!(?from, label = ?label_string, "Received choice");

        // Put the channel back and mark operation
        ep.put_channel(from, channel);
        ep.mark_operation(&from, "Offer");

        // Convert String to &'static str by leaking (labels are small and long-lived)
        let label_str: &'static str = Box::leak(label_string.into_boxed_str());

        Ok(Label(label_str))
    }

    async fn with_timeout<F, T>(
        &mut self,
        _ep: &mut Self::Endpoint,
        _at: Self::Role,
        dur: Duration,
        body: F,
    ) -> Result<T>
    where
        F: std::future::Future<Output = Result<T>> + Send,
    {
        #[cfg(not(target_arch = "wasm32"))]
        {
            match tokio::time::timeout(dur, body).await {
                Ok(result) => result,
                Err(_) => Err(ChoreographyError::Timeout(dur)),
            }
        }

        #[cfg(target_arch = "wasm32")]
        {
            use futures::future::{select, Either};
            use futures::pin_mut;
            use wasm_timer::Delay;

            let timeout = Delay::new(dur);
            pin_mut!(body);
            pin_mut!(timeout);

            match select(body, timeout).await {
                Either::Left((result, _)) => result,
                Either::Right(_) => Err(ChoreographyError::Timeout(dur)),
            }
        }
    }
}
