package main

import (
	"context"
	"embed"
	"os"

	"github.com/wailsapp/wails/v2"
	"github.com/wailsapp/wails/v2/pkg/options"
	"github.com/wailsapp/wails/v2/pkg/options/assetserver"
	"github.com/wailsapp/wails/v2/pkg/runtime"

	"canopy/internal/mediaserver"
	"canopy/internal/settings"
)

//go:embed all:frontend/dist
var assets embed.FS

func main() {
	fileService := NewFileService()
	fileService.media = mediaserver.Start()
	fileService.openAt(folderArgument())
	indexService := NewIndexService()
	window := fileService.LoadSettings().Window

	err := wails.Run(&options.App{
		Title:  "Canopy",
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

// folderArgument is what Canopy was asked to open, which is how a desktop starts
// a file manager on a folder. Anything else on the command line belongs to
// whatever launched it and is left alone.
func folderArgument() string {
	if len(os.Args) < 2 {
		return ""
	}
	return os.Args[1]
}

// restoreWindowPosition puts the window back where it was closed. A first launch
// has no position recorded and is left to the window manager.
func restoreWindowPosition(applicationContext context.Context, window settings.Window) {
	if window.X == 0 && window.Y == 0 {
		return
	}
	runtime.WindowSetPosition(applicationContext, window.X, window.Y)
}
