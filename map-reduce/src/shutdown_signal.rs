/// Trait for shutdown signaling
pub trait ShutdownSignal: Clone + Send + 'static {
    fn is_cancelled(&self) -> bool;
}
