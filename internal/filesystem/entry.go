package filesystem

// Entry describes a single item inside a directory. JSON tags drive the
// TypeScript bindings Wails generates for the frontend.
type Entry struct {
	Name    string `json:"name"`
	Path    string `json:"path"`
	IsDir   bool   `json:"isDir"`
	Size    int64  `json:"size"`
	ModTime string `json:"modTime"`
	Mode    string `json:"mode"`
}

// Listing is the result of reading one directory.
type Listing struct {
	Path       string  `json:"path"`
	ParentPath string  `json:"parentPath"`
	Entries    []Entry `json:"entries"`
}
