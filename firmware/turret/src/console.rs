use std::io::{Read, Write};
use std::sync::mpsc::Sender;
use std::thread;

use esp_idf_svc::sys::{
    esp, esp_vfs_usb_serial_jtag_use_driver, usb_serial_jtag_driver_config_t,
    usb_serial_jtag_driver_install, EspError,
};
use turret_core::command::{self, ParseError};
use turret_core::line::{LineBuffer, LineEvent};

use crate::turret::Input;

pub fn start(commands: Sender<Input>) -> Result<(), EspError> {
    let mut config = usb_serial_jtag_driver_config_t {
        tx_buffer_size: 1024,
        rx_buffer_size: 256,
    };
    esp!(unsafe { usb_serial_jtag_driver_install(&mut config) })?;
    unsafe { esp_vfs_usb_serial_jtag_use_driver() };
    thread::Builder::new()
        .name("console".into())
        .stack_size(6 * 1024)
        .spawn(move || read_commands(commands))
        .expect("console thread");
    Ok(())
}

fn read_commands(commands: Sender<Input>) {
    let mut stdin = std::io::stdin();
    let mut lines = LineBuffer::default();
    let mut byte = [0u8; 1];
    loop {
        if stdin.read_exact(&mut byte).is_err() {
            continue;
        }
        echo(byte[0]);
        match lines.push(byte[0]) {
            Some(LineEvent::Line(line)) => match command::parse(&line) {
                Ok(command) => {
                    if commands.send(Input::Command(command)).is_err() {
                        return;
                    }
                }
                Err(ParseError::Empty) => {}
                Err(error) => println!("? {line}: {}", describe(error)),
            },
            Some(LineEvent::TooLong) => println!("? line too long"),
            None => {}
        }
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

fn describe(error: ParseError) -> String {
    match error {
        ParseError::Empty => "empty".into(),
        ParseError::Unknown => "unknown command, try help".into(),
        ParseError::Missing(field) => format!("missing {field}"),
        ParseError::Invalid(field) => format!("invalid {field}"),
    }
}
