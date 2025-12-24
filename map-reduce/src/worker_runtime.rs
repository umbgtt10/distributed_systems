/// Trait for abstracting worker runtime (tasks, threads, processes)
pub trait WorkerRuntime: Send + 'static {
    type Handle: Send;
    type Error: std::fmt::Display + Send;

    /// Spawn a worker task/thread/process
    fn spawn<F, Fut>(f: F) -> Self::Handle
    where
        F: FnOnce() -> Fut + Send + 'static,
        Fut: std::future::Future<Output = ()> + Send + 'static;

    /// Wait for the worker to complete
    async fn join(handle: Self::Handle) -> Result<(), Self::Error>;
}
