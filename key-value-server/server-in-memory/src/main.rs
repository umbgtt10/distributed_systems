mod in_memory_storage;

use crate::in_memory_storage::InMemoryStorage;
use key_value_server_core::{rpc::proto::kv_service_server::KvServiceServer, KeyValueServer};
use tonic::transport::Server;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "127.0.0.1:50051".parse()?;
    let storage = InMemoryStorage::new();
    let service = KeyValueServer::new(storage);

    println!("KV Server listening on {}", addr);
    println!("Press Ctrl+C to stop the server");

    Server::builder()
        .add_service(KvServiceServer::new(service))
        .serve_with_shutdown(addr, async {
            tokio::signal::ctrl_c()
                .await
                .expect("Failed to listen for Ctrl+C");
            println!("\nShutting down server...");
        })
        .await?;

    println!("Server stopped");
    Ok(())
}
