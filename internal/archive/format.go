// Package archive reads and writes the archive formats a file manager is
// expected to open: zip and the tar family. It knows about formats and members
// and nothing about destinations, progress or conflicts, which belong to
// whoever is driving it.
package archive

import (
	"errors"
	"strings"
)

// Format is how an archive is packed, worked out from the name rather than the
// contents: a file manager labels an entry before it has opened it.
type Format string

const (
	FormatZip      Format = "zip"
	FormatTar      Format = "tar"
	FormatTarGzip  Format = "tar.gz"
	FormatTarBzip2 Format = "tar.bz2"
	FormatUnknown  Format = ""
)

var (
	ErrUnsupportedFormat = errors.New("this is not an archive the explorer can open")
	// Bzip2 has no writer in the standard library, and a format that can only be
	// read is not one to offer for packing.
	ErrCannotWriteFormat = errors.New("this archive format can only be read")
)

// The longer suffixes come first, so a .tar.gz is not read as a .gz.
var suffixFormats = []struct {
	suffix string
	format Format
}{
	{".tar.gz", FormatTarGzip},
	{".tgz", FormatTarGzip},
	{".tar.bz2", FormatTarBzip2},
	{".tbz2", FormatTarBzip2},
	{".tar", FormatTar},
	{".zip", FormatZip},
}

// FormatOf reports how a file is packed, or FormatUnknown when the name says it
// is not an archive at all.
func FormatOf(name string) Format {
	lowered := strings.ToLower(name)
	for _, candidate := range suffixFormats {
		if strings.HasSuffix(lowered, candidate.suffix) {
			return candidate.format
		}
	}
	return FormatUnknown
}

func IsArchive(name string) bool {
	return FormatOf(name) != FormatUnknown
}

// BaseName is an archive's name without the suffix that says how it is packed,
// which is what a folder unpacked from it is called.
func BaseName(name string) string {
	lowered := strings.ToLower(name)
	for _, candidate := range suffixFormats {
		if strings.HasSuffix(lowered, candidate.suffix) {
			return name[:len(name)-len(candidate.suffix)]
		}
	}
	return name
}

// SuggestedName is what an archive holding the given entry should be called.
func SuggestedName(sourceName string) string {
	return sourceName + ".zip"
}
