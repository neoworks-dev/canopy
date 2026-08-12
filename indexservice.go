package main

import (
	"os"

	"canopy/internal/filesystem"
	"canopy/internal/searchindex"

	"canopy/daemon/proto"
)

// IndexService is the Wails-bound bridge to the shared Neoworks daemon. Directory
// listings still come from FileService; only search and index status need the
// daemon.
type IndexService struct {
	connection *searchindex.Connection
}

func NewIndexService() *IndexService {
	return &IndexService{connection: searchindex.New()}
}

// Search returns indexed entries whose names match the query. An empty query
// yields no hits rather than the whole index. An empty scope searches every
// indexed root; otherwise only the given directory and what is under it.
func (service *IndexService) Search(query string, scope string) (proto.SearchResult, error) {
	return service.connection.Search(query, scope)
}

// SearchEntries runs a query and describes each hit the way a listing describes
// its own, so a saved search can be shown as a folder rather than as a list of
// suggestions. A hit the index still holds but the filesystem has lost is left
// out: the crawl runs behind the tree it follows.
func (service *IndexService) SearchEntries(query string, scope string) ([]filesystem.Entry, error) {
	result, err := service.connection.Search(query, scope)
	if err != nil {
		return nil, err
	}
	return describeHits(result.Hits), nil
}

func describeHits(hits []proto.Hit) []filesystem.Entry {
	entries := []filesystem.Entry{}
	for _, hit := range hits {
		entries = appendExisting(entries, hit.Path)
	}
	return entries
}

func appendExisting(entries []filesystem.Entry, path string) []filesystem.Entry {
	if _, err := os.Lstat(path); err != nil {
		return entries
	}
	return append(entries, filesystem.Describe(path))
}

// IndexStatus tells the frontend whether a crawl is still running, so a partial
// result set can be labelled as such.
func (service *IndexService) IndexStatus() (proto.StatusResult, error) {
	return service.connection.Status()
}

// shutdown is unexported so Wails does not expose it to the frontend; only
// main's shutdown hook needs it.
func (service *IndexService) shutdown() error {
	return service.connection.Close()
}
