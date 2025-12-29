/// Trait for abstracting work distribution to workers
/// Different implementations for mpsc, sockets, RPC, etc.
pub trait WorkSender<A, C>: Clone + Send + 'static {
    /// Send initialization sender to worker
    fn initialize(&self, sender: C);

    /// Send work assignment with completion sender
    fn send_work(&self, assignment: A, completion: C);
}
