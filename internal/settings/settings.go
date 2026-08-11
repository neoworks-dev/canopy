// Package settings persists the preferences that would otherwise reset on every
// launch: view mode, columns, hidden files, sort order, window geometry and the
// directory the window was last showing.
package settings

import (
	"file-explorer-neoworks/internal/filesystem"
)

const currentVersion = 1

// Window geometry, in the coordinates the window manager reports.
type Window struct {
	Width  int `json:"width"`
	Height int `json:"height"`
	X      int `json:"x"`
	Y      int `json:"y"`
}

// Settings is the whole persisted file. Fields the frontend owns are left empty
// here, so their defaults live in one place rather than two: an empty view mode
// or column list means "whatever the frontend starts with".
type Settings struct {
	Version       int              `json:"version"`
	ViewMode      string           `json:"viewMode"`
	Columns       []string         `json:"columns"`
	ShowHidden    bool             `json:"showHidden"`
	Sort          filesystem.Order `json:"sort"`
	LastDirectory string           `json:"lastDirectory"`
	Window        Window           `json:"window"`
}

// Defaults are what a first launch, or an unreadable file, starts from.
func Defaults() Settings {
	return Settings{
		Version: currentVersion,
		Sort:    filesystem.DefaultOrder(),
		Window:  Window{Width: 1100, Height: 720},
	}
}
