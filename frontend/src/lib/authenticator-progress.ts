export const authenticatorProgressEndpointGraceMs = 300;

export function visualAuthenticatorProgress(
	progress: number,
	periodMs: number,
): number {
	if (periodMs <= authenticatorProgressEndpointGraceMs * 2)
		throw new RangeError(
			'Authenticator period is too short for endpoint grace.',
		);
	const grace = authenticatorProgressEndpointGraceMs / periodMs;
	if (progress <= grace) return 0;
	if (progress >= 1 - grace) return 1;
	return (progress - grace) / (1 - grace * 2);
}
