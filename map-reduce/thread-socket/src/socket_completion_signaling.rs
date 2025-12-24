use map_reduce_core::completion_signaling::CompletionSignaling;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Completion message type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CompletionMessage {
    Success(usize),
    Failure(usize),
}

/// Socket-based completion signaling
pub struct SocketCompletionSignaling {
    base_port: u16, // Kept for compatibility but unused
    listeners: Arc<Mutex<HashMap<usize, Arc<TcpListener>>>>,
    ports: Arc<HashMap<usize, u16>>,
}

impl SocketCompletionSignaling {
    pub fn new(num_workers: usize) -> Self {
        let mut listeners = HashMap::new();
        let mut ports = HashMap::new();

        for i in 0..num_workers {
            // Use port 0 to let OS assign an available port
            let listener =
                TcpListener::bind("127.0.0.1:0").expect("Failed to bind completion listener");
            let actual_port = listener
                .local_addr()
                .expect("Failed to get local address")
                .port();

            listener
                .set_nonblocking(true)
                .expect("Failed to set nonblocking");

            listeners.insert(i, Arc::new(listener));
            ports.insert(i, actual_port);
        }

        Self {
            base_port: 0, // No longer used
            listeners: Arc::new(Mutex::new(listeners)),
            ports: Arc::new(ports),
        }
    }

    pub fn get_sender(&self, worker_id: usize) -> CompletionSender {
        let port = self
            .ports
            .get(&worker_id)
            .copied()
            .expect("Invalid worker_id");
        CompletionSender { port, worker_id }
    }
}

impl CompletionSignaling for SocketCompletionSignaling {
    type Token = CompletionSender;

    fn setup(num_workers: usize) -> Self {
        Self::new(num_workers)
    }

    fn get_token(&self, worker_id: usize) -> Self::Token {
        self.get_sender(worker_id)
    }

    async fn wait_next(&mut self) -> Option<Result<usize, usize>> {
        loop {
            {
                let listeners_guard = self.listeners.lock().unwrap();
                for (worker_id, listener) in listeners_guard.iter() {
                    let worker_id = *worker_id;
                    match listener.accept() {
                        Ok((mut stream, _)) => {
                            drop(listeners_guard);

                            // Set stream to blocking mode for reading
                            if stream.set_nonblocking(false).is_err() {
                                return None;
                            }

                            let mut len_bytes = [0u8; 4];
                            if stream.read_exact(&mut len_bytes).is_ok() {
                                let len = u32::from_be_bytes(len_bytes) as usize;
                                let mut buffer = vec![0u8; len];
                                if stream.read_exact(&mut buffer).is_ok() {
                                    if let Ok(msg) =
                                        serde_json::from_slice::<CompletionMessage>(&buffer)
                                    {
                                        return Some(match msg {
                                            CompletionMessage::Success(id) => Ok(id),
                                            CompletionMessage::Failure(id) => Err(id),
                                        });
                                    }
                                }
                            }
                            return None;
                        }
                        Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                            // No data available
                        }
                        Err(_) => {
                            // Error occurred
                        }
                    }
                }
            }

            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }

    async fn drain_worker(&mut self, worker_id: usize) {
        if let Some(listener) = self.listeners.lock().unwrap().get(&worker_id) {
            let start = std::time::Instant::now();
            while start.elapsed() < Duration::from_millis(50) {
                match listener.accept() {
                    Ok((mut stream, _)) => {
                        // Set blocking mode for reading
                        let _ = stream.set_nonblocking(false);

                        let mut len_bytes = [0u8; 4];
                        if stream.read_exact(&mut len_bytes).is_ok() {
                            let len = u32::from_be_bytes(len_bytes) as usize;
                            let mut buffer = vec![0u8; len];
                            let _ = stream.read_exact(&mut buffer);
                        }
                    }
                    Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        break;
                    }
                    Err(_) => break,
                }
            }
        }
    }
}

/// Completion sender
#[derive(Clone, Serialize, Deserialize)]
pub struct CompletionSender {
    port: u16,
    worker_id: usize,
}

impl CompletionSender {
    pub fn send(&self, result: Result<usize, ()>) {
        let addr = format!("127.0.0.1:{}", self.port);
        let message = match result {
            Ok(id) => CompletionMessage::Success(id),
            Err(_) => CompletionMessage::Failure(self.worker_id),
        };
        match TcpStream::connect(&addr) {
            Ok(mut stream) => {
                if let Ok(serialized) = serde_json::to_vec(&message) {
                    let len = serialized.len() as u32;
                    let _ = stream.write_all(&len.to_be_bytes());
                    let _ = stream.write_all(&serialized);
                }
            }
            Err(_) => {}
        }
    }
}
