#![no_std]

use serde::{Deserialize, Serialize};

pub const MAX_MESSAGE_SIZE: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeId {
    Turret,
    Vision,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Message {
    Heartbeat { node: NodeId, uptime_ms: u64 },
}

pub fn encode<'a>(message: &Message, buffer: &'a mut [u8]) -> postcard::Result<&'a mut [u8]> {
    postcard::to_slice(message, buffer)
}

pub fn decode(bytes: &[u8]) -> postcard::Result<Message> {
    postcard::from_bytes(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn heartbeat_round_trips() {
        let message = Message::Heartbeat {
            node: NodeId::Vision,
            uptime_ms: 123_456_789,
        };
        let mut buffer = [0u8; MAX_MESSAGE_SIZE];

        let encoded = encode(&message, &mut buffer).unwrap();

        assert_eq!(decode(encoded).unwrap(), message);
    }

    #[test]
    fn encode_fails_when_buffer_is_too_small() {
        let message = Message::Heartbeat {
            node: NodeId::Turret,
            uptime_ms: u64::MAX,
        };
        let mut buffer = [0u8; 2];

        assert!(encode(&message, &mut buffer).is_err());
    }
}
