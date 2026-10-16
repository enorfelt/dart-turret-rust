use alloc::string::String;
use alloc::vec::Vec;

pub fn query_param(uri: &str, key: &str) -> Option<String> {
    let (_, query) = uri.split_once('?')?;
    query.split('&').find_map(|pair| {
        let (name, value) = pair.split_once('=').unwrap_or((pair, ""));
        (decode(name)? == key).then(|| decode(value)).flatten()
    })
}

pub fn resource<'a>(uri: &'a str, prefix: &str) -> Option<(u16, &'a str)> {
    let path = uri.split('?').next()?.strip_prefix(prefix)?;
    let (id, action) = path.split_once('/').unwrap_or((path, ""));
    Some((id.parse().ok()?, action))
}

fn decode(text: &str) -> Option<String> {
    let mut bytes = Vec::with_capacity(text.len());
    let mut input = text.bytes();
    while let Some(byte) = input.next() {
        match byte {
            b'+' => bytes.push(b' '),
            b'%' => {
                let high = hex(input.next()?)?;
                let low = hex(input.next()?)?;
                bytes.push(high << 4 | low);
            }
            other => bytes.push(other),
        }
    }
    String::from_utf8(bytes).ok()
}

fn hex(digit: u8) -> Option<u8> {
    char::from(digit).to_digit(16).map(|value| value as u8)
}

pub fn json_string(text: &str, out: &mut String) {
    out.push('"');
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            c if (c as u32) < 0x20 => {
                let _ = core::fmt::Write::write_fmt(out, format_args!("\\u{:04x}", c as u32));
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_and_decodes_query_parameters() {
        let uri = "/api/people?name=Ada+L%C3%B6f&role=target";

        assert_eq!(query_param(uri, "name").as_deref(), Some("Ada Löf"));
        assert_eq!(query_param(uri, "role").as_deref(), Some("target"));
        assert_eq!(query_param(uri, "missing"), None);
        assert_eq!(query_param("/api/people", "name"), None);
    }

    #[test]
    fn malformed_escapes_are_rejected() {
        assert_eq!(query_param("/x?name=%G1", "name"), None);
        assert_eq!(query_param("/x?name=%4", "name"), None);
        assert_eq!(query_param("/x?name=%FF", "name"), None);
    }

    #[test]
    fn flag_without_value_is_empty() {
        assert_eq!(query_param("/x?name", "name").as_deref(), Some(""));
    }

    #[test]
    fn splits_resource_id_and_action() {
        assert_eq!(
            resource("/api/people/12/photo", "/api/people/"),
            Some((12, "photo"))
        );
        assert_eq!(
            resource("/api/people/12/photo?t=5", "/api/people/"),
            Some((12, "photo"))
        );
        assert_eq!(resource("/api/people/7", "/api/people/"), Some((7, "")));
        assert_eq!(resource("/api/people/7?x=1", "/api/people/"), Some((7, "")));
        assert_eq!(resource("/api/people/abc", "/api/people/"), None);
        assert_eq!(resource("/other/7", "/api/people/"), None);
    }

    #[test]
    fn json_strings_are_escaped() {
        let mut out = String::new();
        json_string("a\"b\\c\u{1}", &mut out);

        assert_eq!(out, "\"a\\\"b\\\\c\\u0001\"");
    }
}
