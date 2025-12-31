// Copyright 2025 Umberto Gotti <umberto.gotti@umbertogotti.dev>
// Licensed under the Apache License, Version 2.0
// http://www.apache.org/licenses/LICENSE-2.0

use crate::Random;

pub struct FastrandRandom;

impl Random for FastrandRandom {
    fn usize(&self, range: std::ops::Range<usize>) -> usize {
        fastrand::usize(range)
    }
    fn bool(&self) -> bool {
        fastrand::bool()
    }
    fn u32(&self, range: std::ops::Range<u32>) -> u32 {
        fastrand::u32(range)
    }
    fn f32(&self) -> f32 {
        fastrand::f32()
    }
}
