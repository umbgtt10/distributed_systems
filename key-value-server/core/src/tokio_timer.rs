// Copyright 2025 Umberto Gotti <umberto.gotti@umbertogotti.dev>
// Licensed under the Apache License, Version 2.0
// http://www.apache.org/licenses/LICENSE-2.0

use crate::Timer;
use std::time::Duration;
use tokio::time::sleep;

pub struct TokioTimer;

#[async_trait::async_trait]
impl Timer for TokioTimer {
    async fn sleep(&self, duration: Duration) {
        sleep(duration).await;
    }
}
