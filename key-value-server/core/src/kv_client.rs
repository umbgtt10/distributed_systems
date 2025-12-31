// Copyright 2025 Umberto Gotti <umberto.gotti@umbertogotti.dev>
// Licensed under the Apache License, Version 2.0
// http://www.apache.org/licenses/LICENSE-2.0

use crate::rpc::proto::{
    kv_service_client::KvServiceClient, GetRequest, GetResponse, PutRequest, PutResponse,
};
use async_trait::async_trait;
use tonic::{transport::Channel, Request, Response, Status};

#[async_trait]
pub trait KvClient: Send + Sync {
    async fn get(&mut self, request: Request<GetRequest>) -> Result<Response<GetResponse>, Status>;
    async fn put(&mut self, request: Request<PutRequest>) -> Result<Response<PutResponse>, Status>;
}

#[async_trait]
impl KvClient for KvServiceClient<Channel> {
    async fn get(&mut self, request: Request<GetRequest>) -> Result<Response<GetResponse>, Status> {
        self.get(request).await
    }

    async fn put(&mut self, request: Request<PutRequest>) -> Result<Response<PutResponse>, Status> {
        self.put(request).await
    }
}
