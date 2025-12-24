/// Trait for abstracting work distribution to workers
/// Different implementations for mpsc, sockets, RPC, etc.
pub trait WorkDistributor<A, C>: Clone + Send + 'static {
    /// Send work assignment with completion token
    fn send_work(&self, assignment: A, completion: C);
}
