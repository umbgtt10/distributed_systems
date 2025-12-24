use async_trait::async_trait;
use futures::{SinkExt, StreamExt};
use map_reduce_core::completion_signaling::CompletionSignaling;
use map_reduce_core::worker_io::AsyncCompletionSender;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;
use tokio_util::codec::{Framed, LengthDelimitedCodec};
use bytes::Bytes;

#[derive(Serialize, Deserialize, Debug)]
struct CompletionMessage {
    worker_id: usize,
    success: bool,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct RpcCompletionToken {
    server_addr: SocketAddr,
    worker_id: usize,
}

#[async_trait]
impl AsyncCompletionSender for RpcCompletionToken {
    async fn send(&self, result: Result<usize, ()>) -> bool {
        let msg = CompletionMessage {
            worker_id: self.worker_id,
            success: result.is_ok(),
        };

        let json = match serde_json::to_vec(&msg) {
            Ok(j) => j,
            Err(_) => return false,
        };

        // Retry loop for connecting to coordinator
        for _ in 0..5 {
            if let Ok(stream) = TcpStream::connect(self.server_addr).await {
                let mut framed = Framed::new(stream, LengthDelimitedCodec::new());
                if framed.send(Bytes::from(json.clone())).await.is_ok() {
                    return true;
                }
            }
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
        false
    }
}

pub struct RpcCompletionSignaling {
    port: u16,
    rx: mpsc::Receiver<(usize, bool)>,
}

impl CompletionSignaling for RpcCompletionSignaling {
    type Token = RpcCompletionToken;

    fn setup(_num_workers: usize) -> Self {
        let (tx, rx) = mpsc::channel(100);
        let (port_tx, port_rx) = std::sync::mpsc::channel();

        tokio::spawn(async move {
            let listener = match TcpListener::bind("0.0.0.0:0").await {
                Ok(l) => l,
                Err(e) => {
                    eprintln!("Failed to bind completion listener: {}", e);
                    return;
                }
            };

            if let Ok(addr) = listener.local_addr() {
                let _ = port_tx.send(addr.port());
            } else {
                return;
            }

            loop {
                if let Ok((stream, _)) = listener.accept().await {
                    let tx = tx.clone();
                    tokio::spawn(async move {
                        let mut framed = Framed::new(stream, LengthDelimitedCodec::new());
                        if let Some(Ok(bytes)) = framed.next().await {
                            if let Ok(msg) = serde_json::from_slice::<CompletionMessage>(&bytes) {
                                let _ = tx.send((msg.worker_id, msg.success)).await;
                            }
                        }
                    });
                }
            }
        });

        let port = port_rx.recv().unwrap_or(0);
        Self { port, rx }
    }

    fn get_token(&self, worker_id: usize) -> Self::Token {
        RpcCompletionToken {
            server_addr: format!("127.0.0.1:{}", self.port).parse().unwrap(),
            worker_id,
        }
    }

    async fn wait_next(&mut self) -> Option<Result<usize, usize>> {
        self.rx
            .recv()
            .await
            .map(|(id, success)| if success { Ok(id) } else { Err(id) })
    }

    async fn reset_worker(&mut self, worker_id: usize) -> Self::Token {
        self.get_token(worker_id)
    }
}
