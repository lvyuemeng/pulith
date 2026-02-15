//! Throttled stream implementation for bandwidth limiting.
//!
//! This module provides a stream wrapper that limits the rate of data transfer
//! using a token bucket algorithm.

use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use bytes::Bytes;
use futures_util::{Stream, StreamExt};

use crate::rate::bandwidth::TokenBucket;
use crate::error::Error;

/// A stream that throttles the rate of data transfer.
/// 
/// This wrapper applies bandwidth limiting to an underlying stream by
/// acquiring tokens from a TokenBucket before yielding each chunk.
pub struct ThrottledStream<S> {
    inner: S,
    limiter: Arc<TokenBucket>,
    pending_chunk: Option<Bytes>,
}

impl<S> ThrottledStream<S>
where
    S: Stream<Item = std::result::Result<Bytes, Box<dyn std::error::Error + Send>>> + Unpin,
{
    /// Create a new ThrottledStream with the specified rate limit.
    /// 
    /// # Arguments
    /// 
    /// * `inner` - The underlying stream to throttle
    /// * `bytes_per_second` - Maximum transfer rate in bytes per second
pub fn new(inner: S, bytes_per_second: u64) -> Self {
        // Use a burst capacity of 1 second worth of data
        let capacity = bytes_per_second;
        let refill_rate = bytes_per_second;
        
        Self {
            inner,
            limiter: Arc::new(TokenBucket::new(capacity, refill_rate)),
            pending_chunk: None,
        }
    }
    
    /// Create a new ThrottledStream with custom bucket parameters.
    /// 
    /// # Arguments
    /// 
    /// * `inner` - The underlying stream to throttle
    /// * `bucket` - The TokenBucket to use for rate limiting
    pub fn with_bucket(inner: S, bucket: Arc<TokenBucket>) -> Self {
        Self {
            inner,
            limiter: bucket,
            pending_chunk: None,
        }
    }
}

impl<S> Stream for ThrottledStream<S>
where
    S: Stream<Item = std::result::Result<Bytes, Box<dyn std::error::Error + Send>>> + Unpin,
{
    type Item = std::result::Result<Bytes, Error>;
    
    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        // Get a mutable reference to the inner stream
        let this = self.get_mut();
        
        // If we have a pending chunk, check if we can acquire tokens
        if let Some(chunk) = this.pending_chunk.take() {
            let chunk_size = chunk.len();
            let limiter = Arc::clone(&this.limiter);
            
            // Try to acquire tokens immediately
            if limiter.try_acquire(chunk_size) {
                // Got tokens, return the chunk
                return Poll::Ready(Some(Ok(chunk)));
            } else {
                // Not enough tokens, put it back and spawn task to wait
                this.pending_chunk = Some(chunk.clone());
                
                let limiter_clone = Arc::clone(&this.limiter);
                let waker = cx.waker().clone();
                
                tokio::spawn(async move {
                    limiter_clone.acquire(chunk_size).await;
                    waker.wake();
                });
                
                return Poll::Pending;
            }
        }
        
        // Poll the inner stream
        match Pin::new(&mut this.inner).poll_next(cx) {
            Poll::Ready(Some(Ok(chunk))) => {
                // Store the chunk and try to acquire tokens
                this.pending_chunk = Some(chunk.clone());
                
                let chunk_size = chunk.len();
                let limiter = Arc::clone(&this.limiter);
                
                if limiter.try_acquire(chunk_size) {
                    // Got tokens immediately
                    this.pending_chunk = None;
                    Poll::Ready(Some(Ok(chunk)))
                } else {
                    // Need to wait for tokens
                    let limiter_clone = Arc::clone(&this.limiter);
                    let waker = cx.waker().clone();
                    
                    tokio::spawn(async move {
                        limiter_clone.acquire(chunk_size).await;
                        waker.wake();
                    });
                    
                    Poll::Pending
                }
            }
            Poll::Ready(Some(Err(e))) => {
                // Convert the error
                Poll::Ready(Some(Err(Error::Network(e.to_string()))))
            }
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }
}

/// An async-friendly throttled stream that properly handles token acquisition.
pub struct AsyncThrottledStream<S> {
    inner: S,
    limiter: Arc<TokenBucket>,
    pending_chunk: Option<Bytes>,
}

impl<S> AsyncThrottledStream<S>
where
    S: Stream<Item = std::result::Result<Bytes, Box<dyn std::error::Error + Send>>> + Unpin,
{
    /// Create a new AsyncThrottledStream with the specified rate limit.
    pub fn new(inner: S, bytes_per_second: u64) -> Self {
        let capacity = bytes_per_second;
        let refill_rate = bytes_per_second;
        
        Self {
            inner,
            limiter: Arc::new(TokenBucket::new(capacity, refill_rate)),
            pending_chunk: None,
        }
    }
}

impl<S> Stream for AsyncThrottledStream<S>
where
    S: Stream<Item = std::result::Result<Bytes, Box<dyn std::error::Error + Send>>> + Unpin,
{
    type Item = std::result::Result<Bytes, Error>;
    
    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();
        
        // If we have a pending chunk, yield it
        if let Some(chunk) = this.pending_chunk.take() {
            return Poll::Ready(Some(Ok(chunk)));
        }
        
        // Poll the inner stream
        match Pin::new(&mut this.inner).poll_next(cx) {
            Poll::Ready(Some(Ok(chunk))) => {
                // Store the chunk and return Pending to allow token acquisition
                this.pending_chunk = Some(chunk.clone());
                
                // Spawn a task to acquire tokens
                let limiter = Arc::clone(&this.limiter);
                let chunk_size = chunk.len();
                let waker = cx.waker().clone();
                
                tokio::spawn(async move {
                    limiter.acquire(chunk_size).await;
                    waker.wake();
                });
                
                Poll::Pending
            }
            Poll::Ready(Some(Err(e))) => {
                Poll::Ready(Some(Err(Error::Network(e.to_string()))))
            }
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures_util::stream;
    use std::time::{Duration, Instant};
    
#[tokio::test]
    async fn test_throttled_stream_basic() {
        // Create a stream of small chunks
        let chunks = vec![
            Ok::<_, Box<dyn std::error::Error + Send>>(Bytes::from("hi")),
            Ok(Bytes::from("hi")),
            Ok(Bytes::from("hi")),
        ];
        let stream = stream::iter(chunks);
        
        // Throttle to 100 bytes/second (high rate for test)
        let throttled = ThrottledStream::new(stream, 100);
        
        let results: Vec<_> = throttled.collect().await;
        
        // Should have received all chunks
        assert_eq!(results.len(), 3);
        assert!(results.iter().all(|r| r.is_ok()));
    }
    
#[tokio::test]
    async fn test_throttled_stream_rate_limit() {
        // Create a stream with medium chunks
        let chunks = vec![
            Ok::<_, Box<dyn std::error::Error + Send>>(Bytes::from(vec![0u8; 50])),
            Ok(Bytes::from(vec![0u8; 50])),
        ];
        let stream = stream::iter(chunks);
        
        // Throttle to 100 bytes/second (should allow both chunks quickly)
        let throttled = ThrottledStream::new(stream, 100);
        
        let results: Vec<_> = throttled.collect().await;
        
        assert_eq!(results.len(), 2);
        assert!(results.iter().all(|r| r.is_ok()));
    }
}