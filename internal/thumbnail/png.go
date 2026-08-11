package thumbnail

import (
	"bytes"
	"encoding/binary"
	"errors"
	"hash/crc32"
)

// The thumbnail managing standard identifies a cached image by the source it
// was made from and the modification time it had then, both stored as PNG text
// chunks so any file manager can validate the same cache.
const (
	uriKeyword   = "Thumb::URI"
	mtimeKeyword = "Thumb::MTime"
)

const (
	signatureLength = 8
	lengthBytes     = 4
	typeBytes       = 4
	checksumBytes   = 4
	headerChunkType = "IHDR"
	textChunkType   = "tEXt"
)

var errNotPNG = errors.New("not a PNG image")

// withThumbnailMetadata inserts the standard text chunks after the header, so
// the caller can annotate an image produced by the standard encoder or by an
// external renderer without decoding it again.
func withThumbnailMetadata(encoded []byte, uri string, modifiedAt string) ([]byte, error) {
	end, err := headerChunkEnd(encoded)
	if err != nil {
		return nil, err
	}
	annotated := bytes.Clone(encoded[:end])
	annotated = append(annotated, textChunk(uriKeyword, uri)...)
	annotated = append(annotated, textChunk(mtimeKeyword, modifiedAt)...)
	return append(annotated, encoded[end:]...), nil
}

// thumbnailMetadata reports the source and modification time a cached image
// records, empty when it carries neither.
func thumbnailMetadata(encoded []byte) (string, string) {
	return textValue(encoded, uriKeyword), textValue(encoded, mtimeKeyword)
}

func headerChunkEnd(encoded []byte) (int, error) {
	if len(encoded) < signatureLength+lengthBytes+typeBytes {
		return 0, errNotPNG
	}
	if chunkTypeAt(encoded, signatureLength) != headerChunkType {
		return 0, errNotPNG
	}
	return chunkEnd(encoded, signatureLength), nil
}

func textValue(encoded []byte, keyword string) string {
	for offset := signatureLength; offset+lengthBytes+typeBytes <= len(encoded); offset = chunkEnd(encoded, offset) {
		value, found := keywordValueAt(encoded, offset, keyword)
		if found {
			return value
		}
	}
	return ""
}

func keywordValueAt(encoded []byte, offset int, keyword string) (string, bool) {
	if chunkTypeAt(encoded, offset) != textChunkType {
		return "", false
	}
	data := chunkData(encoded, offset)
	name, value, separated := bytes.Cut(data, []byte{0})
	if !separated || string(name) != keyword {
		return "", false
	}
	return string(value), true
}

func textChunk(keyword string, text string) []byte {
	data := append([]byte(keyword+"\x00"), text...)
	chunk := binary.BigEndian.AppendUint32(nil, uint32(len(data)))
	chunk = append(chunk, textChunkType...)
	chunk = append(chunk, data...)
	return binary.BigEndian.AppendUint32(chunk, crc32.ChecksumIEEE(chunk[lengthBytes:]))
}

func chunkTypeAt(encoded []byte, offset int) string {
	return string(encoded[offset+lengthBytes : offset+lengthBytes+typeBytes])
}

func chunkLengthAt(encoded []byte, offset int) int {
	return int(binary.BigEndian.Uint32(encoded[offset : offset+lengthBytes]))
}

func chunkEnd(encoded []byte, offset int) int {
	return offset + lengthBytes + typeBytes + chunkLengthAt(encoded, offset) + checksumBytes
}

// chunkData returns the payload of the chunk at offset, empty when the declared
// length runs past the end of a truncated file.
func chunkData(encoded []byte, offset int) []byte {
	start := offset + lengthBytes + typeBytes
	end := start + chunkLengthAt(encoded, offset)
	if end > len(encoded) {
		return nil
	}
	return encoded[start:end]
}
