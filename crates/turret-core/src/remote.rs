use heapless::Vec;

use crate::command::{Command, FireMode, FireRequest};
use crate::fire::{Event, Phase, Rejection};

pub const PROTOCOL_VERSION: u8 = 2;
pub const STATE_LEN: usize = 10;
pub const MAX_EVENT_LEN: usize = 4;
pub const UNKNOWN_POSITION: u16 = 0xFFFF;
pub const NO_SPEED: u16 = 0xFFFF;

const DEFAULT_ATT_PAYLOAD: usize = 20;
const _: () = assert!(STATE_LEN <= DEFAULT_ATT_PAYLOAD && MAX_EVENT_LEN <= DEFAULT_ATT_PAYLOAD);

const OP_ARM: u8 = 0x01;
const OP_DISARM: u8 = 0x02;
const OP_AIM: u8 = 0x03;
const OP_FIRE: u8 = 0x04;
const OP_CEASE_FIRE: u8 = 0x05;

const EVENT_SHOT: u8 = 0x01;
const EVENT_FINISHED: u8 = 0x02;
const EVENT_ABORTED: u8 = 0x03;
const EVENT_FIRE_REJECTED: u8 = 0x04;
const EVENT_MOVE_FAILED: u8 = 0x05;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandError {
    UnknownOp,
    Malformed,
}

impl CommandError {
    pub const fn att_code(self) -> u8 {
        match self {
            Self::UnknownOp => 0x80,
            Self::Malformed => 0x81,
        }
    }
}

pub fn decode_command(bytes: &[u8]) -> Result<Command, CommandError> {
    let (&op, args) = bytes.split_first().ok_or(CommandError::Malformed)?;
    match op {
        OP_ARM => without_args(args, Command::Arm),
        OP_DISARM => without_args(args, Command::Disarm),
        OP_AIM => decode_aim(args),
        OP_FIRE => decode_fire(args),
        OP_CEASE_FIRE => without_args(args, Command::CeaseFire),
        _ => Err(CommandError::UnknownOp),
    }
}

fn without_args(args: &[u8], command: Command) -> Result<Command, CommandError> {
    args.is_empty()
        .then_some(command)
        .ok_or(CommandError::Malformed)
}

fn decode_aim(args: &[u8]) -> Result<Command, CommandError> {
    let &[pan_lo, pan_hi, tilt_lo, tilt_hi, pan_speed_lo, pan_speed_hi, tilt_speed_lo, tilt_speed_hi] =
        args
    else {
        return Err(CommandError::Malformed);
    };
    Ok(Command::Move {
        pan: u16::from_le_bytes([pan_lo, pan_hi]),
        tilt: u16::from_le_bytes([tilt_lo, tilt_hi]),
        pan_speed: u16::from_le_bytes([pan_speed_lo, pan_speed_hi]),
        tilt_speed: u16::from_le_bytes([tilt_speed_lo, tilt_speed_hi]),
    })
}

