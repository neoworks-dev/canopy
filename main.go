package main

import (
	"context"
	"embed"

	"github.com/wailsapp/wails/v2"
	"github.com/wailsapp/wails/v2/pkg/options"
	"github.com/wailsapp/wails/v2/pkg/options/assetserver"
)

//go:embed all:frontend/dist
var assets embed.FS

func main() {
	fileService := NewFileService()
	indexService := NewIndexService()

	err := wails.Run(&options.App{
		Title:  "Neoworks Files",
		Width:  1100,
		Height: 720,
		AssetServer: &assetserver.Options{
			Assets: assets,
		},
		BackgroundColour: &options.RGBA{R: 10, G: 10, B: 11, A: 1},
		Bind: []interface{}{
			fileService,
			indexService,
		},
		OnShutdown: func(_ context.Context) {
			indexService.shutdown()
		},
	})

	if err != nil {
		println("Error:", err.Error())
	}
}
