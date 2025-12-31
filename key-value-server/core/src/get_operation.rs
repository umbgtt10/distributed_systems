// Copyright 2025 Umberto Gotti <umberto.gotti@umbertogotti.dev>
// Licensed under the Apache License, Version 2.0
// http://www.apache.org/licenses/LICENSE-2.0

use crate::{
    grpc_client::{Random, Timer},
    rpc::proto::{get_response, kv_service_client::KvServiceClient, ErrorType, GetRequest},
    ClientConfig,
};
use std::time::Duration;

pub struct GetOperation<'a, T: Timer, R: Random> {
    config: &'a ClientConfig,
    key: String,
    op_num: u64,
    timer: &'a T,
    random: &'a R,
}

impl<'a, T: Timer, R: Random> GetOperation<'a, T, R> {
    pub fn new(
        config: &'a ClientConfig,
        key: &str,
        op_num: u64,
        timer: &'a T,
        random: &'a R,
    ) -> Self {
        Self {
            config,
            key: key.to_string(),
            op_num,
            timer,
            random,
        }
    }

    pub async fn execute(self, client: &mut KvServiceClient<tonic::transport::Channel>) {
        // Simulate client-side packet loss BEFORE sending request
        if self.random.f32() < (self.config.client_packet_loss_rate / 100.0) {
            println!(
                "[{}][{}] GET '{}' -> CLIENT PACKET LOSS (request not sent)",
                self.config.name, self.op_num, self.key
            );
            self.timer
                .sleep(Duration::from_millis(self.config.error_sleep_ms))
                .await;
            return;
        }

        let request = tonic::Request::new(GetRequest {
            key: self.key.clone(),
        });

        let response = client.get(request).await;
        match response {
            Ok(resp) => {
                let result = resp.into_inner().result;
                match result {
                    Some(get_response::Result::Success(success)) => {
                        println!(
                            "[{}][{}] GET '{}' -> OK (value='{}', version={})",
                            self.config.name, self.op_num, self.key, success.value, success.version
                        );
                        self.timer
                            .sleep(Duration::from_millis(self.config.success_sleep_ms))
                            .await;
                    }
                    Some(get_response::Result::Error(error)) => {
                        let error_type =
                            ErrorType::try_from(error.error_type).unwrap_or(ErrorType::KeyNotFound);
                        println!(
                            "[{}][{}] GET '{}' -> ERROR ({:?}: {})",
                            self.config.name, self.op_num, self.key, error_type, error.message
                        );
                        self.timer
                            .sleep(Duration::from_millis(self.config.error_sleep_ms))
                            .await;
                    }
                    None => {
                        println!(
                            "[{}][{}] GET '{}' -> ERROR (No result)",
                            self.config.name, self.op_num, self.key
                        );
                        self.timer
                            .sleep(Duration::from_millis(self.config.error_sleep_ms))
                            .await;
                    }
                }
            }
            Err(status) => {
                println!(
                    "[{}][{}] GET '{}' -> NETWORK ERROR ({})",
                    self.config.name,
                    self.op_num,
                    self.key,
                    status.message()
                );
                self.timer
                    .sleep(Duration::from_millis(self.config.error_sleep_ms))
                    .await;
            }
        }
    }
}
