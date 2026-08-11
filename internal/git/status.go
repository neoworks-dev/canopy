package git

import (
	"context"
	"os/exec"
	"path/filepath"
	"strings"
	"time"
)

// A repository that takes longer than this to report is one whose status is not
// worth holding a listing for.
const statusTimeout = 5 * time.Second

// What an entry can be, most notable first. A folder shows whichever of these
// comes first among everything underneath it.
var statusPriority = []string{
	"conflicted",
	"modified",
	"added",
	"deleted",
	"renamed",
	"untracked",
	"ignored",
}

// The two porcelain columns are the staged and unstaged states; a file that is
// both staged and changed again reads as modified either way.
var statusByCode = map[byte]string{
	'M': "modified",
	'T': "modified",
	'A': "added",
	'D': "deleted",
	'R': "renamed",
	'C': "added",
	'U': "conflicted",
}

// statusesIn reports the state of the entries directly inside a directory,
// keyed by entry name.
func statusesIn(root string, directory string) map[string]string {
	prefix := relativePrefix(root, directory)
	if prefix == "" && directory != root {
		return map[string]string{}
	}
	return foldIntoEntries(runStatus(directory), prefix)
}

// The pathspec confines the walk to the directory being listed, so a listing
// inside a large repository does not pay for the whole of it.
func runStatus(directory string) []string {
	ctx, cancel := context.WithTimeout(context.Background(), statusTimeout)
	defer cancel()
	command := exec.CommandContext(ctx, "git", "-C", directory, "status",
		"--porcelain=v1", "-z", "--ignored=matching", "--untracked-files=normal", "--", ".")
	output, err := command.Output()
	if err != nil {
		return nil
	}
	return strings.Split(string(output), "\x00")
}

// relativePrefix is where the listed directory sits inside the repository, with
// a trailing separator, empty at the root.
func relativePrefix(root string, directory string) string {
	relative, err := filepath.Rel(root, directory)
	if err != nil || relative == "." {
		return ""
	}
	if strings.HasPrefix(relative, "..") {
		return ""
	}
	return relative + string(filepath.Separator)
}

// foldIntoEntries turns paths anywhere under the directory into the entry that
// holds them, so a folder carries the state of its contents.
func foldIntoEntries(records []string, prefix string) map[string]string {
	statuses := map[string]string{}
	for _, record := range records {
		applyRecord(statuses, record, prefix)
	}
	return statuses
}

func applyRecord(statuses map[string]string, record string, prefix string) {
	name, status, ok := parseRecord(record, prefix)
	if !ok {
		return
	}
	statuses[name] = moreNotable(statuses[name], status)
}

// A record is two status columns, a space and a path. The rename form carries a
// second path in the next record, which has no columns of its own and is
// skipped by the same length check.
func parseRecord(record string, prefix string) (string, string, bool) {
	if len(record) < 4 || record[2] != ' ' {
		return "", "", false
	}
	status, known := statusFor(record[0], record[1])
	if !known {
		return "", "", false
	}
	name, inside := entryNameOf(record[3:], prefix)
	if !inside {
		return "", "", false
	}
	return name, status, true
}

func statusFor(staged byte, unstaged byte) (string, bool) {
	if staged == '?' {
		return "untracked", true
	}
	if staged == '!' {
		return "ignored", true
	}
	return firstKnownStatus(staged, unstaged)
}

func firstKnownStatus(staged byte, unstaged byte) (string, bool) {
	unstagedStatus, hasUnstaged := statusByCode[unstaged]
	if hasUnstaged {
		return unstagedStatus, true
	}
	stagedStatus, hasStaged := statusByCode[staged]
	return stagedStatus, hasStaged
}

// entryNameOf is the first path segment below the listed directory, which is
// the entry standing for everything deeper.
func entryNameOf(path string, prefix string) (string, bool) {
	trimmed := strings.TrimSuffix(path, "/")
	if !strings.HasPrefix(trimmed, prefix) {
		return "", false
	}
	inside := strings.TrimPrefix(trimmed, prefix)
	if inside == "" {
		return "", false
	}
	return strings.SplitN(inside, "/", 2)[0], true
}

func moreNotable(current string, candidate string) string {
	if current == "" {
		return candidate
	}
	if rankOf(candidate) < rankOf(current) {
		return candidate
	}
	return current
}

func rankOf(status string) int {
	for rank, known := range statusPriority {
		if known == status {
			return rank
		}
	}
	return len(statusPriority)
}
