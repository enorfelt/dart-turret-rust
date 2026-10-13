mod actuators;
mod battery;
mod board;
mod console;
mod gimbal;
mod light_gate;
mod turret;

use std::sync::mpsc;

use esp_idf_svc::hal::peripherals::Peripherals;
use turret_core::config::TurretConfig;

use crate::actuators::{ActuatorPins, Actuators};
use crate::battery::BatteryMonitor;
use crate::board::Board;
use crate::gimbal::Gimbal;
use crate::light_gate::LightGate;
use crate::turret::Turret;

fn main() -> anyhow::Result<()> {
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    let config = TurretConfig::default();
    let peripherals = Peripherals::take()?;
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
    let battery = BatteryMonitor::new(peripherals.adc2, board.battery, config.battery)?;

    let (commands, received) = mpsc::channel();
    console::start(commands)?;

    Turret {
        actuators,
        gimbal,
        light_gate,
        battery,
        config,
    }
    .run(received)
}
