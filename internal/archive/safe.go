package archive

import (
	"errors"
	"path/filepath"
	"strings"
)

// ErrEscapingMember reports a member whose name would place it outside the
// folder being extracted into. An archive can name anything it likes, including
// a path that climbs out of the destination, and unpacking that would write
// wherever the archive said.
var ErrEscapingMember = errors.New("this archive holds an entry that points outside the folder")

// DestinationFor is where a member lands under a destination folder, refusing
// any name that would not stay inside it.
func DestinationFor(destination string, member Member) (string, error) {
	cleaned := filepath.Clean(filepath.FromSlash(member.Name))
	if filepath.IsAbs(cleaned) || cleaned == ".." || strings.HasPrefix(cleaned, ".."+string(filepath.Separator)) {
		return "", ErrEscapingMember
	}
	target := filepath.Join(destination, cleaned)
	if !strings.HasPrefix(target, destination+string(filepath.Separator)) {
		return "", ErrEscapingMember
	}
	return target, nil
}
