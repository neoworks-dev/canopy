//! Percent-encoding for the path part of a URL, the way `.trashinfo` files and
//! thumbnail cache identities write it.

/// Escapes a path, leaving letters, digits, `-_.~` and the characters a URL
/// path allows (`$&+,/:;=@`) readable.
pub fn escape_path(path: &str) -> String {
    let mut escaped = String::with_capacity(path.len());
    for byte in path.bytes() {
        if keeps_literal(byte) {
            escaped.push(byte as char);
            continue;
        }
        escaped.push_str(&format!("%{byte:02X}"));
    }
    escaped
}

fn keeps_literal(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || b"-_.~$&+,/:;=@".contains(&byte)
}

/// Decodes `%XX` escapes, or `None` when one is malformed or the result is not
/// UTF-8.
pub fn unescape_path(escaped: &str) -> Option<String> {
    let bytes = escaped.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] != b'%' {
            decoded.push(bytes[index]);
            index += 1;
            continue;
        }
        decoded.push(escaped_byte(bytes, index)?);
        index += 3;
    }
    String::from_utf8(decoded).ok()
}

fn escaped_byte(bytes: &[u8], percent_index: usize) -> Option<u8> {
    let digits = bytes.get(percent_index + 1..percent_index + 3)?;
    let text = std::str::from_utf8(digits).ok()?;
    u8::from_str_radix(text, 16).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spaces_and_non_ascii_are_escaped() {
        assert_eq!(escape_path("/home/a b/ä.txt"), "/home/a%20b/%C3%A4.txt");
    }

    #[test]
    fn escaping_round_trips() {
        let path = "/home/user/100% (copy)?.txt";
        assert_eq!(unescape_path(&escape_path(path)).as_deref(), Some(path));
    }

    #[test]
    fn a_malformed_escape_is_refused() {
        assert_eq!(unescape_path("/a%2"), None);
        assert_eq!(unescape_path("/a%zz"), None);
    }
}
