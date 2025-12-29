use map_reduce_core::shutdown_signal::ShutdownSignal;
use tokio_util::sync::CancellationToken;

/// Tokio CancellationToken-based shutdown signal
#[derive(Clone)]
pub struct ChannelShutdownSignal {
    token: CancellationToken,
}

impl ChannelShutdownSignal {
    pub fn new(token: CancellationToken) -> Self {
        Self { token }
    }
}

impl ShutdownSignal for ChannelShutdownSignal {
    fn is_cancelled(&self) -> bool {
        self.token.is_cancelled()
    }
}
