package main

import (
	"os"

	"file-explorer-neoworks/internal/filesystem"
)

// FileService is the Wails-bound bridge between the frontend and the
// filesystem package. It holds no state; every call is independent.
type FileService struct{}

func NewFileService() *FileService {
	return &FileService{}
}

// ListDirectory reads the directory at path. An empty path means the home
// directory.
func (service *FileService) ListDirectory(path string) (filesystem.Listing, error) {
	return filesystem.ReadDirectory(path)
}

// HomeDirectory is the frontend's starting location.
func (service *FileService) HomeDirectory() (string, error) {
	return os.UserHomeDir()
}

// PreviewFile reads as much of one file as the preview pane can show.
func (service *FileService) PreviewFile(path string) (filesystem.Preview, error) {
	return filesystem.ReadPreview(path)
}

// Places lists the sidebar shortcuts to well-known directories.
func (service *FileService) Places() []filesystem.Place {
	return filesystem.Places()
}
