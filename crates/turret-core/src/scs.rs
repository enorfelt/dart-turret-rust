use heapless::Vec;

pub const BROADCAST_ID: u8 = 0xFE;
pub const GOAL_POSITION: u8 = 42;
pub const MAX_PACKET: usize = 16;

const HEADER: [u8; 2] = [0xFF, 0xFF];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Instruction {
    Ping = 0x01,
    Read = 0x02,
    Write = 0x03,
    RegWrite = 0x04,
    Action = 0x05,
}

pub type Packet = Vec<u8, MAX_PACKET>;

pub fn packet(id: u8, instruction: Instruction, params: &[u8]) -> Packet {
    let length = params.len() as u8 + 2;
    let mut packet = Packet::new();
    packet.extend_from_slice(&HEADER).unwrap();
    packet
        .extend_from_slice(&[id, length, instruction as u8])
        .unwrap();
    packet.extend_from_slice(params).unwrap();
    packet.push(checksum(&packet[2..])).unwrap();
    packet
}

pub fn ping(id: u8) -> Packet {
    packet(id, Instruction::Ping, &[])
}

pub fn action() -> Packet {
    packet(BROADCAST_ID, Instruction::Action, &[])
}

pub fn reg_write_position(id: u8, position: u16, time: u16, speed: u16) -> Packet {
    let [p_hi, p_lo] = position.to_be_bytes();
    let [t_hi, t_lo] = time.to_be_bytes();
    let [s_hi, s_lo] = speed.to_be_bytes();
    packet(
        id,
        Instruction::RegWrite,
        &[GOAL_POSITION, p_hi, p_lo, t_hi, t_lo, s_hi, s_lo],
    )
}

fn checksum(bytes: &[u8]) -> u8 {
    !bytes.iter().fold(0u8, |sum, byte| sum.wrapping_add(*byte))
}

pub fn strip_echo<'a>(reply: &'a [u8], request: &[u8]) -> &'a [u8] {
    reply.strip_prefix(request).unwrap_or(reply)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Status {
    pub id: u8,
    pub error: u8,
    pub data: Vec<u8, MAX_PACKET>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusError {
    Incomplete,
    BadHeader,
    BadLength,
    BadChecksum,
}

pub fn parse_status(bytes: &[u8]) -> Result<Status, StatusError> {
    let start = bytes
        .windows(2)
        .position(|pair| pair == HEADER)
        .ok_or(StatusError::BadHeader)?;
    let frame = &bytes[start..];
    let [_, _, id, length, ..] = *frame else {
        return Err(StatusError::Incomplete);
    };
    let length = usize::from(length);
    if !(2..=MAX_PACKET).contains(&length) {
        return Err(StatusError::BadLength);
    }
    let end = 4 + length;
    if frame.len() < end {
        return Err(StatusError::Incomplete);
    }
    let body = &frame[2..end - 1];
    if checksum(body) != frame[end - 1] {
        return Err(StatusError::BadChecksum);
    }
    Ok(Status {
        id,
        error: frame[4],
        data: Vec::from_slice(&frame[5..end - 1]).unwrap(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ping_matches_feetech_reference() {
        assert_eq!(ping(1).as_slice(), &[0xFF, 0xFF, 0x01, 0x02, 0x01, 0xFB]);
    }

    #[test]
    fn action_is_broadcast() {
        assert_eq!(action().as_slice(), &[0xFF, 0xFF, 0xFE, 0x02, 0x05, 0xFA]);
    }

    #[test]
    fn reg_write_position_is_big_endian_from_goal_position() {
        let packet = reg_write_position(0, 0x0200, 0, 1000);

        assert_eq!(
            packet.as_slice(),
            &[0xFF, 0xFF, 0x00, 0x09, 0x04, 42, 0x02, 0x00, 0x00, 0x00, 0x03, 0xE8, 0xDB]
        );
    }

    #[test]
    fn parses_ping_reply_after_leading_noise() {
        let status = parse_status(&[0x00, 0xFF, 0xFF, 0x01, 0x02, 0x00, 0xFC]).unwrap();

        assert_eq!(status.id, 1);
        assert_eq!(status.error, 0);
        assert!(status.data.is_empty());
    }

    #[test]
    fn parses_reply_with_data() {
        let reply = [0xFF, 0xFF, 0x01, 0x04, 0x00, 0x01, 0xF4, 0x05];

        let status = parse_status(&reply).unwrap();

        assert_eq!(status.data.as_slice(), &[0x01, 0xF4]);
    }

    #[test]
    fn half_duplex_echo_of_the_request_is_skipped() {
        let request = ping(1);
        let mut bus = Vec::<u8, 32>::from_slice(&request).unwrap();
        bus.extend_from_slice(&[0xFF, 0xFF, 0x01, 0x02, 0x00, 0xFC])
            .unwrap();

        let status = parse_status(strip_echo(&bus, &request)).unwrap();

        assert_eq!(status.error, 0);
    }

    #[test]
    fn reply_without_echo_is_left_untouched() {
        let reply = [0xFF, 0xFF, 0x01, 0x02, 0x00, 0xFC];

        assert_eq!(strip_echo(&reply, &ping(1)), &reply);
    }

    #[test]
    fn rejects_corrupted_reply() {
        assert_eq!(
            parse_status(&[0xFF, 0xFF, 0x01, 0x02, 0x00, 0x00]),
            Err(StatusError::BadChecksum)
        );
    }

    #[test]
    fn reports_truncated_reply() {
        assert_eq!(
            parse_status(&[0xFF, 0xFF, 0x01, 0x02, 0x00]),
            Err(StatusError::Incomplete)
        );
    }
}
