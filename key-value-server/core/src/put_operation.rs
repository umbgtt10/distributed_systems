// Copyright 2025 Umberto Gotti <umberto.gotti@umbertogotti.dev>
// Licensed under the Apache License, Version 2.0
// http://www.apache.org/licenses/LICENSE-2.0

use crate::{
    rpc::proto::{
        get_response, kv_service_client::KvServiceClient, put_response, ErrorType, GetRequest,
        PutRequest,
    },
    ClientConfig, Random, Timer,
};
use std::time::Duration;
use tokio_util::sync::CancellationToken;

#[derive(Debug)]
enum PutAction {
    RetryWithNewVersion,
    DoGetForVersion,
    ReturnSuccess,
    ReturnError,
    NetworkRetry,
}

pub struct PutOperation<'a, T: Timer, R: Random> {
    config: &'a ClientConfig,
    key: String,
    value: String,
    version: u64,
    network_retry_count: u32,
    max_retries: u32,
    cancellation_token: &'a CancellationToken,
    op_num: u64,
    timer: &'a T,
    random: &'a R,
}

impl<'a, T: Timer, R: Random> PutOperation<'a, T, R> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        config: &'a ClientConfig,
        key: &str,
        value: String,
        op_num: u64,
        max_retries: u32,
        cancellation_token: &'a CancellationToken,
        timer: &'a T,
        random: &'a R,
    ) -> Self {
        Self {
            config,
            key: key.to_string(),
            value,
            version: 0,
            network_retry_count: 0,
            max_retries,
            cancellation_token,
            op_num,
            timer,
            random,
        }
    }

    pub async fn execute(
        mut self,
        client: &mut KvServiceClient<tonic::transport::Channel>,
    ) -> Result<(), ()> {
        loop {
            if self.cancellation_token.is_cancelled() {
                println!(
                    "[{}][{}] PUT '{}' -> CANCELLED",
                    self.config.name, self.op_num, self.key
                );
                return Err(());
            }

            // Simulate client-side packet loss BEFORE sending request
            if self.random.f32() < (self.config.client_packet_loss_rate / 100.0) {
                self.network_retry_count += 1;
                println!(
                    "[{}][{}] PUT '{}' -> CLIENT PACKET LOSS (request not sent)",
                    self.config.name, self.op_num, self.key
                );

                if self.network_retry_count >= self.max_retries {
                    println!(
                        "[{}][{}] PUT '{}' -> CLIENT PACKET LOSS after {} attempts, giving up",
                        self.config.name, self.op_num, self.key, self.network_retry_count
                    );
                    self.timer
                        .sleep(Duration::from_millis(self.config.error_sleep_ms))
                        .await;
                    return Err(());
                }

                self.timer
                    .sleep(Duration::from_millis(self.config.error_sleep_ms))
                    .await;
                continue;
            }

            let request = tonic::Request::new(PutRequest {
                key: self.key.clone(),
                value: self.value.clone(),
                version: self.version,
            });

            let response = client.put(request).await;
            let action = self.handle_put_response(response);

            match action {
                PutAction::RetryWithNewVersion => continue,
                PutAction::DoGetForVersion => {
                    // Do a GET to fetch the current version
                    let get_request = tonic::Request::new(GetRequest {
                        key: self.key.clone(),
                    });

                    match client.get(get_request).await {
                        Ok(get_response) => {
                            if let Some(get_response::Result::Success(success)) =
                                get_response.into_inner().result
                            {
                                self.version = success.version;
                                println!(
                                    "[{}][{}] PUT '{}' -> Fetched version={}, switching to update mode",
                                    self.config.name, self.op_num, self.key, self.version
                                );
                                continue;
                            } else {
                                // If GET failed to get version, fall back to version 1
                                self.version = 1;
                                continue;
                            }
                        }
                        Err(_) => {
                            // If GET fails, fall back to version 1
                            self.version = 1;
                            continue;
                        }
                    }
                }
                PutAction::ReturnSuccess => {
                    self.timer
                        .sleep(Duration::from_millis(self.config.success_sleep_ms))
                        .await;
                    return Ok(());
                }
                PutAction::ReturnError => {
                    self.timer
                        .sleep(Duration::from_millis(self.config.error_sleep_ms))
                        .await;
                    return Err(());
                }
                PutAction::NetworkRetry => {
                    self.network_retry_count += 1;
                    if self.network_retry_count >= self.max_retries {
                        println!(
                            "[{}][{}] PUT '{}' -> NETWORK ERROR after {} retries",
                            self.config.name, self.op_num, self.key, self.network_retry_count
                        );
                        self.timer
                            .sleep(Duration::from_millis(self.config.error_sleep_ms))
                            .await;
                        return Err(());
                    }

                    if self.cancellation_token.is_cancelled() {
                        println!(
                            "[{}][{}] PUT '{}' -> CANCELLED during network retry",
                            self.config.name, self.op_num, self.key
                        );
                        return Err(());
                    }

                    println!(
                        "[{}][{}] PUT '{}' -> NETWORK ERROR, retrying... (attempt {}/{})",
                        self.config.name,
                        self.op_num,
                        self.key,
                        self.network_retry_count,
                        self.max_retries
                    );
                    self.timer
                        .sleep(Duration::from_millis(self.config.error_sleep_ms))
                        .await;
                    continue;
                }
            }
        }
    }

    fn handle_put_response(
        &mut self,
        response: Result<tonic::Response<crate::rpc::proto::PutResponse>, tonic::Status>,
    ) -> PutAction {
        match response {
            Ok(resp) => {
                // Save network retry count before resetting for recovery detection
                let had_network_errors = self.network_retry_count > 0;
                let retry_count_for_log = self.network_retry_count;

                // Network is working - reset retry counter
                self.network_retry_count = 0;

                let result = resp.into_inner().result;
                match result {
                    Some(put_response::Result::Success(success)) => {
                        let operation = if self.version == 0 {
                            "CREATE"
                        } else {
                            "UPDATE"
                        };
                        if had_network_errors {
                            let retry_word = if retry_count_for_log == 1 {
                                "retry"
                            } else {
                                "retries"
                            };
                            println!(
                                "[{}][{}] PUT '{}' -> {} RECOVERED after {} network {} (value='{}', new_version={})",
                                self.config.name, self.op_num, self.key, operation, retry_count_for_log, retry_word, self.value, success.new_version
                            );
                        } else {
                            println!(
                                "[{}][{}] PUT '{}' -> {} (value='{}', new_version={})",
                                self.config.name,
                                self.op_num,
                                self.key,
                                operation,
                                self.value,
                                success.new_version
                            );
                        }
                        PutAction::ReturnSuccess
                    }
                    Some(put_response::Result::Error(error)) => {
                        let error_type =
                            ErrorType::try_from(error.error_type).unwrap_or(ErrorType::KeyNotFound);

                        match error_type {
                            ErrorType::VersionMismatch => {
                                // Use the structured actual_version field from the error
                                if let Some(actual_version) = error.actual_version {
                                    if had_network_errors {
                                        let retry_word = if retry_count_for_log == 1 {
                                            "retry"
                                        } else {
                                            "retries"
                                        };
                                        println!(
                                            "[{}][{}] PUT '{}' -> RECOVERED after {} network {} (write succeeded, detected via version_mismatch, server version={})",
                                            self.config.name, self.op_num, self.key, retry_count_for_log, retry_word, actual_version
                                        );
                                        // Recovery detected - the previous write succeeded, we're done!
                                        PutAction::ReturnSuccess
                                    } else {
                                        self.version = actual_version;
                                        println!("[{}][{}] PUT '{}' -> RETRY (version_mismatch, using version={})", self.config.name, self.op_num, self.key, self.version);
                                        PutAction::RetryWithNewVersion
                                    }
                                } else {
                                    println!(
                                        "[{}][{}] PUT '{}' -> ERROR (VersionMismatch without actual_version)",
                                        self.config.name, self.op_num, self.key
                                    );
                                    PutAction::ReturnError
                                }
                            }
                            ErrorType::KeyAlreadyExists => {
                                // Key exists but we tried to create - fetch actual version with GET
                                println!(
                                    "[{}][{}] PUT '{}' -> KEY_EXISTS (fetching current version)",
                                    self.config.name, self.op_num, self.key
                                );
                                PutAction::DoGetForVersion
                            }
                            ErrorType::KeyNotFound => {
                                // Key doesn't exist, try to create
                                println!(
                                    "[{}][{}] PUT '{}' -> KEY_NOT_FOUND (switching to create mode)",
                                    self.config.name, self.op_num, self.key
                                );
                                self.version = 0;
                                PutAction::RetryWithNewVersion
                            }
                        }
                    }
                    None => {
                        println!(
                            "[{}][{}] PUT '{}' -> ERROR (No result)",
                            self.config.name, self.op_num, self.key
                        );
                        PutAction::ReturnError
                    }
                }
            }
            Err(status) => {
                println!(
                    "[{}][{}] PUT '{}' -> NETWORK ERROR ({})",
                    self.config.name,
                    self.op_num,
                    self.key,
                    status.message()
                );
                PutAction::NetworkRetry
            }
        }
    }
}
