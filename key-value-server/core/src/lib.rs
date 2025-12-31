// Copyright 2025 Umberto Gotti <umberto.gotti@umbertogotti.dev>
// Licensed under the Apache License, Version 2.0
// http://www.apache.org/licenses/LICENSE-2.0

mod storage;
pub use storage::Storage;

mod storage_error;
pub use storage_error::StorageError;

mod key_value_server;
pub use key_value_server::KeyValueServer;

mod packet_loss_wrapper;
pub use packet_loss_wrapper::PacketLossWrapper;

mod get_operation;
pub use get_operation::GetOperation;

mod put_operation;
pub use put_operation::PutOperation;

mod kv_client;
pub use kv_client::KvClient;

pub mod random;
pub use random::Random;

pub mod fastrand_random;
pub use fastrand_random::FastrandRandom;

mod grpc_client;
pub use grpc_client::GrpcClient;

mod client_config;
pub use client_config::{ClientConfig, TestConfig};

mod server_runner;
pub use server_runner::ServerRunner;

pub mod timer;
pub use timer::Timer;

pub mod tokio_timer;
pub use tokio_timer::TokioTimer;

pub mod rpc {
    pub mod proto {
        include!("../.generated/kvservice.rs");
    }
}