fn decode_fire(args: &[u8]) -> Result<Command, CommandError> {
    let &[mode, throttle_pct, shots] = args else {
        return Err(CommandError::Malformed);
    };
    let mode = FireMode::try_from(mode).map_err(|()| CommandError::Malformed)?;
    if throttle_pct > FireRequest::MAX_THROTTLE_PCT {
        return Err(CommandError::Malformed);
    }
    Ok(Command::Fire(FireRequest {
        mode,
        throttle_pct,
        burst: shots,
    }))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StateSnapshot {
    pub armed: bool,
    pub ready: bool,
    pub arm_switch: bool,
    pub phase: Phase,
    pub battery_mv: Option<u16>,
    pub battery_pct: u8,
    pub gimbal: Option<(u16, u16)>,
}

impl StateSnapshot {
    pub fn encode(&self) -> [u8; STATE_LEN] {
        let flags =
            u8::from(self.armed) | u8::from(self.ready) << 1 | u8::from(self.arm_switch) << 2;
        let [mv_lo, mv_hi] = self.battery_mv.unwrap_or(0).to_le_bytes();
        let (pan, tilt) = self.gimbal.unwrap_or((UNKNOWN_POSITION, UNKNOWN_POSITION));
        let [pan_lo, pan_hi] = pan.to_le_bytes();
        let [tilt_lo, tilt_hi] = tilt.to_le_bytes();
        [
            PROTOCOL_VERSION,
            flags,
            phase_code(self.phase),
            mv_lo,
            mv_hi,
            self.battery_pct,
            pan_lo,
            pan_hi,
            tilt_lo,
            tilt_hi,
        ]
    }
}

fn phase_code(phase: Phase) -> u8 {
    match phase {
        Phase::Idle => 0,
        Phase::SpinUp => 1,
        Phase::Pushing => 2,
        Phase::Retracting => 3,
        Phase::SpinDown => 4,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RemoteEvent {
    Shot { number: u8, speed_kmh: Option<u32> },
    Finished { shots: u8 },
    Aborted,
    FireRejected(Rejection),
    MoveFailed,
}

impl From<Event> for RemoteEvent {
    fn from(event: Event) -> Self {
        match event {
            Event::Shot { number, speed_kmh } => Self::Shot { number, speed_kmh },
            Event::Finished { shots } => Self::Finished { shots },
            Event::Aborted => Self::Aborted,
        }
    }
}

impl RemoteEvent {
    pub fn encode(&self) -> Vec<u8, MAX_EVENT_LEN> {
        let bytes = match *self {
            Self::Shot { number, speed_kmh } => {
                let [lo, hi] = speed_code(speed_kmh).to_le_bytes();
                Vec::from_slice(&[EVENT_SHOT, number, lo, hi])
            }
            Self::Finished { shots } => Vec::from_slice(&[EVENT_FINISHED, shots]),
            Self::Aborted => Vec::from_slice(&[EVENT_ABORTED]),
            Self::FireRejected(rejection) => {
                Vec::from_slice(&[EVENT_FIRE_REJECTED, rejection_code(rejection)])
            }
            Self::MoveFailed => Vec::from_slice(&[EVENT_MOVE_FAILED]),
        };
        bytes.expect("every event fits MAX_EVENT_LEN")
    }
}

fn speed_code(speed_kmh: Option<u32>) -> u16 {
    speed_kmh.map_or(NO_SPEED, |speed| {
        u16::try_from(speed).map_or(NO_SPEED - 1, |speed| speed.min(NO_SPEED - 1))
    })
}

fn rejection_code(rejection: Rejection) -> u8 {
    match rejection {
        Rejection::Disarmed => 0,
        Rejection::Busy => 1,
        Rejection::CoolingDown => 2,
        Rejection::NoThrottle => 3,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_argument_free_commands() {
        assert_eq!(decode_command(&[0x01]), Ok(Command::Arm));
        assert_eq!(decode_command(&[0x02]), Ok(Command::Disarm));
        assert_eq!(decode_command(&[0x05]), Ok(Command::CeaseFire));
    }

    #[test]
    fn decodes_aim_little_endian() {
        assert_eq!(
            decode_command(&[0x03, 0x00, 0x02, 0x2C, 0x01, 0x64, 0x00, 0x32, 0x00]),
            Ok(Command::Move {
                pan: 512,
                tilt: 300,
                pan_speed: 100,
                tilt_speed: 50
            })
        );
    }

    #[test]
    fn decodes_fire() {
        assert_eq!(
            decode_command(&[0x04, 0x01, 40, 3]),
            Ok(Command::Fire(FireRequest {
                mode: FireMode::Burst,
                throttle_pct: 40,
                burst: 3
            }))
        );
    }

    #[test]
    fn rejects_unknown_ops_and_bad_payloads() {
        assert_eq!(decode_command(&[0x09]), Err(CommandError::UnknownOp));
        assert_eq!(decode_command(&[]), Err(CommandError::Malformed));
        assert_eq!(decode_command(&[0x01, 0x00]), Err(CommandError::Malformed));
        assert_eq!(decode_command(&[0x03, 0x00]), Err(CommandError::Malformed));
        assert_eq!(
            decode_command(&[0x04, 0x03, 40, 1]),
            Err(CommandError::Malformed)
        );
        assert_eq!(
            decode_command(&[0x04, 0x00, 101, 1]),
            Err(CommandError::Malformed)
        );
    }

    #[test]
    fn errors_map_to_application_att_codes() {
        assert_eq!(CommandError::UnknownOp.att_code(), 0x80);
        assert_eq!(CommandError::Malformed.att_code(), 0x81);
    }

    #[test]
    fn encodes_state_byte_exact() {
        let state = StateSnapshot {
            armed: true,
            ready: false,
            arm_switch: true,
            phase: Phase::Pushing,
            battery_mv: Some(15_600),
            battery_pct: 71,
            gimbal: Some((512, 300)),
        };

        assert_eq!(
            state.encode(),
            [2, 0b101, 2, 0xF0, 0x3C, 71, 0x00, 0x02, 0x2C, 0x01]
        );
    }

    #[test]
    fn encodes_unknown_state_values() {
        let state = StateSnapshot {
            armed: false,
            ready: true,
            arm_switch: false,
            phase: Phase::Idle,
            battery_mv: None,
            battery_pct: 0,
            gimbal: None,
        };

        assert_eq!(
            state.encode(),
            [2, 0b010, 0, 0, 0, 0, 0xFF, 0xFF, 0xFF, 0xFF]
        );
    }

    #[test]
    fn encodes_events() {
        let shot = |speed_kmh| {
            RemoteEvent::Shot {
                number: 2,
                speed_kmh,
            }
            .encode()
        };

        assert_eq!(shot(Some(252)), [0x01, 2, 0xFC, 0x00]);
        assert_eq!(shot(None), [0x01, 2, 0xFF, 0xFF]);
        assert_eq!(shot(Some(1_000_000)), [0x01, 2, 0xFE, 0xFF]);
        assert_eq!(RemoteEvent::Finished { shots: 3 }.encode(), [0x02, 3]);
        assert_eq!(RemoteEvent::Aborted.encode(), [0x03]);
        assert_eq!(
            RemoteEvent::FireRejected(Rejection::CoolingDown).encode(),
            [0x04, 2]
        );
        assert_eq!(RemoteEvent::MoveFailed.encode(), [0x05]);
    }

    #[test]
    fn fire_events_convert() {
        assert_eq!(
            RemoteEvent::from(Event::Finished { shots: 1 }),
            RemoteEvent::Finished { shots: 1 }
        );
        assert_eq!(RemoteEvent::from(Event::Aborted), RemoteEvent::Aborted);
    }
}
