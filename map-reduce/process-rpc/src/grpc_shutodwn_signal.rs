use map_reduce_core::shutdown_signal::ShutdownSignal;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct DummyShutdownSignal;

impl ShutdownSignal for DummyShutdownSignal {
    fn is_cancelled(&self) -> bool {
        false
    }
}
