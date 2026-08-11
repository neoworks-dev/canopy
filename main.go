package main

import (
	"context"
	"embed"

	"github.com/wailsapp/wails/v2"
	"github.com/wailsapp/wails/v2/pkg/options"
	"github.com/wailsapp/wails/v2/pkg/options/assetserver"
	"github.com/wailsapp/wails/v2/pkg/runtime"

	"file-explorer-neoworks/internal/mediaserver"
	"file-explorer-neoworks/internal/settings"
)

//go:embed all:frontend/dist
var assets embed.FS

func main() {
	fileService := NewFileService()
	fileService.media = mediaserver.Start()
	indexService := NewIndexService()
	window := fileService.LoadSettings().Window

	err := wails.Run(&options.App{
		Title:  "Neoworks Files",
		Width:  window.Width,
		Height: window.Height,
		AssetServer: &assetserver.Options{
			Assets: assets,
		},
		BackgroundColour: &options.RGBA{R: 10, G: 10, B: 11, A: 1},
		Bind: []interface{}{
			fileService,
			indexService,
		},
		OnStartup: func(applicationContext context.Context) {
			fileService.startup(applicationContext)
			restoreWindowPosition(applicationContext, window)
		},
		OnShutdown: func(applicationContext context.Context) {
			fileService.captureWindow(applicationContext)
			fileService.StopWatching()
			indexService.shutdown()
		},
	})

	if err != nil {
		println("Error:", err.Error())
	}
}

// restoreWindowPosition puts the window back where it was closed. A first launch
// has no position recorded and is left to the window manager.
func restoreWindowPosition(applicationContext context.Context, window settings.Window) {
	if window.X == 0 && window.Y == 0 {
		return
	}
	runtime.WindowSetPosition(applicationContext, window.X, window.Y)
}
