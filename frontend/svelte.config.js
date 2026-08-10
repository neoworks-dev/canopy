import { vitePreprocess } from "@sveltejs/vite-plugin-svelte";

// The shared UI package is authored in runes mode but ships as source from
// node_modules, where Svelte would otherwise infer legacy mode.
function shouldUseRunes({ filename }) {
	if (filename.includes("@neoworks-dev")) {
		return true;
	}
	if (filename.split(/[/\\]/).includes("node_modules")) {
		return undefined;
	}
	return true;
}

export default {
	preprocess: vitePreprocess(),
	compilerOptions: {
		runes: shouldUseRunes,
	},
};
