use std::io::{Read, Write};
use std::sync::Arc;

use esp_idf_svc::sys::{
    esp, esp_get_free_heap_size, heap_caps_get_free_size, uart_driver_install, uart_port_t,
    uart_vfs_dev_use_driver, MALLOC_CAP_INTERNAL, MALLOC_CAP_SPIRAM,
};
use turret_core::line::{LineBuffer, LineEvent};
use vision_core::people::Role;

use crate::pipeline::Shared;
use crate::task;
use crate::wifi;

const CONSOLE_UART: uart_port_t = 0;
const RX_BUFFER: i32 = 256;
const TX_BUFFER: i32 = 1024;

const HELP: &str = "\
help     list commands
status   frame rate, stage timings, faces, memory
people   enrolled people";

pub fn start(shared: Arc<Shared>) -> anyhow::Result<()> {
    esp!(unsafe {
        uart_driver_install(
            CONSOLE_UART,
            RX_BUFFER,
            TX_BUFFER,
            0,
            core::ptr::null_mut(),
            0,
        )
    })?;
    unsafe { uart_vfs_dev_use_driver(CONSOLE_UART as i32) };
    task::spawn_with_psram_stack(c"console", 6 * 1024, move || read_commands(&shared))
}

fn read_commands(shared: &Shared) {
    let mut stdin = std::io::stdin();
    let mut lines = LineBuffer::default();
    let mut byte = [0u8; 1];
    loop {
        if stdin.read_exact(&mut byte).is_err() {
            continue;
        }
        echo(byte[0]);
        match lines.push(byte[0]) {
            Some(LineEvent::Line(line)) => run(line.trim(), shared),
            Some(LineEvent::TooLong) => println!("? line too long"),
            None => {}
        }
    }
}

fn run(line: &str, shared: &Shared) {
    match line {
        "" => {}
        "help" => println!("{HELP}"),
        "status" => {
            let stats = *shared.stats.lock().unwrap();
            println!(
                "{:.1} fps, capture {} ms, detect {} ms, embed {} ms, encode {} ms, {} faces, {} tracks, {} viewers",
                stats.fps,
                stats.capture_ms,
                stats.detect_ms,
                stats.embed_ms,
                stats.encode_ms,
                stats.faces,
                stats.tracks,
                shared.viewers()
            );
            println!(
                "camera brightness {} of 255, last JPEG {} B",
                stats.luma, stats.jpeg_bytes
            );
            let (internal, psram, total) = unsafe {
                (
                    heap_caps_get_free_size(MALLOC_CAP_INTERNAL),
                    heap_caps_get_free_size(MALLOC_CAP_SPIRAM),
                    esp_get_free_heap_size(),
                )
            };
            println!(
                "free heap {total} B (internal {internal} B, PSRAM {psram} B), SD card {}, softAP {}",
                if shared.card { "mounted" } else { "missing" },
                wifi::SSID
            );
        }
        "people" => {
            let people = shared.people.lock().unwrap().clone();
            if people.is_empty() {
                println!("nobody enrolled");
            }
            for person in people {
                let role = match person.role {
                    Role::Target => "target",
                    Role::Protected => "protected",
                };
                println!(
                    "{:>3} {} ({role}, {} samples)",
                    person.id, person.name, person.samples
                );
            }
        }
        _ => println!("? {line}: unknown command, try help"),
    }
}

fn echo(byte: u8) {
    let mut stdout = std::io::stdout();
    let echoed = match byte {
        b'\r' | b'\n' => stdout.write_all(b"\n"),
        0x08 | 0x7F => stdout.write_all(b"\x08 \x08"),
        _ => stdout.write_all(&[byte]),
    };
    let _ = echoed.and_then(|()| stdout.flush());
}
