package proto

import (
	"bytes"
	"encoding/binary"
	"errors"
	"io"
	"strings"
	"testing"
)

func TestWriteMessageAndReadMessageRoundTrip(t *testing.T) {
	payloads := [][]byte{
		[]byte(`{"id":1,"method":"handshake"}`),
		[]byte(``),
		[]byte(strings.Repeat("x", 70000)),
	}
	var buffer bytes.Buffer
	for _, payload := range payloads {
		if err := WriteMessage(&buffer, payload); err != nil {
			t.Fatalf("write: %v", err)
		}
	}
	for index, expected := range payloads {
		readPayload, err := ReadMessage(&buffer)
		if err != nil {
			t.Fatalf("read %d: %v", index, err)
		}
		if !bytes.Equal(readPayload, expected) {
			t.Fatalf("frame %d round-tripped to %d bytes, want %d", index, len(readPayload), len(expected))
		}
	}
}

func TestReadMessageReportsEndOfStream(t *testing.T) {
	_, err := ReadMessage(bytes.NewReader(nil))
	if !errors.Is(err, io.EOF) {
		t.Fatalf("expected io.EOF on empty stream, got %v", err)
	}
}

func TestReadMessageRejectsOversizedLengthPrefix(t *testing.T) {
	header := make([]byte, headerBytes)
	binary.BigEndian.PutUint32(header, maxMessageBytes+1)

	_, err := ReadMessage(bytes.NewReader(header))
	if !errors.Is(err, ErrMessageTooLarge) {
		t.Fatalf("expected ErrMessageTooLarge, got %v", err)
	}
}

func TestWriteMessageRejectsOversizedPayload(t *testing.T) {
	err := WriteMessage(io.Discard, make([]byte, maxMessageBytes+1))
	if !errors.Is(err, ErrMessageTooLarge) {
		t.Fatalf("expected ErrMessageTooLarge, got %v", err)
	}
}

// A truncated frame must not be mistaken for a short but complete one.
func TestReadMessageRejectsTruncatedPayload(t *testing.T) {
	var buffer bytes.Buffer
	if err := WriteMessage(&buffer, []byte("0123456789")); err != nil {
		t.Fatalf("write: %v", err)
	}
	truncated := buffer.Bytes()[:headerBytes+4]

	_, err := ReadMessage(bytes.NewReader(truncated))
	if !errors.Is(err, io.ErrUnexpectedEOF) {
		t.Fatalf("expected io.ErrUnexpectedEOF, got %v", err)
	}
}
