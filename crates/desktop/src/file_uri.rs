use percent_encoding::{AsciiSet, NON_ALPHANUMERIC, percent_encode};

/// Everything but the characters a URL path may carry unescaped.
const ESCAPED_IN_PATH: &AsciiSet = &NON_ALPHANUMERIC
    .remove(b'-')
    .remove(b'_')
    .remove(b'.')
    .remove(b'~')
    .remove(b'$')
    .remove(b'&')
    .remove(b'+')
    .remove(b',')
    .remove(b'/')
    .remove(b':')
    .remove(b';')
    .remove(b'=')
    .remove(b'@');

/// The `file://` URI for a path, escaped with `ESCAPED_IN_PATH`.
pub(crate) fn from_path(path: &str) -> String {
    if path.is_empty() {
        return "file:".to_string();
    }
    format!("file://{}", percent_encode(path.as_bytes(), ESCAPED_IN_PATH))
}

/// The local path a `file:` URI names. Anything that is not a well-formed
/// `file:` URI with a path gives `None`.
pub(crate) fn to_path(uri: &str) -> Option<String> {
    let (scheme, rest) = uri.split_once(':')?;
    if !is_file_scheme(scheme) {
        return None;
    }
    let without_fragment = rest.split('#').next()?;
    let without_query = without_fragment.split('?').next()?;
    let path = path_after_authority(without_query)?;
    if path.is_empty() {
        return None;
    }
    percent_decode_strict(path)
}

fn is_file_scheme(scheme: &str) -> bool {
    scheme.eq_ignore_ascii_case("file")
}

fn path_after_authority(rest: &str) -> Option<&str> {
    if let Some(after_slashes) = rest.strip_prefix("//") {
        let path_start = after_slashes.find('/')?;
        return Some(&after_slashes[path_start..]);
    }
    if rest.starts_with('/') {
        return Some(rest);
    }
    None
}

fn percent_decode_strict(text: &str) -> Option<String> {
    let bytes = text.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] != b'%' {
            decoded.push(bytes[index]);
            index += 1;
            continue;
        }
        decoded.push(decode_escape(bytes.get(index + 1..index + 3)?)?);
        index += 3;
    }
    Some(String::from_utf8_lossy(&decoded).into_owned())
}

fn decode_escape(digits: &[u8]) -> Option<u8> {
    let high = hex_value(digits[0])?;
    let low = hex_value(digits[1])?;
    Some(high * 16 + low)
}

fn hex_value(digit: u8) -> Option<u8> {
    match digit {
        b'0'..=b'9' => Some(digit - b'0'),
        b'a'..=b'f' => Some(digit - b'a' + 10),
        b'A'..=b'F' => Some(digit - b'A' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spaces_and_percent_signs_are_escaped() {
        assert_eq!(from_path("/home/user/a b.txt"), "file:///home/user/a%20b.txt");
        assert_eq!(from_path("/100%"), "file:///100%25");
    }

    #[test]
    fn reserved_path_characters_stay_readable() {
        assert_eq!(from_path("/a+b,c;d=e@f:g"), "file:///a+b,c;d=e@f:g");
    }

    #[test]
    fn paths_round_trip() {
        let path = "/home/user/ünï cödé #1?.txt";
        assert_eq!(to_path(&from_path(path)).as_deref(), Some(path));
    }

    #[test]
    fn a_host_is_dropped() {
        assert_eq!(to_path("file://localhost/tmp/a").as_deref(), Some("/tmp/a"));
    }

    #[test]
    fn other_schemes_and_broken_escapes_are_refused() {
        assert_eq!(to_path("https://example.org/a"), None);
        assert_eq!(to_path("file:///a%zz"), None);
        assert_eq!(to_path("file:///a%2"), None);
        assert_eq!(to_path("file://"), None);
    }
}
