// Copyright 2025 Umberto Gotti <umberto.gotti@umbertogotti.dev>
// Licensed under the Apache License, Version 2.0
// http://www.apache.org/licenses/LICENSE-2.0

use crate::types::NodeId;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CollectionError {
    Full,
}

pub trait NodeCollection {
    type Iter<'a>: Iterator<Item = &'a NodeId>
    where
        Self: 'a;

    fn new() -> Self;
    fn clear(&mut self);
    fn push(&mut self, id: NodeId) -> Result<(), CollectionError>;
    fn len(&self) -> usize;
    fn iter(&self) -> Self::Iter<'_>;
    fn is_empty(&self) -> bool;
}
