use heapless::String;

pub const MAX_LINE: usize = 96;

#[derive(Debug, Default)]
pub struct LineBuffer {
    line: String<MAX_LINE>,
    overflowed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LineEvent {
    Line(String<MAX_LINE>),
    TooLong,
}

impl LineBuffer {
    pub fn push(&mut self, byte: u8) -> Option<LineEvent> {
        match byte {
            b'\r' | b'\n' => self.finish(),
            0x08 | 0x7F => {
                self.line.pop();
                None
            }
            byte if byte.is_ascii() && !byte.is_ascii_control() => {
                if self.line.push(char::from(byte)).is_err() {
                    self.overflowed = true;
                }
                None
            }
            _ => None,
        }
    }

    fn finish(&mut self) -> Option<LineEvent> {
        let line = core::mem::take(&mut self.line);
        if core::mem::take(&mut self.overflowed) {
            return Some(LineEvent::TooLong);
        }
        (!line.is_empty()).then_some(LineEvent::Line(line))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn feed(buffer: &mut LineBuffer, bytes: &[u8]) -> Option<LineEvent> {
        bytes.iter().fold(None, |_, byte| buffer.push(*byte))
    }

    fn line(text: &str) -> Option<LineEvent> {
        Some(LineEvent::Line(String::try_from(text).unwrap()))
    }

    #[test]
    fn completes_lines_on_cr_or_lf() {
        let mut buffer = LineBuffer::default();

        assert_eq!(feed(&mut buffer, b"arm\r"), line("arm"));
        assert_eq!(feed(&mut buffer, b"status\n"), line("status"));
    }

    #[test]
    fn crlf_yields_a_single_line() {
        let mut buffer = LineBuffer::default();

        assert_eq!(feed(&mut buffer, b"arm\r"), line("arm"));
        assert_eq!(buffer.push(b'\n'), None);
    }

    #[test]
    fn backspace_removes_the_previous_character() {
        let mut buffer = LineBuffer::default();

        assert_eq!(feed(&mut buffer, b"armx\x7f\r"), line("arm"));
    }

    #[test]
    fn overlong_lines_are_reported_and_discarded() {
        let mut buffer = LineBuffer::default();
        let long = [b'a'; MAX_LINE + 1];

        feed(&mut buffer, &long);

        assert_eq!(buffer.push(b'\r'), Some(LineEvent::TooLong));
        assert_eq!(feed(&mut buffer, b"ok\r"), line("ok"));
    }
}
