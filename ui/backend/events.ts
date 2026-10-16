import { listen } from "@neoworks/svelte-gpui";

/** Hears one event the Rust side emits; returns the function that stops listening. */
export function EventsOn<Payload>(name: string, callback: (payload: Payload) => void): () => void {
	return listen<Payload>(name, (payload) => callback(payload));
}
