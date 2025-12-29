use async_trait::async_trait;
use map_reduce_core::completion_signaling::CompletionSignaling;
use map_reduce_core::worker_io::CompletionSender;
use serde::{Deserialize, Serialize};
use tonic::transport::{Channel, Server};
use tonic::{Request, Response, Status};

use crate::rpc::proto;
use proto::completion_service_client::CompletionServiceClient;
use proto::completion_service_server::{
    CompletionService as CompletionServiceTrait, CompletionServiceServer,
};
use proto::{CompletionAck, CompletionMessage};

/// gRPC Completion Token
/// Sent to workers to report completion back to coordinator
#[derive(Clone, Serialize, Deserialize, Default)]
pub struct GrpcCompletionToken {
    server_addr: String,
    worker_id: usize,
}

#[async_trait]
impl CompletionSender for GrpcCompletionToken {
    async fn send(&self, result: Result<usize, ()>) -> bool {
        let endpoint = format!("http://{}", self.server_addr);

        // Retry logic for connecting to coordinator
        for _ in 0..5 {
            if let Ok(channel) = Channel::from_shared(endpoint.clone())
                .unwrap()
                .connect()
                .await
            {
                let mut client = CompletionServiceClient::new(channel);
                let request = tonic::Request::new(CompletionMessage {
                    worker_id: self.worker_id as u64,
                    success: result.is_ok(),
                });

                if client.report_completion(request).await.is_ok() {
                    return true;
                }
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }
        false
    }
}

/// gRPC Completion Service implementation
struct CompletionServiceImpl {
    tx: tokio::sync::mpsc::Sender<(usize, bool)>,
}

#[tonic::async_trait]
impl CompletionServiceTrait for CompletionServiceImpl {
    async fn report_completion(
        &self,
        request: Request<CompletionMessage>,
    ) -> Result<Response<CompletionAck>, Status> {
        let msg = request.into_inner();

        self.tx
            .send((msg.worker_id as usize, msg.success))
            .await
            .map_err(|_| Status::internal("Failed to queue completion"))?;

        Ok(Response::new(CompletionAck { received: true }))
    }
}

/// gRPC Completion Signaling
/// Coordinator receives completion notifications from workers
pub struct GrpcCompletionSignaling {
    port: u16,
    rx: tokio::sync::mpsc::Receiver<(usize, bool)>,
}

impl CompletionSignaling for GrpcCompletionSignaling {
    type Token = GrpcCompletionToken;

    fn setup(_num_workers: usize) -> Self {
        let (tx, rx) = tokio::sync::mpsc::channel(100);
        let (port_tx, port_rx) = std::sync::mpsc::channel();

        tokio::spawn(async move {
            // Bind to a random available port
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
                .await
                .expect("Failed to bind completion listener");

            let addr = listener.local_addr().expect("No local address");
            port_tx.send(addr.port()).expect("Failed to send port");

            let service = CompletionServiceImpl { tx };

            // Use the listener directly instead of binding again
            let incoming = tokio_stream::wrappers::TcpListenerStream::new(listener);

            if let Err(e) = Server::builder()
                .add_service(CompletionServiceServer::new(service))
                .serve_with_incoming(incoming)
                .await
            {
                eprintln!("Completion service error: {}", e);
            }
        });

        let port = port_rx.recv().expect("Failed to receive port");

        Self { port, rx }
    }

    fn get_token(&self, worker_id: usize) -> Self::Token {
        GrpcCompletionToken {
            server_addr: format!("127.0.0.1:{}", self.port),
            worker_id,
        }
    }

    async fn wait_next(&mut self) -> Option<Result<usize, usize>> {
        self.rx.recv().await.map(|(worker_id, success)| {
            if success {
                Ok(worker_id)
            } else {
                Err(worker_id)
            }
        })
    }

    async fn reset_worker(&mut self, worker_id: usize) -> Self::Token {
        // Drain any pending messages for this worker
        while let Ok((id, _)) = self.rx.try_recv() {
            if id != worker_id {
                // Put it back if it's not for this worker (simplified - in production use a queue)
                break;
            }
        }

        self.get_token(worker_id)
    }
}
