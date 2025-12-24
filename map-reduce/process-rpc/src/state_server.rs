use crate::rpc::{StateRequest, StateResponse};
use map_reduce_core::state_access::StateAccess;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

pub struct StateServer<S> {
    state: S,
    listener: TcpListener,
}

impl<S: StateAccess + Send + Sync + 'static> StateServer<S> {
    pub async fn new(state: S, port: u16) -> Self {
        let listener = TcpListener::bind(format!("0.0.0.0:{}", port))
            .await
            .unwrap();
        Self { state, listener }
    }

    pub fn local_addr(&self) -> std::net::SocketAddr {
        self.listener.local_addr().unwrap()
    }

    pub async fn run(self) {
        let state = Arc::new(self.state);
        loop {
            let (mut socket, _) = match self.listener.accept().await {
                Ok(x) => x,
                Err(_) => continue,
            };
            let state = state.clone();

            tokio::spawn(async move {
                loop {
                    let mut len_bytes = [0u8; 4];
                    if socket.read_exact(&mut len_bytes).await.is_err() {
                        return;
                    }
                    let len = u32::from_be_bytes(len_bytes) as usize;
                    let mut buffer = vec![0u8; len];
                    if socket.read_exact(&mut buffer).await.is_err() {
                        return;
                    }

                    let request: StateRequest = match serde_json::from_slice(&buffer) {
                        Ok(req) => req,
                        Err(_) => return,
                    };

                    let response = match request {
                        StateRequest::Initialize(keys) => {
                            state.initialize(keys);
                            StateResponse::Ok
                        }
                        StateRequest::Update(k, v) => {
                            state.update(k, v);
                            StateResponse::Ok
                        }
                        StateRequest::Replace(k, v) => {
                            state.replace(k, v);
                            StateResponse::Ok
                        }
                        StateRequest::Get(k) => {
                            let val = state.get(&k);
                            StateResponse::Value(val)
                        }
                    };

                    let resp_bytes = serde_json::to_vec(&response).unwrap();
                    let resp_len = resp_bytes.len() as u32;

                    if socket.write_all(&resp_len.to_be_bytes()).await.is_err() {
                        return;
                    }
                    if socket.write_all(&resp_bytes).await.is_err() {
                        return;
                    }
                }
            });
        }
    }
}
