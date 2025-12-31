// Copyright 2025 Umberto Gotti <umberto.gotti@umbertogotti.dev>
// Licensed under the Apache License, Version 2.0
// http://www.apache.org/licenses/LICENSE-2.0

use crate::rpc::proto::kv_service_client::KvServiceClient;
use crate::{
    ClientConfig, FastrandRandom, GetOperation, KvClient, PutOperation, Random, Timer, TokioTimer,
};
use tokio_util::sync::CancellationToken;
use tonic::transport::Channel;

pub struct GrpcClient<
    T: Timer = TokioTimer,
    R: Random = FastrandRandom,
    C: KvClient = KvServiceClient<Channel>,
> {
    config: ClientConfig,
    max_retries: u32,
    cancellation_token: CancellationToken,
    timer: T,
    random: R,
    client: C,
}

impl<T: Timer, R: Random, C: KvClient> GrpcClient<T, R, C> {
    pub fn new(config: ClientConfig, max_retries: u32, timer: T, random: R, client: C) -> Self {
        Self {
            config,
            max_retries,
            cancellation_token: CancellationToken::new(),
            timer,
            random,
            client,
        }
    }

    pub async fn connect(
        config: ClientConfig,
        server_address: String,
        max_retries: u32,
        timer: T,
        random: R,
    ) -> Result<GrpcClient<T, R, KvServiceClient<Channel>>, Box<dyn std::error::Error>> {
        let client = KvServiceClient::connect(server_address).await?;
        Ok(GrpcClient::new(config, max_retries, timer, random, client))
    }

    pub fn cancellation_token(&self) -> CancellationToken {
        self.cancellation_token.clone()
    }

    pub async fn start(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        println!(
            "[{}] Running stress test with {} keys...\n",
            self.config.name,
            self.config.keys.len()
        );

        let mut operation_count = 0;

        loop {
            // Check for cancellation
            if self.cancellation_token.is_cancelled() {
                println!("\n[{}] Shutting down client...", self.config.name);
                break;
            }

            operation_count += 1;

            self.perform_operation(operation_count).await;
        }

        println!("[{}] Client stopped", self.config.name);
        Ok(())
    }

    pub async fn perform_operation(&mut self, op_num: u64) {
        let key = &self.config.keys[self.random.usize(0..self.config.keys.len())];

        let is_get = self.random.bool();

        if is_get {
            let op = GetOperation::new(&self.config, key, op_num, &self.timer, &self.random);
            op.execute(&mut self.client).await;
        } else {
            let value = format!("value_{}", self.random.u32(0..u32::MAX));

            let op = PutOperation::new(
                &self.config,
                key,
                value,
                op_num,
                self.max_retries,
                &self.cancellation_token,
                &self.timer,
                &self.random,
            );
            let _ = op.execute(&mut self.client).await;
        }
    }
}
