use crate::identity::Class;
use crate::rect::Rect;

pub const VERSION: u8 = 1;
pub const HEADER_LEN: usize = 9;
pub const FACE_LEN: usize = 11;
pub const MAX_STREAM_FACES: usize = 8;

pub const FLAG_RECOGNITION: u8 = 1 << 0;
pub const FLAG_ENROLLING: u8 = 1 << 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StreamFace {
    pub track: u8,
    pub class: Class,
    pub confidence: u8,
    pub rect: Rect,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StreamHeader {
    pub flags: u8,
    pub seq: u16,
    pub width: u16,
    pub height: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecodeError {
    TooShort,
    UnknownVersion(u8),
    BadClass(u8),
}

pub const fn frame_header_len(faces: usize) -> usize {
    HEADER_LEN + faces * FACE_LEN
}

pub fn class_code(class: Class) -> u8 {
    match class {
        Class::Unknown => 0,
        Class::Protected => 1,
        Class::Target => 2,
    }
}

fn class_from_code(code: u8) -> Result<Class, DecodeError> {
    match code {
        0 => Ok(Class::Unknown),
        1 => Ok(Class::Protected),
        2 => Ok(Class::Target),
        other => Err(DecodeError::BadClass(other)),
    }
}

pub fn encode_header(header: &StreamHeader, faces: &[StreamFace], out: &mut [u8]) -> usize {
    let faces = &faces[..faces.len().min(MAX_STREAM_FACES)];
    let len = frame_header_len(faces.len());
    let out = &mut out[..len];
    out[0] = VERSION;
    out[1] = header.flags;
    out[2..4].copy_from_slice(&header.seq.to_le_bytes());
    out[4..6].copy_from_slice(&header.width.to_le_bytes());
    out[6..8].copy_from_slice(&header.height.to_le_bytes());
    out[8] = faces.len() as u8;
    for (face, chunk) in faces
        .iter()
        .zip(out[HEADER_LEN..].chunks_exact_mut(FACE_LEN))
    {
        chunk[0] = face.track;
        chunk[1] = class_code(face.class);
        chunk[2] = face.confidence;
        let x = face.rect.x0.max(0) as u16;
        let y = face.rect.y0.max(0) as u16;
        chunk[3..5].copy_from_slice(&x.to_le_bytes());
        chunk[5..7].copy_from_slice(&y.to_le_bytes());
        chunk[7..9].copy_from_slice(&(face.rect.width() as u16).to_le_bytes());
        chunk[9..11].copy_from_slice(&(face.rect.height() as u16).to_le_bytes());
    }
    len
}

pub fn decode(
    bytes: &[u8],
) -> Result<
    (
        StreamHeader,
        heapless::Vec<StreamFace, MAX_STREAM_FACES>,
        &[u8],
    ),
    DecodeError,
> {
    if bytes.len() < HEADER_LEN {
        return Err(DecodeError::TooShort);
    }
    if bytes[0] != VERSION {
        return Err(DecodeError::UnknownVersion(bytes[0]));
    }
    let u16_at = |i: usize| u16::from_le_bytes([bytes[i], bytes[i + 1]]);
    let header = StreamHeader {
        flags: bytes[1],
        seq: u16_at(2),
        width: u16_at(4),
        height: u16_at(6),
    };
    let count = usize::from(bytes[8]).min(MAX_STREAM_FACES);
    let end = frame_header_len(count);
    if bytes.len() < end {
        return Err(DecodeError::TooShort);
    }
    let mut faces = heapless::Vec::new();
    for chunk in bytes[HEADER_LEN..end].chunks_exact(FACE_LEN) {
        let at = |i: usize| u16::from_le_bytes([chunk[i], chunk[i + 1]]) as i16;
        let (x, y, w, h) = (at(3), at(5), at(7), at(9));
        let _ = faces.push(StreamFace {
            track: chunk[0],
            class: class_from_code(chunk[1])?,
            confidence: chunk[2],
            rect: Rect::new(x, y, x + w, y + h),
        });
    }
    Ok((header, faces, &bytes[end..]))
}

#[cfg(test)]
mod tests {
    use super::*;

    const HEADER: StreamHeader = StreamHeader {
        flags: FLAG_RECOGNITION,
        seq: 0x0102,
        width: 320,
        height: 240,
    };

    const FACES: [StreamFace; 2] = [
        StreamFace {
            track: 7,
            class: Class::Target,
            confidence: 83,
            rect: Rect::new(100, 50, 160, 130),
        },
        StreamFace {
            track: 9,
            class: Class::Protected,
            confidence: 61,
            rect: Rect::new(10, 20, 40, 60),
        },
    ];

    const FRAME: [u8; 31 + 2] = [
        0x01, 0x01, 0x02, 0x01, 0x40, 0x01, 0xF0, 0x00, 0x02, //
        0x07, 0x02, 0x53, 0x64, 0x00, 0x32, 0x00, 0x3C, 0x00, 0x50, 0x00, //
        0x09, 0x01, 0x3D, 0x0A, 0x00, 0x14, 0x00, 0x1E, 0x00, 0x28, 0x00, //
        0xFF, 0xD8,
    ];

    #[test]
    fn encodes_header_and_faces() {
        let mut out = [0u8; 64];

        let len = encode_header(&HEADER, &FACES, &mut out);

        assert_eq!(len, 31);
        assert_eq!(&out[..len], &FRAME[..31]);
    }

    #[test]
    fn decodes_header_faces_and_jpeg() {
        let (header, faces, jpeg) = decode(&FRAME).unwrap();

        assert_eq!(header, HEADER);
        assert_eq!(faces.as_slice(), FACES);
        assert_eq!(jpeg, [0xFF, 0xD8]);
    }

    #[test]
    fn empty_frame_is_just_the_header() {
        let mut out = [0u8; HEADER_LEN];
        let header = StreamHeader {
            flags: 0,
            seq: 0,
            width: 320,
            height: 240,
        };

        assert_eq!(encode_header(&header, &[], &mut out), HEADER_LEN);
        assert_eq!(out, [0x01, 0x00, 0x00, 0x00, 0x40, 0x01, 0xF0, 0x00, 0x00]);
    }

    #[test]
    fn negative_box_origin_is_clamped() {
        let mut out = [0u8; 32];
        let face = StreamFace {
            track: 1,
            class: Class::Unknown,
            confidence: 0,
            rect: Rect::new(-5, -3, 20, 30),
        };

        encode_header(&HEADER, &[face], &mut out);

        assert_eq!(&out[12..16], [0x00, 0x00, 0x00, 0x00]);
    }

    #[test]
    fn rejects_truncated_and_foreign_frames() {
        assert_eq!(decode(&FRAME[..8]), Err(DecodeError::TooShort));
        assert_eq!(decode(&FRAME[..20]), Err(DecodeError::TooShort));
        let mut foreign = FRAME;
        foreign[0] = 2;
        assert_eq!(decode(&foreign), Err(DecodeError::UnknownVersion(2)));
        let mut bad = FRAME;
        bad[10] = 5;
        assert_eq!(decode(&bad), Err(DecodeError::BadClass(5)));
    }
}
