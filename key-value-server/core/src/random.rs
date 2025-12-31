// Copyright 2025 Umberto Gotti <umberto.gotti@umbertogotti.dev>
// Licensed under the Apache License, Version 2.0
// http://www.apache.org/licenses/LICENSE-2.0

pub trait Random: Send + Sync {
    fn usize(&self, range: std::ops::Range<usize>) -> usize;
    fn bool(&self) -> bool;
    fn u32(&self, range: std::ops::Range<u32>) -> u32;
    fn f32(&self) -> f32;
}
