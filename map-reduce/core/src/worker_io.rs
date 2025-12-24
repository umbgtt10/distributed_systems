use async_trait::async_trait;

/// Trait for receiving work assignments asynchronously
#[async_trait]
pub trait AsyncWorkReceiver<A, C>: Send {
    /// Receive the next work assignment
    /// Returns None if the channel is closed
    async fn recv(&mut self) -> Option<(A, C)>;
}

/// Trait for sending completion signals asynchronously
#[async_trait]
pub trait AsyncCompletionSender: Send + Clone + Sync {
    /// Send a completion signal (success or failure)
    /// Returns true if the signal was sent successfully, false otherwise
    async fn send(&self, result: Result<usize, ()>) -> bool;
}
