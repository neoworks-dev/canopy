// Package config resolves where this application keeps its own files under the
// XDG config directory.
package config

import (
	"os"
	"path/filepath"
)

const applicationDirectory = "canopy"

// Path is where the named config file belongs, honouring XDG_CONFIG_HOME and
// falling back to ~/.config.
func Path(fileName string) (string, error) {
	configHome := os.Getenv("XDG_CONFIG_HOME")
	if configHome != "" {
		return filepath.Join(configHome, applicationDirectory, fileName), nil
	}
	homeDirectory, err := os.UserHomeDir()
	if err != nil {
		return "", err
	}
	return filepath.Join(homeDirectory, ".config", applicationDirectory, fileName), nil
}
