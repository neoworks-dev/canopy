package main

import (
	"file-explorer-neoworks/internal/searchindex"

	"github.com/neoworks-dev/neoworksd/proto"
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
// yields no hits rather than the whole index.
func (service *IndexService) Search(query string) (proto.SearchResult, error) {
	return service.connection.Search(query)
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
