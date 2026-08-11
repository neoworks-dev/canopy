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

// Panes is the width in pixels of the panes flanking the listing. Zero means
// the frontend's own default, so a file written before these existed still
// opens on a sensible layout.
type Panes struct {
	Sidebar int `json:"sidebar"`
	Details int `json:"details"`
}

// Keybinding is one entry in the user's keymap, appended after the built-in
// bindings so it overrides them. What the fields mean is the frontend's
// business; the file only has to carry them across a launch.
type Keybinding struct {
	// A chord, or a sequence of them separated by spaces: "ctrl+c", "g g".
	Key string `json:"key"`
	// Empty unbinds whatever the built-in keymap put on this key.
	Command string `json:"command"`
	When    string `json:"when,omitempty"`
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
	Panes         Panes            `json:"panes"`
	// The sidebar groups the user has folded shut, by the frontend's own names.
	CollapsedSections []string `json:"collapsedSections,omitempty"`
	// Whether the details pane is on screen at all, which does not depend on
	// there being anything selected to describe.
	ShowDetails bool `json:"showDetails"`
	// Written by hand only. The store restores whatever was on disk before every
	// write, so the application cannot overwrite them.
	Keybindings []Keybinding `json:"keybindings,omitempty"`
}

// Defaults are what a first launch, or an unreadable file, starts from.
func Defaults() Settings {
	return Settings{
		Version: currentVersion,
		Sort:    filesystem.DefaultOrder(),
		Window:  Window{Width: 1100, Height: 720},
	}
}
