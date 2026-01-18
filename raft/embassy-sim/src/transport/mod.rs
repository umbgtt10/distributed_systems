// Copyright 2025 Umberto Gotti <umberto.gotti@umbertogotti.dev>
// Licensed under the Apache License, Version 2.0
// http://www.apache.org/licenses/LICENSE-2.0

pub mod async_transport;
pub mod embassy_transport;

#[cfg(feature = "channel-transport")]
pub mod channel;

#[cfg(feature = "channel-transport")]
pub use channel::setup;

#[cfg(feature = "udp-transport")]
pub mod udp;

#[cfg(feature = "udp-transport")]
pub use udp::setup;
