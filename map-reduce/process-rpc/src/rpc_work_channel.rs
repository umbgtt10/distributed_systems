use async_trait::async_trait;
use futures::{SinkExt, StreamExt};
use map_reduce_core::work_channel::WorkDistributor;
use map_reduce_core::worker_io::WorkReceiver;
use serde::{Deserialize, Serialize};
use std::marker::PhantomData;
use std::net::SocketAddr;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;
use tokio_util::codec::{Framed, LengthDelimitedCodec};
use bytes::Bytes;

#[derive(Serialize, Deserialize)]
struct WorkRequest {
    assignment: String,
    completion: String,
}

#[derive(Clone)]
pub struct RpcWorkChannel<A, C> {
    worker_addr: SocketAddr,
    _phantom: PhantomData<(A, C)>,
}

impl<A, C> RpcWorkChannel<A, C> {
    pub fn new(worker_addr: SocketAddr) -> Self {
        Self {
            worker_addr,
            _phantom: PhantomData,
        }
    }
}

impl<A, C> WorkDistributor<A, C> for RpcWorkChannel<A, C>
where
    A: Clone + Send + Serialize + 'static,
    C: Clone + Send + Serialize + 'static,
{
    fn send_work(&self, assignment: A, completion: C) {
        let addr = self.worker_addr;
        let assignment_json = serde_json::to_string(&assignment).unwrap();
        let completion_json = serde_json::to_string(&completion).unwrap();

        let request = WorkRequest {
            assignment: assignment_json,
            completion: completion_json,
        };
        let request_bytes = serde_json::to_vec(&request).unwrap();
        let request_bytes = Bytes::from(request_bytes);

        tokio::spawn(async move {
            let mut attempts = 0;
            loop {
                match TcpStream::connect(addr).await {
                    Ok(stream) => {
                        let mut framed = Framed::new(stream, LengthDelimitedCodec::new());
                        if let Err(e) = framed.send(request_bytes.clone()).await {
                             eprintln!("Failed to send work to {}: {}", addr, e);
                        }
                        break;
                    }
                    Err(e) => {
                        attempts += 1;
                        if attempts >= 20 {
                            eprintln!("Failed to connect to worker at {} after {} attempts: {}", addr, attempts, e);
                            break;
                        }
                        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                    }
                }
            }
        });
    }
}

#[derive(Serialize, Deserialize)]
#[serde(bound = "")]
pub struct RpcWorkReceiver<A, C> {
    port: u16,
    #[serde(skip)]
    rx: Option<mpsc::Receiver<(A, C)>>,
}

impl<A, C> RpcWorkReceiver<A, C> {
    pub fn new(port: u16) -> Self {
        Self { port, rx: None }
    }
}

#[async_trait]
impl<A, C> WorkReceiver<A, C> for RpcWorkReceiver<A, C>
where
    A: Send + Serialize + for<'de> Deserialize<'de> + 'static,
    C: Send + Serialize + for<'de> Deserialize<'de> + 'static,
{
    async fn recv(&mut self) -> Option<(A, C)> {
        if self.rx.is_none() {
            let (tx, rx) = mpsc::channel(1);
            self.rx = Some(rx);
            let port = self.port;

            tokio::spawn(async move {
                println!("Worker listening on port {}", port);
                let listener = match TcpListener::bind(format!("0.0.0.0:{}", port)).await {
                    Ok(l) => l,
                    Err(e) => {
                        eprintln!("Worker failed to listen on port {}: {}", port, e);
                        return;
                    }
                };

                loop {
                    if let Ok((stream, _)) = listener.accept().await {
                        let tx = tx.clone();
                        tokio::spawn(async move {
                            let mut framed = Framed::new(stream, LengthDelimitedCodec::new());
                            if let Some(Ok(bytes)) = framed.next().await {
                                if let Ok(req) = serde_json::from_slice::<WorkRequest>(&bytes) {
                                    if let (Ok(a), Ok(c)) = (
                                        serde_json::from_str::<A>(&req.assignment),
                                        serde_json::from_str::<C>(&req.completion)
                                    ) {
                                        println!("Worker received work assignment");
                                        let _ = tx.send((a, c)).await;
                                    } else {
                                        eprintln!("Worker failed to deserialize assignment/completion");
                                    }
                                } else {
                                    eprintln!("Worker failed to deserialize WorkRequest");
                                }
                            }
                        });
                    }
                }
            });
        }
        self.rx.as_mut().unwrap().recv().await
    }
}

