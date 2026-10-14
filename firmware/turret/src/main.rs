mod actuators;
mod battery;
mod ble;
mod board;
mod console;
mod gimbal;
mod light_gate;
mod turret;

use std::sync::mpsc;

use esp_idf_svc::hal::gpio::{PinDriver, Pull};
use esp_idf_svc::hal::peripherals::Peripherals;
use esp_idf_svc::nvs::EspDefaultNvsPartition;
use turret_core::config::TurretConfig;

use crate::actuators::{ActuatorPins, Actuators};
use crate::battery::BatteryMonitor;
use crate::board::Board;
use crate::gimbal::Gimbal;
use crate::light_gate::LightGate;
use crate::turret::Turret;

const PASSKEY: u32 = parse_passkey(env!(
    "TURRET_BLE_PASSKEY",
    "set TURRET_BLE_PASSKEY to the 6-digit BLE pairing passkey before building"
));

fn main() -> anyhow::Result<()> {
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    let config = TurretConfig::default();
    let peripherals = Peripherals::take()?;
    let _nvs = EspDefaultNvsPartition::take()?;
    let board = Board::new(peripherals.pins);

    let actuators = Actuators::new(
        peripherals.ledc,
        ActuatorPins {
            esc_left: board.esc_left,
            esc_right: board.esc_right,
            pusher: board.pusher,
        },
        config,
    )?;
    let gimbal = Gimbal::new(
        peripherals.uart1,
        board.servo_tx,
        board.servo_rx,
        config.gimbal,
    )?;
    let light_gate = LightGate::new(board.light_gate)?;
    let arm_switch = PinDriver::input(board.arm_switch, Pull::Up)?;
    let battery = BatteryMonitor::new(peripherals.adc1, board.battery, config.battery)?;

    let (inputs, received) = mpsc::channel();
    let remote = ble::start(peripherals.modem, inputs.clone(), PASSKEY)?;
    console::start(inputs)?;

    Turret {
        actuators,
        gimbal,
        light_gate,
        arm_switch,
        battery,
        remote,
        config,
    }
    .run(received)
}

const fn parse_passkey(digits: &str) -> u32 {
    let digits = digits.as_bytes();
    assert!(digits.len() == 6, "TURRET_BLE_PASSKEY must be 6 digits");
    let mut passkey = 0;
    let mut i = 0;
    while i < digits.len() {
        assert!(
            digits[i].is_ascii_digit(),
            "TURRET_BLE_PASSKEY must be 6 digits"
        );
        passkey = passkey * 10 + (digits[i] - b'0') as u32;
        i += 1;
    }
    passkey
}
