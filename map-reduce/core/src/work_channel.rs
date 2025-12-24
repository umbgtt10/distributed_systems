/// Trait for abstracting work assignment channels
/// Different implementations for mpsc, sockets, RPC, etc.
pub trait WorkChannel<A, C>: Clone + Send + 'static {
    /// Send work assignment with completion token
    fn send_work(&self, assignment: A, completion: C);
}
