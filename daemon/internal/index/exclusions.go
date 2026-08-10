package index

import "strings"

// excludedDirectoryNames are trees that are large, machine-generated, and never
// what a user searches for. Skipping them is the difference between an index of
// tens of thousands of entries and one of millions.
var excludedDirectoryNames = []string{
	".git",
	".hg",
	".svn",
	".cache",
	".npm",
	".bun",
	".gradle",
	".venv",
	"venv",
	"node_modules",
	"__pycache__",
	"target",
	"vendor",
	"dist",
	"build",
	".next",
	".svelte-kit",
	".terraform",
}

// excludedPathPrefixes are kernel and runtime filesystems. Walking them is
// pointless and can block on device nodes.
var excludedPathPrefixes = []string{
	"/proc",
	"/sys",
	"/dev",
	"/run",
	"/snap",
	"/var/lib/docker",
}

// Exclusions decides which directories a crawl refuses to descend into.
type Exclusions struct {
	directoryNames map[string]struct{}
	pathPrefixes   []string
}

func DefaultExclusions() *Exclusions {
	names := make(map[string]struct{}, len(excludedDirectoryNames))
	for _, name := range excludedDirectoryNames {
		names[name] = struct{}{}
	}
	return &Exclusions{directoryNames: names, pathPrefixes: excludedPathPrefixes}
}

// SkipsDirectory reports whether a directory should be left unindexed. Callers
// must not apply it to the root of a scan; a user who explicitly adds
// ~/projects/node_modules as a root means it.
func (exclusions *Exclusions) SkipsDirectory(path string, name string) bool {
	if _, excluded := exclusions.directoryNames[name]; excluded {
		return true
	}
	return exclusions.hasExcludedPrefix(path)
}

func (exclusions *Exclusions) hasExcludedPrefix(path string) bool {
	for _, prefix := range exclusions.pathPrefixes {
		if path == prefix || strings.HasPrefix(path, prefix+"/") {
			return true
		}
	}
	return false
}
