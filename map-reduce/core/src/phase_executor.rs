use crate::worker::Worker;

/// Trait for executing a phase (map or reduce) with fault tolerance
/// This abstracts the entire work distribution pattern:
/// - Setting up completion signaling
/// - Initial work assignment
/// - Dynamic reassignment as workers complete
/// - Worker shutdown
pub trait PhaseExecutor: Send {
    /// The type of worker this executor manages
    type Worker: Worker;

    /// Execute a phase by distributing assignments to workers
    /// This method handles the complete lifecycle:
    /// - Assigns initial work
    /// - Waits for completions and reassigns dynamically
    /// - Waits for all workers to finish
    fn execute(
        &mut self,
        workers: Vec<Self::Worker>,
        assignments: Vec<<Self::Worker as Worker>::Assignment>,
    ) -> impl std::future::Future<Output = ()> + Send
    where
        <Self::Worker as Worker>::Assignment: Clone;
}
