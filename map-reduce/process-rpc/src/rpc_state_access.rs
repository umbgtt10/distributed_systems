use crate::rpc::{StateRequest, StateResponse};
use map_reduce_core::state_access::StateAccess;
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::sync::{Arc, Mutex};

fn default_stream() -> Arc<Mutex<Option<TcpStream>>> {
    Arc::new(Mutex::new(None))
}

#[derive(Clone, Serialize, Deserialize)]
pub struct RpcStateAccess {
    server_addr: SocketAddr,
    #[serde(skip, default = "default_stream")]
    stream: Arc<Mutex<Option<TcpStream>>>,
}

impl RpcStateAccess {
    pub fn new(server_addr: SocketAddr) -> Self {
        Self {
            server_addr,
            stream: Arc::new(Mutex::new(None)),
        }
    }

    fn send_request(&self, request: StateRequest) -> StateResponse {
        let mut stream_guard = self.stream.lock().unwrap();

        if stream_guard.is_none() {
            match TcpStream::connect(self.server_addr) {
                Ok(s) => *stream_guard = Some(s),
                Err(e) => return StateResponse::Error(format!("Failed to connect: {}", e)),
            }
        }

        let stream = stream_guard.as_mut().unwrap();

        // Simple length-prefixed JSON protocol
        let body = serde_json::to_vec(&request).unwrap();
        let len = body.len() as u32;

        if let Err(e) = stream.write_all(&len.to_be_bytes()) {
            *stream_guard = None; // Invalidate connection
            return StateResponse::Error(format!("Write error: {}", e));
        }
        if let Err(e) = stream.write_all(&body) {
            *stream_guard = None;
            return StateResponse::Error(format!("Write error: {}", e));
        }

        // Read response
        let mut len_bytes = [0u8; 4];
        if let Err(e) = stream.read_exact(&mut len_bytes) {
            *stream_guard = None;
            return StateResponse::Error(format!("Read len error: {}", e));
        }
        let len = u32::from_be_bytes(len_bytes) as usize;
        let mut buffer = vec![0u8; len];
        if let Err(e) = stream.read_exact(&mut buffer) {
            *stream_guard = None;
            return StateResponse::Error(format!("Read body error: {}", e));
        }

        serde_json::from_slice(&buffer)
            .unwrap_or_else(|e| StateResponse::Error(format!("Deserialize error: {}", e)))
    }
}

impl StateAccess for RpcStateAccess {
    fn initialize(&self, keys: Vec<String>) {
        self.send_request(StateRequest::Initialize(keys));
    }

    fn update(&self, key: String, value: i32) {
        if let StateResponse::Error(e) = self.send_request(StateRequest::Update(key, value)) {
            eprintln!("State update error: {}", e);
        }
    }

    fn replace(&self, key: String, value: i32) {
        if let StateResponse::Error(e) = self.send_request(StateRequest::Replace(key, value)) {
            eprintln!("State replace error: {}", e);
        }
    }

    fn get(&self, key: &str) -> Vec<i32> {
        match self.send_request(StateRequest::Get(key.to_string())) {
            StateResponse::Value(v) => v,
            _ => Vec::new(),
        }
    }
}
