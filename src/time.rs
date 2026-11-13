//! Clocks, timers and background tasks: std's and tokio's natively; on WebAssembly
//! (`target_family = "wasm"`) `web-time`'s clocks (`std::time::Instant::now` and
//! `SystemTime::now` panic on `wasm32-unknown-unknown`), the JavaScript host's `setTimeout`
//! (`gloo-timers`) and `wasm_bindgen_futures::spawn_local`.
//!
//! The same approach as `timvw-rust-runtime`'s timer, whose types are crate-private.

use std::future::Future;
use std::time::Duration;

/// A monotonic clock: `std::time::Instant` natively, `web_time::Instant` (`performance.now()`)
/// on WebAssembly.
#[cfg(not(target_family = "wasm"))]
pub use std::time::Instant;
/// A monotonic clock: `std::time::Instant` natively, `web_time::Instant` (`performance.now()`)
/// on WebAssembly.
#[cfg(target_family = "wasm")]
pub use web_time::Instant;

/// The wall clock: `std::time::SystemTime` natively, `web_time::SystemTime` (`Date.now()`) on
/// WebAssembly.
#[cfg(not(target_family = "wasm"))]
#[allow(unused_imports)] // only the `webhook` feature reads the wall clock
pub use std::time::{SystemTime, UNIX_EPOCH};
/// The wall clock: `std::time::SystemTime` natively, `web_time::SystemTime` (`Date.now()`) on
/// WebAssembly.
#[cfg(target_family = "wasm")]
#[allow(unused_imports)] // only the `webhook` feature reads the wall clock
pub use web_time::{SystemTime, UNIX_EPOCH};

/// Waits for `duration`: tokio's timer natively (a tokio runtime with the time driver must be
/// running), the host's `setTimeout` on WebAssembly.
#[cfg(not(target_family = "wasm"))]
pub async fn sleep(duration: Duration) {
    tokio::time::sleep(duration).await;
}

/// Waits for `duration`: tokio's timer natively (a tokio runtime with the time driver must be
/// running), the host's `setTimeout` on WebAssembly.
#[cfg(target_family = "wasm")]
pub async fn sleep(duration: Duration) {
    let deadline = Instant::now() + duration;
    loop {
        let left = deadline.saturating_duration_since(Instant::now());
        if left.is_zero() {
            return;
        }
        gloo_timers::future::TimeoutFuture::new(timeout_millis(left)).await;
    }
}

/// A delay for `setTimeout`: whole milliseconds, rounded up, at most 2^31 - 1 (a longer one is
/// waited for in steps; a timer that fires early is armed again).
#[cfg_attr(not(target_family = "wasm"), allow(dead_code))]
fn timeout_millis(delay: Duration) -> u32 {
    const MAX: u128 = i32::MAX as u128;
    #[allow(clippy::cast_possible_truncation)]
    let millis = delay.as_nanos().div_ceil(1_000_000).min(MAX) as u32;
    millis
}

/// Starts background tasks: on the tokio runtime that was current when it was made natively, on
/// the JavaScript host's event loop on WebAssembly.
#[derive(Clone, Debug)]
pub struct Spawner {
    #[cfg(not(target_family = "wasm"))]
    handle: tokio::runtime::Handle,
}

impl Spawner {
    /// The current tokio runtime's spawner natively (`None` outside one); always one on
    /// WebAssembly.
    #[cfg_attr(target_family = "wasm", allow(clippy::unnecessary_wraps))]
    pub fn current() -> Option<Self> {
        #[cfg(not(target_family = "wasm"))]
        {
            tokio::runtime::Handle::try_current()
                .ok()
                .map(|handle| Self { handle })
        }
        #[cfg(target_family = "wasm")]
        {
            Some(Self {})
        }
    }

    /// Runs `future` in the background.
    #[cfg(not(target_family = "wasm"))]
    pub fn spawn<F>(&self, future: F)
    where
        F: Future<Output = ()> + Send + 'static,
    {
        drop(self.handle.spawn(future));
    }

    /// Runs `future` in the background: `wasm_bindgen_futures::spawn_local`, so it need not be
    /// `Send`.
    #[cfg(target_family = "wasm")]
    #[allow(clippy::unused_self)]
    pub fn spawn<F>(&self, future: F)
    where
        F: Future<Output = ()> + 'static,
    {
        wasm_bindgen_futures::spawn_local(future);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timeouts_round_up_and_stay_in_range() {
        assert_eq!(timeout_millis(Duration::ZERO), 0);
        assert_eq!(timeout_millis(Duration::from_nanos(1)), 1);
        assert_eq!(timeout_millis(Duration::from_micros(1500)), 2);
        assert_eq!(timeout_millis(Duration::from_secs(2)), 2000);
        assert_eq!(
            timeout_millis(Duration::from_secs(100 * 86_400)),
            i32::MAX as u32
        );
    }
}
