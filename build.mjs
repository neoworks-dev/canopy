// Bundles the UI for the svelte-gpui host. `--watch` rebuilds on every save.
// `--entry` and `--out` build another entry point, such as a scratch preview
// under ui/dev/, into its own bundle.
import { fileURLToPath } from "node:url";
import { build } from "@neoworks/svelte-gpui/build";

const here = (path) => fileURLToPath(new URL(path, import.meta.url));

function option(name, fallback) {
	const index = process.argv.indexOf(name);
	if (index === -1) {
		return fallback;
	}
	return process.argv[index + 1];
}

await build({
	entry: here(option("--entry", "./ui/main.ts")),
	outfile: here(option("--out", "./dist/canopy.js")),
	watch: process.argv.includes("--watch"),
});
