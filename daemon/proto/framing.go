package proto

import (
	"encoding/binary"
	"errors"
	"fmt"
	"io"
)

// maxMessageBytes bounds a single frame so a corrupt or hostile length prefix
// cannot make the peer allocate without limit.
const maxMessageBytes = 8 << 20

const headerBytes = 4

var ErrMessageTooLarge = errors.New("proto: message exceeds size limit")

// WriteMessage writes one length-prefixed frame in a single Write so a frame is
// never split across syscalls.
func WriteMessage(writer io.Writer, payload []byte) error {
	if len(payload) > maxMessageBytes {
		return fmt.Errorf("%w: %d bytes", ErrMessageTooLarge, len(payload))
	}
	frame := make([]byte, headerBytes+len(payload))
	binary.BigEndian.PutUint32(frame[:headerBytes], uint32(len(payload)))
	copy(frame[headerBytes:], payload)
	_, err := writer.Write(frame)
	return err
}

// ReadMessage reads one length-prefixed frame. A clean EOF on the header is
// returned as io.EOF so callers can treat it as a closed connection.
func ReadMessage(reader io.Reader) ([]byte, error) {
	header := make([]byte, headerBytes)
	if _, err := io.ReadFull(reader, header); err != nil {
		return nil, err
	}
	length := binary.BigEndian.Uint32(header)
	if length > maxMessageBytes {
		return nil, fmt.Errorf("%w: %d bytes", ErrMessageTooLarge, length)
	}
	payload := make([]byte, length)
	if _, err := io.ReadFull(reader, payload); err != nil {
		return nil, err
	}
	return payload, nil
}
