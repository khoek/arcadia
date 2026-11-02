export function requestError(error: unknown): string {
	return error instanceof Error ? error.message : 'Request failed';
}
