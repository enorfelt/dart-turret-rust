mod board;
mod camera;
mod console;
mod face;
mod http;
mod pipeline;
mod storage;
mod task;
mod wifi;

use std::sync::mpsc;

use esp_idf_svc::hal::peripherals::Peripherals;
use vision_core::config::VisionConfig;
use vision_core::people::People;

use crate::board::Board;
use crate::camera::Camera;
use crate::face::{Detector, Embedder};
use crate::pipeline::Vision;
use crate::storage::Storage;

const WIFI_PASSWORD: &str = valid_password(env!(
    "VISION_WIFI_PASSWORD",
    "set VISION_WIFI_PASSWORD to the softAP password (8 to 63 characters) before building"
));

fn main() {
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    if let Err(error) = run() {
        log::error!("vision stopped: {error:#}");
    }
    loop {
        std::thread::park();
    }
}

fn run() -> anyhow::Result<()> {
    let config = VisionConfig::default();
    let peripherals = Peripherals::take()?;
    let board = Board::new(peripherals.pins);

    let camera = Camera::new(board.camera)?;
    let detector = Detector::new()?;
    let embedder = Embedder::new()?;
    let storage = match Storage::mount(peripherals.sdmmc1, board.sd) {
        Ok(storage) => Some(storage),
        Err(error) => {
            log::warn!("no SD card, recognition and enrollment are off: {error:#}");
            None
        }
    };
    let people = match &storage {
        Some(storage) => storage.load_people(embedder.len()).unwrap_or_else(|error| {
            log::error!("people not loaded, starting empty: {error:#}");
            People::new(embedder.len())
        }),
        None => People::new(embedder.len()),
    };
    log::info!("{} people enrolled", people.len());

    let wifi = wifi::start_access_point(peripherals.modem, WIFI_PASSWORD)?;
    let (requests, received) = mpsc::channel();
    let shared = pipeline::start(
        Vision {
            camera,
            detector,
            embedder,
            storage,
            people,
            config,
        },
        received,
    )?;
    let server = http::start(shared.clone(), requests)?;
    console::start(shared)?;
    let internal =
        unsafe { esp_idf_svc::sys::heap_caps_get_free_size(esp_idf_svc::sys::MALLOC_CAP_INTERNAL) };
    log::info!("vision ready, {internal} B internal RAM free; type help");
    std::mem::forget(wifi);
    std::mem::forget(server);
    Ok(())
}

const fn valid_password(password: &str) -> &str {
    let len = password.len();
    assert!(
        len >= 8 && len <= 63,
        "VISION_WIFI_PASSWORD must be 8 to 63 characters"
    );
    password
}
