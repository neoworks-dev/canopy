// Package git reports what a working tree says about the files in one
// directory: which branch is checked out and which entries are modified,
// untracked or ignored.
//
// Everything here is best effort. A directory outside a repository, a git that
// is not installed and a repository git refuses to read all come back as "no
// repository", because none of them is a reason to fail a listing.
package git

import (
	"os"
	"path/filepath"
	"strings"
)

// Repository is the working tree a directory belongs to.
type Repository struct {
	Root   string `json:"root"`
	Branch string `json:"branch"`
}

// Report is what one directory looks like to git: the repository it is in and
// the state of the entries directly inside it, keyed by entry name.
type Report struct {
	Repository Repository        `json:"repository"`
	IsRepo     bool              `json:"isRepo"`
	Statuses   map[string]string `json:"statuses"`
}

const detachedBranchName = "detached"

// Describe reports the state of one directory. The statuses cover the entries
// directly inside it, with a folder carrying the most notable state of anything
// underneath it.
func Describe(directory string) Report {
	root, found := repositoryRoot(directory)
	if !found {
		return Report{Statuses: map[string]string{}}
	}
	repository := Repository{Root: root, Branch: branchAt(root)}
	return Report{Repository: repository, IsRepo: true, Statuses: statusesIn(root, directory)}
}

// repositoryRoot walks up from a directory looking for the working tree it
// belongs to.
func repositoryRoot(directory string) (string, bool) {
	current := directory
	for current != "" {
		if hasGitDirectory(current) {
			return current, true
		}
		current = parentOf(current)
	}
	return "", false
}

func hasGitDirectory(directory string) bool {
	_, err := os.Stat(filepath.Join(directory, ".git"))
	return err == nil
}

func parentOf(directory string) string {
	parent := filepath.Dir(directory)
	if parent == directory {
		return ""
	}
	return parent
}

// branchAt reads the checked-out branch straight out of HEAD, which is cheaper
// than asking git and is the same answer.
func branchAt(root string) string {
	head, err := os.ReadFile(headPath(root))
	if err != nil {
		return ""
	}
	contents := strings.TrimSpace(string(head))
	reference, isSymbolic := strings.CutPrefix(contents, "ref: ")
	if !isSymbolic {
		return shortCommit(contents)
	}
	return filepath.Base(reference)
}

// A worktree and a submodule keep a file where the directory would be, holding
// the path of the real git directory.
func headPath(root string) string {
	marker := filepath.Join(root, ".git")
	info, err := os.Stat(marker)
	if err == nil && info.IsDir() {
		return filepath.Join(marker, "HEAD")
	}
	return filepath.Join(gitDirectoryFrom(marker, root), "HEAD")
}

func gitDirectoryFrom(marker string, root string) string {
	contents, err := os.ReadFile(marker)
	if err != nil {
		return marker
	}
	path := strings.TrimSpace(strings.TrimPrefix(strings.TrimSpace(string(contents)), "gitdir:"))
	if filepath.IsAbs(path) {
		return path
	}
	return filepath.Join(root, path)
}

// A detached head has no branch name, so the commit it sits on stands in for
// one.
func shortCommit(commit string) string {
	if len(commit) < 7 {
		return detachedBranchName
	}
	return commit[:7]
}
