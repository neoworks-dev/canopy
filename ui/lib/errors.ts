/** Turns whatever a failed command rejects with into a readable message. */
export function describeError(error: unknown): string {
	if (error instanceof Error) {
		return error.message;
	}
	return String(error);
}
