//! Client-side request spacing.

use std::sync::{Mutex, PoisonError};
use std::time::Duration;

use tokio::time::Instant;

/// A next-slot limiter: requests are spaced evenly at `interval`, without
/// bursts. Each call to [`RateLimiter::acquire`] reserves the next free slot
/// and sleeps until it starts. A cancelled `acquire` still consumes its slot.
#[derive(Debug)]
pub(crate) struct RateLimiter {
    interval: Duration,
    next: Mutex<Option<Instant>>,
}

impl RateLimiter {
    /// Allows `per_minute` requests per minute. `per_minute` must be non-zero.
    pub(crate) fn per_minute(per_minute: u32) -> Self {
        debug_assert!(per_minute > 0);
        Self {
            interval: Duration::from_secs(60) / per_minute.max(1),
            next: Mutex::new(None),
        }
    }

    /// Waits until the next request slot is available.
    pub(crate) async fn acquire(&self) {
        acquire_all(&[self]).await;
    }
}

/// Waits for the first slot that is free on every limiter in `limiters`.
///
/// The slot is reserved on all of them atomically (their locks are taken in
/// slice order and held together), so each limiter's next slot is one
/// interval after the shared send time.
pub(crate) async fn acquire_all(limiters: &[&RateLimiter]) {
    let slot = {
        let mut guards: Vec<_> = limiters
            .iter()
            .map(|limiter| limiter.next.lock().unwrap_or_else(PoisonError::into_inner))
            .collect();
        let now = Instant::now();
        let slot = guards
            .iter()
            .filter_map(|next| **next)
            .fold(now, Instant::max);
        for (next, limiter) in guards.iter_mut().zip(limiters) {
            **next = Some(slot + limiter.interval);
        }
        slot
    };
    tokio::time::sleep_until(slot).await;
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;

    #[tokio::test(start_paused = true)]
    async fn spaces_requests_evenly() {
        let limiter = RateLimiter::per_minute(20);
        let start = Instant::now();
        let mut offsets = Vec::new();
        for _ in 0..4 {
            limiter.acquire().await;
            offsets.push(start.elapsed());
        }
        let secs = |s| Duration::from_secs(s);
        assert_eq!(offsets, [secs(0), secs(3), secs(6), secs(9)]);
    }

    #[tokio::test(start_paused = true)]
    async fn idle_time_is_not_banked() {
        let limiter = RateLimiter::per_minute(60);
        limiter.acquire().await;
        tokio::time::sleep(Duration::from_secs(10)).await;
        let start = Instant::now();
        limiter.acquire().await;
        limiter.acquire().await;
        assert_eq!(start.elapsed(), Duration::from_secs(1));
    }

    #[tokio::test(start_paused = true)]
    async fn concurrent_callers_share_slots() {
        let limiter = Arc::new(RateLimiter::per_minute(30));
        let start = Instant::now();
        let tasks: Vec<_> = (0..3)
            .map(|_| {
                let limiter = Arc::clone(&limiter);
                tokio::spawn(async move {
                    limiter.acquire().await;
                    start.elapsed()
                })
            })
            .collect();
        let mut offsets = Vec::new();
        for task in tasks {
            offsets.push(task.await.unwrap());
        }
        offsets.sort();
        let secs = |s| Duration::from_secs(s);
        assert_eq!(offsets, [secs(0), secs(2), secs(4)]);
    }
}
