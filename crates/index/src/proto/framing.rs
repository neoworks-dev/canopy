use std::io::{self, Read, Write};

/// Bounds a single frame so a corrupt or hostile length prefix cannot make the
/// peer allocate without limit.
pub const MAXIMUM_MESSAGE_BYTES: usize = 8 << 20;

const HEADER_BYTES: usize = 4;

/// The payload of the `io::Error` returned for a frame over the size limit.
#[derive(Debug, thiserror::Error)]
#[error("proto: message exceeds size limit: {0} bytes")]
pub struct MessageTooLarge(pub usize);

pub fn is_message_too_large(error: &io::Error) -> bool {
    error.get_ref().is_some_and(|inner| inner.is::<MessageTooLarge>())
}

fn too_large(length: usize) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, MessageTooLarge(length))
}

/// Writes one length-prefixed frame in a single write so a frame is never split
/// across syscalls.
pub fn write_message(writer: &mut impl Write, payload: &[u8]) -> io::Result<()> {
    if payload.len() > MAXIMUM_MESSAGE_BYTES {
        return Err(too_large(payload.len()));
    }
    let mut frame = Vec::with_capacity(HEADER_BYTES + payload.len());
    frame.extend_from_slice(&(payload.len() as u32).to_be_bytes());
    frame.extend_from_slice(payload);
    writer.write_all(&frame)
}

/// Reads one length-prefixed frame. A stream that ends before or inside a frame
/// yields `ErrorKind::UnexpectedEof`.
pub fn read_message(reader: &mut impl Read) -> io::Result<Vec<u8>> {
    let mut header = [0u8; HEADER_BYTES];
    reader.read_exact(&mut header)?;
    let length = u32::from_be_bytes(header) as usize;
    if length > MAXIMUM_MESSAGE_BYTES {
        return Err(too_large(length));
    }
    let mut payload = vec![0u8; length];
    reader.read_exact(&mut payload)?;
    Ok(payload)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_and_read_round_trip() {
        let payloads: Vec<Vec<u8>> = vec![br#"{"id":1,"method":"handshake"}"#.to_vec(), Vec::new(), vec![b'x'; 70_000]];
        let mut buffer = Vec::new();
        for payload in &payloads {
            write_message(&mut buffer, payload).unwrap();
        }
        let mut reader = buffer.as_slice();
        for expected in &payloads {
            assert_eq!(&read_message(&mut reader).unwrap(), expected);
        }
    }

    #[test]
    fn read_reports_end_of_stream() {
        let error = read_message(&mut io::empty()).unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::UnexpectedEof);
    }

    #[test]
    fn read_rejects_oversized_length_prefix() {
        let header = ((MAXIMUM_MESSAGE_BYTES + 1) as u32).to_be_bytes();
        let error = read_message(&mut header.as_slice()).unwrap_err();
        assert!(is_message_too_large(&error));
    }

    #[test]
    fn write_rejects_oversized_payload() {
        let error = write_message(&mut io::sink(), &vec![0u8; MAXIMUM_MESSAGE_BYTES + 1]).unwrap_err();
        assert!(is_message_too_large(&error));
    }

    #[test]
    fn read_rejects_truncated_payload() {
        let mut buffer = Vec::new();
        write_message(&mut buffer, b"0123456789").unwrap();
        let truncated = &buffer[..HEADER_BYTES + 4];
        let error = read_message(&mut &truncated[..]).unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::UnexpectedEof);
    }
}
