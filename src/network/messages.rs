use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
pub enum NetworkMessage {
    StartTimer,
    StopTimer,
    ResetTimer,
    Ping,
    Pong,
    Connect,
}