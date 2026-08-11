# file-explorer-neoworks

A desktop file explorer built with [Wails](https://wails.io) (Go backend,
system webview) and a Svelte 5 frontend on the shared `@neoworks-dev/ui`
design system.

## Requirements

- Go 1.25+
- Bun
- `webkit2gtk-4.1` and `gtk3`
- `ffmpeg` for thumbnails, image previews and audio waveforms
- `gst-plugins-good` for playback in the details pane. WebKit plays media
  through GStreamer, and without this package there is no audio sink and no
  parser for mp3, m4a, wav or flac, which takes the web process down with it.

## Commands

```sh
bun run dev       # wails dev — live reload for Go and frontend
bun run build     # production binary at build/bin/neoworks-files
bun run bindings  # regenerate frontend/wailsjs from the bound Go methods
bun test          # frontend logic tests
bun run test:go   # backend tests
bun run check     # svelte-check
```

## Arch and the webkit build tag

Wails v2 links against `webkit2gtk-4.0` by default. Arch ships only
`webkit2gtk-4.1`, so every Wails command needs `-tags webkit2_41`. The scripts
above already pass it. `wails doctor` reports `libwebkit` as missing on this
setup — that check only looks for the 4.0 package name and can be ignored.

## Design system

`@neoworks-dev/ui` is consumed via `bun link`, not a registry. If it goes
missing:

```sh
cd frontend && bun link @neoworks-dev/ui
```

The package ships Svelte source rather than a build, so two things are wired up
for it:

- `frontend/svelte.config.js` forces runes mode for files under
  `@neoworks-dev`, which would otherwise be inferred as legacy mode inside
  `node_modules`.
- `frontend/src/app.css` adds an `@source` directive so Tailwind scans the
  package for utility classes instead of purging them.

## frontend/dist placeholder

`main.go` embeds `frontend/dist`, so the directory must exist before Go will
compile — including on a fresh clone that has never run a frontend build. A
tracked `frontend/dist/.gitkeep` covers that. Vite empties `dist` on every
build, so `frontend/public/.gitkeep` is copied back in to restore it and keep
the worktree clean.
