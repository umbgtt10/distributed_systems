use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub enum StateRequest {
    Initialize(Vec<String>),
    Update(String, i32),
    Replace(String, i32),
    Get(String),
}

#[derive(Serialize, Deserialize, Debug)]
pub enum StateResponse {
    Ok,
    Value(Vec<i32>),
    Error(String),
}
