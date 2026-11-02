export function matchesSearchQuery(value: string, query: string): boolean {
	return normalize(value).includes(normalize(query));
}

function normalize(value: string): string {
	return value
		.normalize('NFKD')
		.toLocaleLowerCase()
		.replace(/[^\p{Letter}\p{Number}]+/gu, '');
}
