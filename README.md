# Canopy

A keyboard-driven file explorer for Linux. The UI is written in Svelte 5 and
drawn natively by [svelte-gpui](https://github.com/neoworks-dev/svelte-gpui):
there is no browser engine, only GPUI rendering Svelte components. Everything
that touches the filesystem is Rust.

Grove edits the files; Canopy is how you look down over them.

## Requirements

- Rust (2024 edition) and Bun
- A checkout of svelte-gpui next to this one (`../svelte-gpui`), linked with
  `bun link`: run `bun link` inside svelte-gpui once, then `bun install` here
- Vulkan drivers, `libxkbcommon` and Wayland or X11, which GPUI draws with
- `ffmpeg` for video thumbnails, audio cover art and waveforms
- Optionally: `poppler` for PDF thumbnails, `git` for working tree marks,
  `zoxide` for jumping by frecency, `wl-clipboard` or `xclip` for copying files
  to other applications

## Commands

```sh
bun run dev        # build the UI, then run the app with hot reload
bun run build      # the UI bundle and the canopy and canopyd binaries
bun run package    # one self-contained executable in dist/package/canopy
bun test           # UI logic tests
bun run test:rust  # backend tests
bun run check      # svelte-check
```

`scripts/screenshot.sh` runs a bundle on a headless Hyprland output and saves
a screenshot, with its settings and index in a scratch directory, so trying
something out never touches your own files.

## How it fits together

```
ui/              The Svelte app svelte-gpui runs
  App.svelte     The window: toolbar, sidebar, panes, status strip, dialogs
  components/    One component per part of the window
    design/      The Neoworks design system, rebuilt on svelte-gpui primitives
  lib/           Logic with no UI in it: selection, sorting, keymap, tabs, …
                 Every module here has tests in tests/
  backend/       The functions the UI calls; each one invokes a Rust command
  theme.ts       Design tokens: colours, spacing, radii, type sizes
  icons.ts       Phosphor icons as SVG markup
app/             The executable: svelte-gpui's host plus Canopy's commands
  src/commands/  One module per area; command names match ui/backend/
crates/
  model/         Types every crate shares: Entry, Listing, Order
  files/         Listing, previews, thumbnails, archives, the operation
                 engine with undo, folder sizes and duplicate search
  desktop/       Bookmarks, clipboard, git, launching applications, recents,
                 saved searches, settings, tags, watching and zoxide
  index/         canopyd, the search index daemon, with its protocol and client
tests/           Tests for ui/lib
packaging/       Desktop entry, AppStream metainfo, icons and the PKGBUILD
```

A call from the UI goes `ui/backend/FileService.ts` → `invoke("ListDirectory",
…)` → `app/src/commands/listing.rs` → `canopy_files::filesystem`. Work that
takes a while (copying, measuring, searching for duplicates) answers with an
id at once and reports progress as events, which the UI hears through
`ui/backend/events.ts`.

Styling uses svelte-gpui's style objects rather than CSS. Values come from
`ui/theme.ts`, and the design system's components live in
`ui/components/design/`. Look there before writing a new one.

## Packaging

`packaging/` holds the desktop entry, the AppStream metainfo, the icons and a
`PKGBUILD`. To build and install the Arch package:

```sh
cd packaging && makepkg -si
```

The package installs `canopy` and the `canopyd` daemon beside it, and
registers `inode/directory` and the archive types Canopy can open.

## Where Canopy keeps things

- `~/.config/canopy`: settings, saved searches and recent folders
- `~/.local/share/neoworks`: the search index. It belongs to the daemon
  rather than to Canopy, because `canopyd` is shared by every Neoworks
  application, and the first one to launch wins the daemon lock.
- `~/.cache/thumbnails`: thumbnails, in the cache every Linux desktop shares

## License

MIT. See [LICENSE](LICENSE).
