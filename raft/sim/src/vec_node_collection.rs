// Copyright 2025 Umberto Gotti <umberto.gotti@umbertogotti.dev>
// Licensed under the Apache License, Version 2.0
// http://www.apache.org/licenses/LICENSE-2.0

use raft_core::{
    node_collection::{CollectionError, NodeCollection},
    types::NodeId,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VecNodeCollection {
    nodes: Vec<NodeId>,
}

impl VecNodeCollection {
    pub fn new() -> Self {
        Self { nodes: Vec::new() }
    }
}

impl Default for VecNodeCollection {
    fn default() -> Self {
        Self::new()
    }
}

impl NodeCollection for VecNodeCollection {
    type Iter<'a> = std::slice::Iter<'a, NodeId>;

    fn new() -> Self {
        Self { nodes: Vec::new() }
    }

    fn clear(&mut self) {
        self.nodes.clear();
    }

    fn push(&mut self, id: NodeId) -> Result<(), CollectionError> {
        self.nodes.push(id);
        Ok(())
    }

    fn len(&self) -> usize {
        self.nodes.len()
    }

    fn iter(&self) -> Self::Iter<'_> {
        self.nodes.iter()
    }

    fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }
}
