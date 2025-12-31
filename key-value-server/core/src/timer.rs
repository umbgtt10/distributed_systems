// Copyright 2025 Umberto Gotti <umberto.gotti@umbertogotti.dev>
// Licensed under the Apache License, Version 2.0
// http://www.apache.org/licenses/LICENSE-2.0

use std::time::Duration;

#[async_trait::async_trait]
pub trait Timer: Send + Sync {
    async fn sleep(&self, duration: Duration);
}
