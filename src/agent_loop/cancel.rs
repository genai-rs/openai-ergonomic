//! Cancelling a running [`ToolLoop`](super::ToolLoop).

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use tokio::sync::Notify;

/// A signal that cancels a [`ToolLoop`](super::ToolLoop) run.
///
/// Clone it, pass one clone to [`ToolLoop::run_with`](super::ToolLoop::run_with), and call
/// [`cancel`](Self::cancel) on another (from another task, a UI handler, a timer).
///
/// Once cancelled it stays cancelled; use a new signal for the next run.
///
/// ```
/// use openai_ergonomic::CancellationSignal;
///
/// let signal = CancellationSignal::new();
/// let handle = signal.clone();
/// assert!(!signal.is_cancelled());
/// handle.cancel();
/// assert!(signal.is_cancelled());
/// ```
#[derive(Debug, Clone, Default)]
pub struct CancellationSignal {
    inner: Arc<Inner>,
}

#[derive(Debug, Default)]
struct Inner {
    cancelled: AtomicBool,
    notify: Notify,
}

impl CancellationSignal {
    /// A signal that is not cancelled.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Cancel: the run that holds a clone stops at its next await.
    pub fn cancel(&self) {
        self.inner.cancelled.store(true, Ordering::SeqCst);
        self.inner.notify.notify_waiters();
    }

    /// Whether [`cancel`](Self::cancel) was called.
    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.inner.cancelled.load(Ordering::SeqCst)
    }

    /// Resolves once the signal is cancelled (at once if it already is).
    pub async fn cancelled(&self) {
        loop {
            let notified = self.inner.notify.notified();
            futures::pin_mut!(notified);
            // Registered before the check, so a `cancel` between the two is not missed.
            notified.as_mut().enable();
            if self.is_cancelled() {
                return;
            }
            notified.await;
        }
    }
}
