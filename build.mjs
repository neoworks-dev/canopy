import { fileURLToPath } from "node:url";
import { build } from "@neoworks/svelte-gpui/build";

const here = (path) => fileURLToPath(new URL(path, import.meta.url));

await build({
	entry: here("./ui/main.ts"),
	outfile: here("./dist/canopy.js"),
	watch: process.argv.includes("--watch"),
});
