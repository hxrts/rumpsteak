// Middleware layers for effect handlers
//
// These composable handlers allow adding cross-cutting concerns like
// tracing, metrics, fault injection, etc. without modifying generated code.

use async_trait::async_trait;
use serde::{Serialize, de::DeserializeOwned};
use std::time::{Duration, Instant};
use tracing::{debug, trace, warn};

use crate::effects::{ChoreoHandler, Result, Label};

/// Tracing middleware that logs all choreographic operations
pub struct Trace<H> {
    inner: H,
    #[allow(dead_code)]
    prefix: String,
}

impl<H> Trace<H> {
    pub fn new(inner: H) -> Self {
        Self::with_prefix(inner, "choreo")
    }
    
    pub fn with_prefix(inner: H, prefix: impl Into<String>) -> Self {
        Self {
            inner,
            prefix: prefix.into(),
        }
    }
}

#[async_trait]
impl<H: ChoreoHandler + Send> ChoreoHandler for Trace<H> {
    type Role = H::Role;
    type Endpoint = H::Endpoint;

    async fn send<M: Serialize + Send + Sync>(
        &mut self, 
        ep: &mut Self::Endpoint, 
        to: Self::Role, 
        msg: &M
    ) -> Result<()> {
        let start = Instant::now();
        trace!(?to, "send: start");
        let result = self.inner.send(ep, to, msg).await;
        let duration = start.elapsed();
        match &result {
            Ok(()) => debug!(?to, ?duration, "send: success"),
            Err(e) => warn!(?to, ?duration, error = %e, "send: failed"),
        }
        result
    }

    async fn recv<M: DeserializeOwned + Send>(
        &mut self, 
        ep: &mut Self::Endpoint, 
        from: Self::Role
    ) -> Result<M> {
        let start = Instant::now();
        trace!(?from, "recv: start");
        let result = self.inner.recv(ep, from).await;
        let duration = start.elapsed();
        match &result {
            Ok(_) => debug!(?from, ?duration, "recv: success"),
            Err(e) => warn!(?from, ?duration, error = %e, "recv: failed"),
        }
        result
    }

    async fn choose(
        &mut self, 
        ep: &mut Self::Endpoint, 
        who: Self::Role, 
        label: Label
    ) -> Result<()> {
        debug!(?who, ?label, "choose");
        self.inner.choose(ep, who, label).await
    }

    async fn offer(
        &mut self, 
        ep: &mut Self::Endpoint, 
        from: Self::Role
    ) -> Result<Label> {
        trace!(?from, "offer: waiting");
        let label = self.inner.offer(ep, from).await?;
        debug!(?from, ?label, "offer: received");
        Ok(label)
    }

    async fn with_timeout<F, T>(
        &mut self,
        ep: &mut Self::Endpoint,
        at: Self::Role,
        dur: Duration,
        body: F,
    ) -> Result<T>
    where
        F: std::future::Future<Output = Result<T>> + Send
    {
        debug!(?at, ?dur, "timeout: start");
        let start = Instant::now();
        let result = self.inner.with_timeout(ep, at, dur, body).await;
        let elapsed = start.elapsed();
        match &result {
            Ok(_) => debug!(?at, ?elapsed, "timeout: completed"),
            Err(e) => warn!(?at, ?elapsed, error = %e, "timeout: failed"),
        }
        result
    }
}

/// Metrics collection middleware
pub struct Metrics<H> {
    inner: H,
    send_count: std::sync::atomic::AtomicU64,
    recv_count: std::sync::atomic::AtomicU64,
    error_count: std::sync::atomic::AtomicU64,
}

impl<H> Metrics<H> {
    pub fn new(inner: H) -> Self {
        Self {
            inner,
            send_count: std::sync::atomic::AtomicU64::new(0),
            recv_count: std::sync::atomic::AtomicU64::new(0),
            error_count: std::sync::atomic::AtomicU64::new(0),
        }
    }
    
    pub fn send_count(&self) -> u64 {
        self.send_count.load(std::sync::atomic::Ordering::Relaxed)
    }
    
    pub fn recv_count(&self) -> u64 {
        self.recv_count.load(std::sync::atomic::Ordering::Relaxed)
    }
    
    pub fn error_count(&self) -> u64 {
        self.error_count.load(std::sync::atomic::Ordering::Relaxed)
    }
}

#[async_trait]
impl<H: ChoreoHandler + Send> ChoreoHandler for Metrics<H> {
    type Role = H::Role;
    type Endpoint = H::Endpoint;

    async fn send<M: Serialize + Send + Sync>(
        &mut self, 
        ep: &mut Self::Endpoint, 
        to: Self::Role, 
        msg: &M
    ) -> Result<()> {
        let result = self.inner.send(ep, to, msg).await;
        if result.is_ok() {
            self.send_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        } else {
            self.error_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }
        result
    }

    async fn recv<M: DeserializeOwned + Send>(
        &mut self, 
        ep: &mut Self::Endpoint, 
        from: Self::Role
    ) -> Result<M> {
        let result = self.inner.recv(ep, from).await;
        if result.is_ok() {
            self.recv_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        } else {
            self.error_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }
        result
    }

    async fn choose(
        &mut self, 
        ep: &mut Self::Endpoint, 
        who: Self::Role, 
        label: Label
    ) -> Result<()> {
        self.inner.choose(ep, who, label).await
    }

