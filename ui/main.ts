// The library must be imported before anything that pulls in Svelte's client
// runtime: it installs the DOM globals that runtime reads at module scope.
import { start } from "@neoworks/svelte-gpui";
import App from "./App.svelte";

// The size comes from the settings file, which the Rust side applies before the window opens.
start(App, {}, { window: { title: "Canopy", minWidth: 720, minHeight: 480 } });
