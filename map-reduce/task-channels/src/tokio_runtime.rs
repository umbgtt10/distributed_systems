use map_reduce_core::shutdown_signal::ShutdownSignal;
use map_reduce_core::worker_runtime::WorkerRuntime;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

/// Tokio task-based runtime
pub struct TokioRuntime;

impl WorkerRuntime for TokioRuntime {
    type Handle = JoinHandle<()>;
    type Error = tokio::task::JoinError;

    fn spawn<F, Fut>(f: F) -> Self::Handle
    where
        F: FnOnce() -> Fut + Send + 'static,
        Fut: std::future::Future<Output = ()> + Send + 'static,
    {
        tokio::spawn(f())
    }

    async fn join(handle: Self::Handle) -> Result<(), Self::Error> {
        handle.await
    }
}

/// Tokio CancellationToken-based shutdown signal
#[derive(Clone)]
pub struct TokenShutdownSignal {
    token: CancellationToken,
}

impl TokenShutdownSignal {
    pub fn new(token: CancellationToken) -> Self {
        Self { token }
    }
}

impl ShutdownSignal for TokenShutdownSignal {
    fn is_cancelled(&self) -> bool {
        self.token.is_cancelled()
    }
}
