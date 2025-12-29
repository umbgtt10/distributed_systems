use map_reduce_core::state_access::StateAccess;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;
use tonic::transport::Channel;

use crate::rpc::proto;
use proto::state_service_client::StateServiceClient;
use proto::{GetRequest, InitializeRequest, ReplaceRequest, UpdateRequest};

/// gRPC client for StateAccess
/// Uses tokio::task::block_in_place to bridge sync trait with async gRPC
#[derive(Clone, Serialize, Deserialize)]
pub struct GrpcStateAccess {
    server_addr: String,
    #[serde(skip)]
    client: Arc<Mutex<Option<StateServiceClient<Channel>>>>,
}

impl GrpcStateAccess {
    pub fn new(server_addr: String) -> Self {
        Self {
            server_addr,
            client: Arc::new(Mutex::new(None)),
        }
    }

    async fn get_client(&self) -> Result<StateServiceClient<Channel>, tonic::transport::Error> {
        let mut client_guard = self.client.lock().await;

        if client_guard.is_none() {
            let endpoint = format!("http://{}", self.server_addr);
            let channel = Channel::from_shared(endpoint).unwrap().connect().await?;
            *client_guard = Some(StateServiceClient::new(channel));
        }

        Ok(client_guard.as_ref().unwrap().clone())
    }
}

impl StateAccess for GrpcStateAccess {
    fn initialize(&self, keys: Vec<String>) {
        let self_clone = self.clone();
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async move {
                if let Ok(mut client) = self_clone.get_client().await {
                    let request = tonic::Request::new(InitializeRequest { keys });
                    let _ = client.initialize(request).await;
                }
            })
        });
    }

    fn update(&self, key: String, value: i32) {
        let self_clone = self.clone();
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async move {
                if let Ok(mut client) = self_clone.get_client().await {
                    let request = tonic::Request::new(UpdateRequest { key, value });
                    if let Err(e) = client.update(request).await {
                        eprintln!("State update error: {}", e);
                    }
                }
            })
        });
    }

    fn replace(&self, key: String, value: i32) {
        let self_clone = self.clone();
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async move {
                if let Ok(mut client) = self_clone.get_client().await {
                    let request = tonic::Request::new(ReplaceRequest { key, value });
                    if let Err(e) = client.replace(request).await {
                        eprintln!("State replace error: {}", e);
                    }
                }
            })
        });
    }

    fn get(&self, key: &str) -> Vec<i32> {
        let self_clone = self.clone();
        let key = key.to_string();
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async move {
                if let Ok(mut client) = self_clone.get_client().await {
                    let request = tonic::Request::new(GetRequest { key });
                    if let Ok(response) = client.get(request).await {
                        return response.into_inner().values;
                    }
                }
                Vec::new()
            })
        })
    }
}
