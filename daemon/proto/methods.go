package proto

// HandshakeParams is the first message on every connection.
type HandshakeParams struct {
	ClientName      string `json:"clientName"`
	ProtocolVersion int    `json:"protocolVersion"`
}

type HandshakeResult struct {
	DaemonVersion   string `json:"daemonVersion"`
	ProtocolVersion int    `json:"protocolVersion"`
	IndexPath       string `json:"indexPath"`
}

type AddRootParams struct {
	Path string `json:"path"`
}

type RemoveRootParams struct {
	Path string `json:"path"`
}

// Root is one indexed directory tree.
type Root struct {
	Path         string `json:"path"`
	FileCount    int64  `json:"fileCount"`
	LastScanUnix int64  `json:"lastScanUnix"`
}

type RootsResult struct {
	Roots []Root `json:"roots"`
}

// RescanParams with an empty Path rescans every root.
type RescanParams struct {
	Path string `json:"path,omitempty"`
}

const (
	StateIdle     = "idle"
	StateScanning = "scanning"
)

type StatusResult struct {
	State              string `json:"state"`
	FilesIndexed       int64  `json:"filesIndexed"`
	ScanningPath       string `json:"scanningPath"`
	WatchedDirectories int    `json:"watchedDirectories"`
	WatchLimitReached  bool   `json:"watchLimitReached"`
	LastScanUnix       int64  `json:"lastScanUnix"`
}

type SearchParams struct {
	Text            string `json:"text"`
	Limit           int    `json:"limit"`
	DirectoriesOnly bool   `json:"directoriesOnly"`
	// Scope limits results to a directory and everything under it. Empty
	// searches every indexed root.
	Scope string `json:"scope,omitempty"`
}

// Hit is one search result. Times are Unix seconds so the wire format stays
// free of timezone ambiguity.
type Hit struct {
	Path        string `json:"path"`
	Name        string `json:"name"`
	IsDir       bool   `json:"isDir"`
	Size        int64  `json:"size"`
	ModTimeUnix int64  `json:"modTimeUnix"`
}

type SearchResult struct {
	Hits []Hit `json:"hits"`
	// Truncated reports that the index held more matches than Limit allowed.
	Truncated bool `json:"truncated"`
}

type EmptyResult struct{}
