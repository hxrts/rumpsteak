// Effect Handler Architecture for Choreographic Programming
//
// This module provides a clean effect boundary between pure choreographic logic
// and runtime transport implementations. It allows for testable, composable,
// and runtime-agnostic protocol implementations.

use async_trait::async_trait;
use serde::{Serialize, de::DeserializeOwned};
use std::time::Duration;
use std::fmt::Debug;
use thiserror::Error;

/// Roles are usually an enum generated per choreography.
pub trait RoleId: Copy + Eq + std::hash::Hash + Debug + Send + Sync {}
impl<T: Copy + Eq + std::hash::Hash + Debug + Send + Sync> RoleId for T {}

/// Labels identify branches in internal/external choice.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Label(pub &'static str);

/// Session endpoint is runtime-specific (e.g., Rumpsteak channel bundle).
/// The generated code will be generic over it.
pub trait Endpoint: Send {}
impl<T: Send> Endpoint for T {}

/// Errors that can occur during choreographic execution
#[derive(Debug, Error)]
pub enum ChoreographyError {
    #[error("Transport error: {0}")]
    Transport(String),
    
    #[error("Serialization error: {0}")]
    Serialization(String),
    
    #[error("Timeout after {0:?}")]
    Timeout(Duration),
    
    #[error("Protocol violation: {0}")]
    ProtocolViolation(String),
    
    #[error("Role {0:?} not found in this choreography")]
    UnknownRole(String),
}

/// Result type for choreography operations
pub type Result<T> = std::result::Result<T, ChoreographyError>;

/// The core effect handler trait that abstracts all communication effects
#[async_trait]
pub trait ChoreoHandler: Send {
    type Role: RoleId;
    type Endpoint: Endpoint;

    /// Send an arbitrary message to `to`.
    async fn send<M: Serialize + Send + Sync>(
        &mut self, 
        ep: &mut Self::Endpoint, 
        to: Self::Role, 
        msg: &M
    ) -> Result<()>;

    /// Receive a strongly-typed message *from* `from`.
    async fn recv<M: DeserializeOwned + Send>(
        &mut self, 
        ep: &mut Self::Endpoint, 
        from: Self::Role
    ) -> Result<M>;

    /// Internal choice taken *at* `who` with a selected label (others will `offer`).
    async fn choose(
        &mut self, 
        ep: &mut Self::Endpoint, 
        who: Self::Role, 
        label: Label
    ) -> Result<()>;

    /// External choice offered *by* `from`: returns the chosen label (then locals branch on it).
    async fn offer(
        &mut self, 
        ep: &mut Self::Endpoint, 
        from: Self::Role
    ) -> Result<Label>;

    /// Timeout/cancellation boundary at a role (projects to timers/abort).
    async fn with_timeout<F, T>(
        &mut self,
        ep: &mut Self::Endpoint,
        at: Self::Role,
        dur: Duration,
        body: F,
    ) -> Result<T>
    where
        F: std::future::Future<Output = Result<T>> + Send;
        
    /// Broadcast a message to multiple recipients
    async fn broadcast<M: Serialize + Send + Sync>(
        &mut self,
        ep: &mut Self::Endpoint,
        recipients: &[Self::Role],
        msg: &M,
    ) -> Result<()> {
        for &recipient in recipients {
            self.send(ep, recipient, msg).await?;
        }
        Ok(())
    }
    
    /// Parallel send to multiple recipients (optimized broadcast)
    async fn parallel_send<M: Serialize + Send + Sync>(
        &mut self,
        ep: &mut Self::Endpoint,
        sends: &[(Self::Role, M)],
    ) -> Result<()> {
        // Default implementation: sequential sends
        for (recipient, msg) in sends {
            self.send(ep, *recipient, msg).await?;
        }
        Ok(())
    }
}

/// Extension trait for handler lifecycle management
#[async_trait]
pub trait ChoreoHandlerExt: ChoreoHandler {
    /// Setup phase - establish connections, initialize state
    async fn setup(
        &mut self, 
        role: Self::Role
    ) -> Result<Self::Endpoint>;
    
    /// Teardown phase - close connections, cleanup
    async fn teardown(
        &mut self, 
        ep: Self::Endpoint
    ) -> Result<()>;
}

/// A no-op handler for testing pure choreographic logic
pub struct NoOpHandler<R: RoleId> {
    _phantom: std::marker::PhantomData<R>,
}

impl<R: RoleId> NoOpHandler<R> {
    pub fn new() -> Self {
        Self {
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<R: RoleId> Default for NoOpHandler<R> {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl<R: RoleId + 'static> ChoreoHandler for NoOpHandler<R> {
    type Role = R;
    type Endpoint = ();

    async fn send<M: Serialize + Send + Sync>(
        &mut self, 
        _ep: &mut Self::Endpoint, 
        _to: Self::Role, 
        _msg: &M
    ) -> Result<()> {
        Ok(())
    }

    async fn recv<M: DeserializeOwned + Send>(
        &mut self, 
        _ep: &mut Self::Endpoint, 
        _from: Self::Role
    ) -> Result<M> {
        Err(ChoreographyError::Transport("NoOpHandler cannot receive".into()))
    }

    async fn choose(
        &mut self, 
        _ep: &mut Self::Endpoint, 
        _who: Self::Role, 
        _label: Label
    ) -> Result<()> {
        Ok(())
    }

    async fn offer(
        &mut self, 
        _ep: &mut Self::Endpoint, 
        _from: Self::Role
    ) -> Result<Label> {
        Err(ChoreographyError::Transport("NoOpHandler cannot offer".into()))
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