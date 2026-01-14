// Copyright 2025 Umberto Gotti <umberto.gotti@umbertogotti.dev>
// Licensed under the Apache License, Version 2.0
// http://www.apache.org/licenses/LICENSE-2.0

use raft_core::state_machine::StateMachine;

pub struct InMemoryStateMachine {
    pub state: Vec<String>,
}

impl InMemoryStateMachine {
    pub fn new() -> Self {
        Self { state: Vec::new() }
    }
}

impl Default for InMemoryStateMachine {
    fn default() -> Self {
        Self::new()
    }
}

impl StateMachine for InMemoryStateMachine {
    type Payload = String;

    fn apply(&mut self, entry: &String) {
        self.state.push(entry.clone());
    }
}
