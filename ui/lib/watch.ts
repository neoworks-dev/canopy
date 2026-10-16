/**
 * Mirrors the event the Go watcher emits when a watched directory changes. Its
 * payload is that directory's path, and it travels over a Wails event rather
 * than the generated bindings.
 */
export const directoryChangedEvent = "watch:changed";
