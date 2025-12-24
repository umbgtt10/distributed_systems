use async_trait::async_trait;
use map_reduce_core::worker_io::{AsyncCompletionSender, AsyncWorkReceiver};
use tokio::sync::mpsc;

pub struct ChannelWorkReceiver<A, C> {
    pub rx: mpsc::Receiver<(A, C)>,
}

#[async_trait]
impl<A, C> AsyncWorkReceiver<A, C> for ChannelWorkReceiver<A, C>
where
    A: Send,
    C: Send,
{
    async fn recv(&mut self) -> Option<(A, C)> {
        self.rx.recv().await
    }
}

#[derive(Clone)]
pub struct ChannelCompletionSender {
    pub tx: mpsc::Sender<Result<usize, ()>>,
}

#[async_trait]
impl AsyncCompletionSender for ChannelCompletionSender {
    async fn send(&self, result: Result<usize, ()>) -> bool {
        self.tx.send(result).await.is_ok()
    }
}
