package fileops

import (
	"errors"
	"io"
	"os"
	"path/filepath"
	"strings"

	"file-explorer-neoworks/internal/archive"
)

// The name an archive of several entries takes, since no single one of them is
// the thing being packed.
const bundleName = "Archive"

// runExtract unpacks one archive into the destination folder, creating it if it
// is not there yet: extracting into a folder of its own is what keeps an archive
// of loose files from scattering them over the folder beside it.
func (op *operation) runExtract(source string, handle *control) error {
	err := handle.waitWhilePaused()
	if err != nil {
		return err
	}
	err = op.prepareDestination()
	if err != nil {
		return err
	}
	unpacked := map[string]bool{}
	return archive.Walk(source, func(member archive.Member, contents io.Reader) error {
		return op.unpack(member, contents, unpacked, handle)
	})
}

// prepareDestination makes the folder to unpack into, remembering it as the one
// thing to remove if the extraction is taken back.
func (op *operation) prepareDestination() error {
	if exists(op.destination) {
		return nil
	}
	err := os.MkdirAll(op.destination, 0o755)
	if err != nil {
		return err
	}
	op.rememberCreated(op.destination)
	return nil
}

// unpack writes one member. A member the user chose to skip leaves the rest of
// the archive alone, which is what makes the conflict prompt worth answering per
// entry rather than for the archive as a whole.
func (op *operation) unpack(
	member archive.Member,
	contents io.Reader,
	unpacked map[string]bool,
	handle *control,
) error {
	err := handle.waitWhilePaused()
	if err != nil {
		return err
	}
	handle.setCurrent(member.Name)
	err = op.unpackMember(member, contents, unpacked, handle)
	if errors.Is(err, ErrSkipped) {
		return nil
	}
	return err
}

func (op *operation) unpackMember(
	member archive.Member,
	contents io.Reader,
	unpacked map[string]bool,
	handle *control,
) error {
	target, err := archive.DestinationFor(op.destination, member)
	if err != nil {
		return err
	}
	op.rememberTopLevel(member, unpacked)
	if member.IsDir {
		return os.MkdirAll(target, 0o755)
	}
	return op.unpackFile(member, contents, target, handle)
}

func (op *operation) unpackFile(
	member archive.Member,
	contents io.Reader,
	target string,
	handle *control,
) error {
	err := os.MkdirAll(filepath.Dir(target), 0o755)
	if err != nil {
		return err
	}
	resolved, err := handle.resolveTarget(member.Name, target)
	if err != nil {
		return err
	}
	return writeUnpacked(resolved, contents, handle)
}

func writeUnpacked(target string, contents io.Reader, handle *control) error {
	file, err := os.OpenFile(target, os.O_WRONLY|os.O_CREATE|os.O_TRUNC, 0o644)
	if err != nil {
		return err
	}
	err = writeContents(file, contents, handle)
	if err != nil {
		return abandonTarget(file, target, err)
	}
	return file.Close()
}

// rememberTopLevel records what the archive adds to a folder that was already
// there, which is what an undo of the extraction removes. Only the topmost name
// of each member is worth keeping: everything below it goes with it.
func (op *operation) rememberTopLevel(member archive.Member, unpacked map[string]bool) {
	top := strings.Split(member.Name, "/")[0]
	if top == "" || unpacked[top] {
		return
	}
	unpacked[top] = true
	if op.createdDestination() {
		return
	}
	op.rememberCreated(filepath.Join(op.destination, top))
}

// createdDestination reports whether the destination folder is itself something
// this operation made, in which case removing it covers everything inside.
func (op *operation) createdDestination() bool {
	op.mutex.Lock()
	defer op.mutex.Unlock()
	return len(op.created) > 0 && op.created[0] == op.destination
}

// runCompress packs every source into one archive, which is why it runs over the
// whole batch rather than once per source like the other kinds.
func (op *operation) runCompress() {
	handle := op.control()
	target, err := op.archiveTarget()
	if err != nil {
		op.recordOutcome(0, op.destination, err)
		return
	}
	err = archive.Create(target, op.sources, func(name string, written int64) error {
		return op.reportPacked(name, written, handle)
	})
	op.finishCompression(target, err)
}

func (op *operation) finishCompression(target string, err error) {
	if err != nil {
		os.Remove(target)
		op.recordOutcome(len(op.sources)-1, target, err)
		return
	}
	op.rememberCreated(target)
	op.recordOutcome(len(op.sources)-1, target, nil)
}

func (op *operation) reportPacked(name string, written int64, handle *control) error {
	err := handle.waitWhilePaused()
	if err != nil {
		return err
	}
	handle.setCurrent(name)
	handle.addBytes(written)
	return nil
}

// archiveTarget is where the new archive lands: named after the one entry being
// packed, or after nothing in particular when there are several. A name already
// taken gains a counter rather than failing the operation.
func (op *operation) archiveTarget() (string, error) {
	name := SuggestName(op.destination, archiveNameFor(op.sources))
	return FreeName(op.destination, name)
}

func archiveNameFor(sources []string) string {
	if len(sources) == 1 {
		return archive.SuggestedName(filepath.Base(sources[0]))
	}
	return archive.SuggestedName(bundleName)
}

// ExtractionFolderFor is where an archive unpacks when nobody chose a folder: one
// named after the archive, beside it, under a name nothing else has taken.
func ExtractionFolderFor(archivePath string) string {
	directory := filepath.Dir(archivePath)
	return filepath.Join(directory, SuggestName(directory, archive.BaseName(filepath.Base(archivePath))))
}
