package filesystem

import (
	"sort"
	"strings"
)

// Sort keys. A key the frontend does not recognise, or an empty one, lists by
// name, which is the order a directory is expected to arrive in.
const (
	SortByName        = "name"
	SortBySize        = "size"
	SortByModified    = "modified"
	SortByPermissions = "permissions"
)

// Order is how a listing is sorted. Directories-first is independent of the key,
// so sorting by size still keeps folders together.
type Order struct {
	Key              string `json:"key"`
	Descending       bool   `json:"descending"`
	DirectoriesFirst bool   `json:"directoriesFirst"`
}

// DefaultOrder is what a listing arrives in before anything is chosen.
func DefaultOrder() Order {
	return Order{Key: SortByName, Descending: false, DirectoriesFirst: true}
}

func sortEntries(entries []Entry, order Order) {
	sort.SliceStable(entries, func(firstIndex, secondIndex int) bool {
		return isBefore(entries[firstIndex], entries[secondIndex], order)
	})
}

func isBefore(first Entry, second Entry, order Order) bool {
	if order.DirectoriesFirst && first.IsDir != second.IsDir {
		return first.IsDir
	}
	difference := compareByKey(first, second, order.Key)
	if difference == 0 {
		// Names break every tie, so entries of the same size or timestamp still
		// arrive in a stable, readable order.
		return compareNames(first, second) < 0
	}
	if order.Descending {
		return difference > 0
	}
	return difference < 0
}

func compareByKey(first Entry, second Entry, key string) int {
	if key == SortBySize {
		return compareInt64(first.Size, second.Size)
	}
	if key == SortByModified {
		return first.modifiedAt.Compare(second.modifiedAt)
	}
	if key == SortByPermissions {
		return strings.Compare(first.Mode, second.Mode)
	}
	return compareNames(first, second)
}

// Names compare case-insensitively, falling back to the raw name so two entries
// differing only in case still have a defined order.
func compareNames(first Entry, second Entry) int {
	folded := strings.Compare(strings.ToLower(first.Name), strings.ToLower(second.Name))
	if folded != 0 {
		return folded
	}
	return strings.Compare(first.Name, second.Name)
}

func compareInt64(first int64, second int64) int {
	if first < second {
		return -1
	}
	if first > second {
		return 1
	}
	return 0
}
