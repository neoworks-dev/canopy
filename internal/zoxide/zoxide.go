// Package zoxide reads the frecency-ranked directories zoxide keeps, so the
// explorer can jump to a folder by a fragment of its name.
//
// The database is read through "zoxide query --list --score" rather than by
// parsing db.zo. The file is a bincode dump of zoxide's own Rust structs whose
// layout has changed between releases and is not a published interface, while
// the command is. The cost the direct read would have avoided — a process per
// keystroke — is avoided instead by dumping the whole list once and matching in
// process, so typing never spawns anything.
package zoxide

import (
	"os"
	"os/exec"
	"sort"
	"strconv"
	"strings"
	"sync"
	"time"
)

const binaryName = "zoxide"

// Long enough that typing a query never spawns more than one process, short
// enough that a directory visited in a terminal shows up while the window is
// still open.
const cacheLifetime = 5 * time.Second

// Entry is one directory zoxide knows about, with the frecency score it ranks by.
type Entry struct {
	Path  string  `json:"path"`
	Score float64 `json:"score"`
}

// Database holds the last dump of zoxide's list. A missing binary is recorded
// once and every later query answers empty, so an explorer on a machine without
// zoxide pays nothing and reports nothing.
type Database struct {
	mutex     sync.Mutex
	entries   []Entry
	readAt    time.Time
	installed bool
	checked   bool
}

func New() *Database {
	return &Database{}
}

// Query ranks the directories matching text, most frecent first. An empty text
// returns the top of the list. The directory in exclude is left out, since
// jumping to where the window already is means nothing.
func (database *Database) Query(text string, exclude string, limit int) []Entry {
	entries := database.list()
	keywords := splitKeywords(text)
	matched := make([]Entry, 0, limit)
	for _, entry := range entries {
		matched = appendUsable(matched, entry, keywords, exclude, limit)
	}
	return matched
}

// Available reports whether zoxide is installed, for hiding a prompt that could
// never return anything.
func (database *Database) Available() bool {
	database.list()
	database.mutex.Lock()
	defer database.mutex.Unlock()
	return database.installed
}

func appendUsable(matched []Entry, entry Entry, keywords []string, exclude string, limit int) []Entry {
	if len(matched) >= limit {
		return matched
	}
	if entry.Path == exclude || !matchesKeywords(entry.Path, keywords) {
		return matched
	}
	if !isDirectory(entry.Path) {
		return matched
	}
	return append(matched, entry)
}

// A directory zoxide remembers may since have been deleted or unmounted, and is
// only stat'd once it is a candidate for the answer.
func isDirectory(path string) bool {
	info, err := os.Stat(path)
	if err != nil {
		return false
	}
	return info.IsDir()
}

func (database *Database) list() []Entry {
	database.mutex.Lock()
	defer database.mutex.Unlock()

	if !database.shouldRead() {
		return database.entries
	}
	database.entries = database.read()
	database.readAt = time.Now()
	return database.entries
}

func (database *Database) shouldRead() bool {
	if database.checked && !database.installed {
		return false
	}
	return time.Since(database.readAt) > cacheLifetime
}

func (database *Database) read() []Entry {
	output, err := exec.Command(binaryName, "query", "--list", "--score").Output()
	database.checked = true
	database.installed = err == nil
	if err != nil {
		return nil
	}
	return parseList(string(output))
}

// parseList reads the "  112.0 /path/to/directory" lines zoxide prints. A line
// that does not look like one is skipped rather than failing the whole dump.
//
// The list is ranked here rather than trusted to arrive ranked, because a query
// stops as soon as it has filled its limit and would otherwise return whichever
// matches came first rather than the most frecent ones.
func parseList(output string) []Entry {
	entries := []Entry{}
	for _, line := range strings.Split(output, "\n") {
		entries = appendParsed(entries, line)
	}
	sort.SliceStable(entries, func(first int, second int) bool {
		return entries[first].Score > entries[second].Score
	})
	return entries
}

func appendParsed(entries []Entry, line string) []Entry {
	entry, ok := parseLine(line)
	if !ok {
		return entries
	}
	return append(entries, entry)
}

func parseLine(line string) (Entry, bool) {
	fields := strings.SplitN(strings.TrimSpace(line), " ", 2)
	if len(fields) != 2 {
		return Entry{}, false
	}
	score, err := strconv.ParseFloat(fields[0], 64)
	if err != nil {
		return Entry{}, false
	}
	path := strings.TrimLeft(fields[1], " ")
	if path == "" {
		return Entry{}, false
	}
	return Entry{Path: path, Score: score}, true
}
