//! Lowercase hexadecimal text for digests.

/// The bytes as lowercase hexadecimal, two digits each.
pub fn encode(bytes: &[u8]) -> String {
    let mut text = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        text.push_str(&format!("{byte:02x}"));
    }
    text
}

#[cfg(test)]
mod tests {
    #[test]
    fn bytes_are_two_digits_each() {
        assert_eq!(super::encode(&[0x00, 0x0f, 0xa5, 0xff]), "000fa5ff");
    }
}