    async fn offer(
        &mut self, 
        ep: &mut Self::Endpoint, 
        from: Self::Role
    ) -> Result<Label> {
        self.inner.offer(ep, from).await
    }

    async fn with_timeout<F, T>(
        &mut self,
        ep: &mut Self::Endpoint,
        at: Self::Role,
        dur: Duration,
        body: F,
    ) -> Result<T>
    where
        F: std::future::Future<Output = Result<T>> + Send
    {
        self.inner.with_timeout(ep, at, dur, body).await
    }
}

/// Retry middleware with exponential backoff
pub struct Retry<H> {
    inner: H,
    max_retries: usize,
    base_delay: Duration,
}

impl<H> Retry<H> {
    pub fn new(inner: H) -> Self {
        Self {
            inner,
            max_retries: 3,
            base_delay: Duration::from_millis(100),
        }
    }
    
    pub fn with_config(inner: H, max_retries: usize, base_delay: Duration) -> Self {
        Self {
            inner,
            max_retries,
            base_delay,
        }
    }
}

#[async_trait]
impl<H: ChoreoHandler + Send> ChoreoHandler for Retry<H> {
    type Role = H::Role;
    type Endpoint = H::Endpoint;

    async fn send<M: Serialize + Send + Sync>(
        &mut self, 
        ep: &mut Self::Endpoint, 
        to: Self::Role, 
        msg: &M
    ) -> Result<()> {
        let mut retries = 0;
        loop {
            match self.inner.send(ep, to, msg).await {
                Ok(()) => return Ok(()),
                Err(_e) if retries < self.max_retries => {
                    retries += 1;
                    let delay = self.base_delay * (1 << (retries - 1));
                    debug!(?to, ?retries, ?delay, "send failed, retrying");
                    tokio::time::sleep(delay).await;
                }
                Err(e) => return Err(e),
            }
        }
    }

    async fn recv<M: DeserializeOwned + Send>(
        &mut self, 
        ep: &mut Self::Endpoint, 
        from: Self::Role
    ) -> Result<M> {
        // Recv typically shouldn't be retried as it changes protocol state
        self.inner.recv(ep, from).await
    }

    async fn choose(
        &mut self, 
        ep: &mut Self::Endpoint, 
        who: Self::Role, 
        label: Label
    ) -> Result<()> {
        self.inner.choose(ep, who, label).await
    }

    async fn offer(
        &mut self, 
        ep: &mut Self::Endpoint, 
        from: Self::Role
    ) -> Result<Label> {
        self.inner.offer(ep, from).await
    }

    async fn with_timeout<F, T>(
        &mut self,
        ep: &mut Self::Endpoint,
        at: Self::Role,
        dur: Duration,
        body: F,
    ) -> Result<T>
    where
        F: std::future::Future<Output = Result<T>> + Send
    {
        self.inner.with_timeout(ep, at, dur, body).await
    }
}

/// Fault injection middleware for testing
#[cfg(feature = "test-utils")]
pub struct FaultInjection<H> {
    inner: H,
    failure_rate: f32,
    delay_range: Option<(Duration, Duration)>,
    rng: rand::rngs::StdRng,
}

#[cfg(feature = "test-utils")]
impl<H> FaultInjection<H> {
    pub fn new(inner: H, failure_rate: f32) -> Self {
        use rand::SeedableRng;
        Self {
            inner,
            failure_rate,
            delay_range: None,
            rng: rand::rngs::StdRng::from_entropy(),
        }
    }
    
    pub fn with_delays(mut self, min: Duration, max: Duration) -> Self {
        self.delay_range = Some((min, max));
        self
    }
}

#[cfg(feature = "test-utils")]
#[async_trait]
impl<H: ChoreoHandler + Send> ChoreoHandler for FaultInjection<H> {
    type Role = H::Role;
    type Endpoint = H::Endpoint;

    async fn send<M: Serialize + Send + Sync>(
        &mut self, 
        ep: &mut Self::Endpoint, 
        to: Self::Role, 
        msg: &M
    ) -> Result<()> {
        use rand::Rng;
        
        // Inject random delay
        if let Some((min, max)) = self.delay_range {
            let delay_ms = self.rng.gen_range(min.as_millis()..=max.as_millis());
            tokio::time::sleep(Duration::from_millis(delay_ms as u64)).await;
        }
        
        // Inject random failure
        if self.rng.gen::<f32>() < self.failure_rate {
            return Err(crate::effects::ChoreographyError::Transport(
                "Injected fault".into()
            ));
        }
        
        self.inner.send(ep, to, msg).await
    }

    async fn recv<M: DeserializeOwned + Send>(
        &mut self, 
        ep: &mut Self::Endpoint, 
        from: Self::Role
    ) -> Result<M> {
        self.inner.recv(ep, from).await
    }

    async fn choose(
        &mut self, 
        ep: &mut Self::Endpoint, 
        who: Self::Role, 
        label: Label
    ) -> Result<()> {
        self.inner.choose(ep, who, label).await
    }

    async fn offer(
        &mut self, 
        ep: &mut Self::Endpoint, 
        from: Self::Role
    ) -> Result<Label> {
        self.inner.offer(ep, from).await
    }

    async fn with_timeout<F, T>(
        &mut self,
        ep: &mut Self::Endpoint,
        at: Self::Role,
        dur: Duration,
        body: F,
    ) -> Result<T>
    where
        F: std::future::Future<Output = Result<T>> + Send
    {
        self.inner.with_timeout(ep, at, dur, body).await
    }
}