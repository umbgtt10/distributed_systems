use async_trait::async_trait;
use map_reduce_core::work_channel::WorkDistributor;
use map_reduce_core::worker_io::WorkReceiver;
use serde::{Deserialize, Serialize};
use std::marker::PhantomData;
use std::sync::Arc;
use tokio::sync::Mutex;
use tonic::transport::{Channel, Server};
use tonic::{Request, Response, Status};

use crate::rpc::proto;
use proto::work_service_client::WorkServiceClient;
use proto::work_service_server::{WorkService as WorkServiceTrait, WorkServiceServer};
use proto::{WorkAck, WorkMessage};

/// gRPC Work Channel Distributor
/// Sends work to workers via gRPC (hybrid JSON approach)
#[derive(Clone)]
pub struct GrpcWorkChannel<A, C> {
    worker_addr: String,
    _phantom: PhantomData<(A, C)>,
}

impl<A, C> GrpcWorkChannel<A, C> {
    pub fn new(worker_addr: String) -> Self {
        Self {
            worker_addr,
            _phantom: PhantomData,
        }
    }
}

impl<A, C> WorkDistributor<A, C> for GrpcWorkChannel<A, C>
where
    A: Clone + Send + Serialize + 'static,
    C: Clone + Send + Serialize + 'static,
{
    fn send_work(&self, assignment: A, completion: C) {
        let addr = self.worker_addr.clone();
        let assignment_json = serde_json::to_string(&assignment).unwrap();
        let completion_json = serde_json::to_string(&completion).unwrap();

        tokio::spawn(async move {
            let endpoint = format!("http://{}", addr);

            // Retry logic for connecting AND sending to worker
            for attempt in 0..30 {
                // Clone data for this attempt
                let req_assignment = assignment_json.clone();
                let req_completion = completion_json.clone();

                let result = async {
                    let channel = Channel::from_shared(endpoint.clone())
                        .unwrap()
                        .connect()
                        .await
                        .map_err(|e| format!("Connection error: {}", e))?;

                    let mut client = WorkServiceClient::new(channel);
                    let request = tonic::Request::new(WorkMessage {
                        assignment_json: req_assignment,
                        completion_json: req_completion,
                    });

                    client
                        .receive_work(request)
                        .await
                        .map_err(|e| format!("RPC error: {}", e))
                }
                .await;

                match result {
                    Ok(_) => break, // Success
                    Err(e) => {
                        if attempt >= 29 {
                            eprintln!(
                                "Failed to send work to {} after {} attempts: {}",
                                addr,
                                attempt + 1,
                                e
                            );
                        }
                        tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;
                    }
                }
            }
        });
    }
}

/// gRPC Work Receiver
/// Receives work assignments from coordinator
#[derive(Serialize, Deserialize)]
pub struct GrpcWorkReceiver<A, C> {
    port: u16,
    #[serde(skip, default = "default_rx")]
    rx: Arc<Mutex<Option<tokio::sync::mpsc::Receiver<(A, C)>>>>,
}

fn default_rx<A, C>() -> Arc<Mutex<Option<tokio::sync::mpsc::Receiver<(A, C)>>>> {
    Arc::new(Mutex::new(None))
}

impl<A, C> GrpcWorkReceiver<A, C> {
    pub fn new(port: u16) -> Self {
        Self {
            port,
            rx: Arc::new(Mutex::new(None)),
        }
    }
}

/// gRPC Work Service implementation
struct WorkServiceImpl<A, C> {
    tx: tokio::sync::mpsc::Sender<(A, C)>,
    _phantom: PhantomData<(A, C)>,
}

impl<A, C> Clone for WorkServiceImpl<A, C> {
    fn clone(&self) -> Self {
        Self {
            tx: self.tx.clone(),
            _phantom: PhantomData,
        }
    }
}

#[tonic::async_trait]
impl<A, C> WorkServiceTrait for WorkServiceImpl<A, C>
where
    A: Send + Sync + for<'de> Deserialize<'de> + 'static,
    C: Send + Sync + for<'de> Deserialize<'de> + 'static,
{
    async fn receive_work(
        &self,
        request: Request<WorkMessage>,
    ) -> Result<Response<WorkAck>, Status> {
        let msg = request.into_inner();

        let assignment: A = serde_json::from_str(&msg.assignment_json)
            .map_err(|e| Status::invalid_argument(format!("Invalid assignment JSON: {}", e)))?;

        let completion: C = serde_json::from_str(&msg.completion_json)
            .map_err(|e| Status::invalid_argument(format!("Invalid completion JSON: {}", e)))?;

        self.tx
            .send((assignment, completion))
            .await
            .map_err(|_| Status::internal("Failed to queue work"))?;

        Ok(Response::new(WorkAck { received: true }))
    }
}

#[async_trait]
impl<A, C> WorkReceiver<A, C> for GrpcWorkReceiver<A, C>
where
    A: Send + Sync + Serialize + for<'de> Deserialize<'de> + 'static,
    C: Send + Sync + Serialize + for<'de> Deserialize<'de> + 'static,
{
    async fn recv(&mut self) -> Option<(A, C)> {
        let mut rx_guard = self.rx.lock().await;

        if rx_guard.is_none() {
            let (tx, rx) = tokio::sync::mpsc::channel(10);
            *rx_guard = Some(rx);
            drop(rx_guard);

            let port = self.port;
            let service = WorkServiceImpl::<A, C> {
                tx,
                _phantom: PhantomData,
            };

            tokio::spawn(async move {
                let addr: std::net::SocketAddr = format!("127.0.0.1:{}", port).parse().unwrap();

                // Retry loop for binding the server port
                // This is crucial for respawned workers where the port might still be in TIME_WAIT
                for attempt in 0..10 {
                    // Use incoming stream to detect bind errors before starting server
                    match tokio::net::TcpListener::bind(addr).await {
                        Ok(listener) => {
                            let incoming = tokio_stream::wrappers::TcpListenerStream::new(listener);
                            if let Err(e) = Server::builder()
                                .add_service(WorkServiceServer::new(service.clone()))
                                .serve_with_incoming(incoming)
                                .await
                            {
                                eprintln!("Work service error: {}", e);
                            }
                            return;
                        }
                        Err(e) => {
                            if attempt == 9 {
                                eprintln!(
                                    "Failed to bind work service to {} after 10 attempts: {}",
                                    addr, e
                                );
                            } else {
                                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                            }
                        }
                    }
                }
            });

            rx_guard = self.rx.lock().await;
        }

        if let Some(rx) = rx_guard.as_mut() {
            rx.recv().await
        } else {
            None
        }
    }
}
