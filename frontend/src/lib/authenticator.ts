export interface AuthenticatorAccount {
	id: string;
	issuer: string;
	account: string;
	accent: string;
	iconImage?: string;
	periodSeconds: number;
}

export interface AuthenticatorCodePeriod {
	code: string;
	validFrom: number;
	validUntil: number;
}

export interface AuthenticatorCodePair {
	authenticatorId: string;
	current: AuthenticatorCodePeriod;
	next: AuthenticatorCodePeriod;
}

export interface AuthenticatorCodeBatch {
	codes: AuthenticatorCodePair[];
	serverTime: number;
}

interface AddAuthenticatorBase {
	iconImage?: string;
}

export type AddAuthenticatorInput = AddAuthenticatorBase &
	(
		| { mode: 'qr'; qr: File }
		| { mode: 'manual'; issuer: string; account: string; secret: string }
	);
