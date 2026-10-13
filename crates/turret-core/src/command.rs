#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FireMode {
    Single,
    Burst,
    Auto,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FireRequest {
    pub mode: FireMode,
    pub speed: u8,
    pub burst: u8,
}

impl FireRequest {
    pub const MAX_SPEED: u8 = 180;

    pub fn throttle_permille(&self) -> u16 {
        (u32::from(self.speed) * 1000 / u32::from(Self::MAX_SPEED)) as u16
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    Move {
        pan: u16,
        tilt: u16,
        pan_speed: u16,
        tilt_speed: u16,
    },
    Fire(FireRequest),
    CeaseFire,
    Battery,
    Setup,
    Arm,
    Disarm,
    Status,
    Ping(u8),
    Help,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParseError {
    Empty,
    Unknown,
    Missing(&'static str),
    Invalid(&'static str),
}

pub fn parse(line: &str) -> Result<Command, ParseError> {
    let line = line.trim();
    let mut fields = line.split(';');
    let head = fields.next().unwrap_or_default();
    match head {
        "" => Err(ParseError::Empty),
        "P" => parse_move(&mut fields),
        "F" => parse_fire(&mut fields),
        "B" => Ok(Command::Battery),
        "S" => Ok(Command::Setup),
        _ => parse_console(line),
    }
}

fn parse_move<'a>(fields: &mut impl Iterator<Item = &'a str>) -> Result<Command, ParseError> {
    Ok(Command::Move {
        pan: field(fields, "pan")?,
        tilt: field(fields, "tilt")?,
        pan_speed: field(fields, "pan speed")?,
        tilt_speed: field(fields, "tilt speed")?,
    })
}

fn parse_fire<'a>(fields: &mut impl Iterator<Item = &'a str>) -> Result<Command, ParseError> {
    let trigger: u8 = field(fields, "fire")?;
    if trigger == 0 {
        return Ok(Command::CeaseFire);
    }
    let mode = match field::<u8>(fields, "mode")? {
        0 => FireMode::Single,
        1 => FireMode::Burst,
        2 => FireMode::Auto,
        _ => return Err(ParseError::Invalid("mode")),
    };
    let speed: u8 = field(fields, "speed")?;
    if speed > FireRequest::MAX_SPEED {
        return Err(ParseError::Invalid("speed"));
    }
    let burst = fields
        .next()
        .map(|burst| {
            burst
                .trim()
                .parse()
                .map_err(|_| ParseError::Invalid("burst"))
        })
        .transpose()?
        .unwrap_or(1);
    Ok(Command::Fire(FireRequest { mode, speed, burst }))
}

fn parse_console(line: &str) -> Result<Command, ParseError> {
    let mut words = line.split_whitespace();
    let verb = words.next().unwrap_or_default();
    let command = if verb.eq_ignore_ascii_case("arm") {
        Command::Arm
    } else if verb.eq_ignore_ascii_case("disarm") {
        Command::Disarm
    } else if verb.eq_ignore_ascii_case("status") {
        Command::Status
    } else if verb.eq_ignore_ascii_case("help") {
        Command::Help
    } else if verb.eq_ignore_ascii_case("ping") {
        Command::Ping(field(&mut words, "id")?)
    } else {
        return Err(ParseError::Unknown);
    };
    Ok(command)
}

fn field<'a, T: core::str::FromStr>(
    fields: &mut impl Iterator<Item = &'a str>,
    name: &'static str,
) -> Result<T, ParseError> {
    fields
        .next()
        .ok_or(ParseError::Missing(name))?
        .trim()
        .parse()
        .map_err(|_| ParseError::Invalid(name))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_app_move() {
        assert_eq!(
            parse("P;512;300;100;50"),
            Ok(Command::Move {
                pan: 512,
                tilt: 300,
                pan_speed: 100,
                tilt_speed: 50
            })
        );
    }

    #[test]
    fn move_needs_all_four_fields() {
        assert_eq!(
            parse("P;512;300;100"),
            Err(ParseError::Missing("tilt speed"))
        );
    }

    #[test]
    fn rejects_non_numeric_fields() {
        assert_eq!(parse("P;abc;300;100;50"), Err(ParseError::Invalid("pan")));
    }

    #[test]
    fn parses_app_fire_as_sent_by_dartdroid() {
        assert_eq!(
            parse("F;1;1;80;3;0"),
            Ok(Command::Fire(FireRequest {
                mode: FireMode::Burst,
                speed: 80,
                burst: 3
            }))
        );
    }

    #[test]
    fn fire_without_burst_defaults_to_one() {
        assert_eq!(
            parse("F;1;0;40"),
            Ok(Command::Fire(FireRequest {
                mode: FireMode::Single,
                speed: 40,
                burst: 1
            }))
        );
    }

    #[test]
    fn fire_zero_is_cease_fire() {
        assert_eq!(parse("F;0;2;80;3;0"), Ok(Command::CeaseFire));
    }

    #[test]
    fn fire_rejects_unknown_mode_and_overspeed() {
        assert_eq!(parse("F;1;7;80;3;0"), Err(ParseError::Invalid("mode")));
        assert_eq!(parse("F;1;0;181;3;0"), Err(ParseError::Invalid("speed")));
    }

    #[test]
    fn app_speed_scale_maps_to_throttle() {
        let request = |speed| FireRequest {
            mode: FireMode::Single,
            speed,
            burst: 1,
        };

        assert_eq!(request(0).throttle_permille(), 0);
        assert_eq!(request(90).throttle_permille(), 500);
        assert_eq!(request(180).throttle_permille(), 1000);
    }

    #[test]
    fn parses_battery_and_setup_requests() {
        assert_eq!(parse("B;"), Ok(Command::Battery));
        assert_eq!(parse("S;anything"), Ok(Command::Setup));
    }

    #[test]
    fn parses_console_words_case_insensitively() {
        assert_eq!(parse(" ARM "), Ok(Command::Arm));
        assert_eq!(parse("disarm"), Ok(Command::Disarm));
        assert_eq!(parse("Status"), Ok(Command::Status));
        assert_eq!(parse("help"), Ok(Command::Help));
        assert_eq!(parse("ping 1"), Ok(Command::Ping(1)));
    }

    #[test]
    fn reports_empty_and_unknown_lines() {
        assert_eq!(parse("   "), Err(ParseError::Empty));
        assert_eq!(parse("launch"), Err(ParseError::Unknown));
        assert_eq!(parse("ping"), Err(ParseError::Missing("id")));
    }
}
