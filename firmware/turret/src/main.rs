use std::time::{Duration, Instant};

use turret_proto::{Message, NodeId, MAX_MESSAGE_SIZE};

fn main() {
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    let boot = Instant::now();
    let mut buffer = [0u8; MAX_MESSAGE_SIZE];

    loop {
        let heartbeat = Message::Heartbeat {
            node: NodeId::Turret,
            uptime_ms: boot.elapsed().as_millis() as u64,
        };
        match turret_proto::encode(&heartbeat, &mut buffer) {
            Ok(frame) => log::info!("{heartbeat:?} -> {frame:02x?}"),
            Err(error) => log::error!("failed to encode {heartbeat:?}: {error}"),
        }
        std::thread::sleep(Duration::from_secs(1));
    }
}
