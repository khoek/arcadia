export interface CardEntry {
	id: string;
	name: string;
	lastFour: string;
	hasNumber: boolean;
	hasCvc: boolean;
	expiry?: string;
	accent: string;
	iconImage?: string;
}

export interface CardEntryDetail extends Omit<
	CardEntry,
	'hasNumber' | 'hasCvc'
> {
	number?: string;
	cvc?: string;
	cardholder?: string;
	notes?: string;
}

export interface AddCardEntryInput {
	name: string;
	number: string;
	expiry: string;
	cvc: string;
	cardholder?: string;
	notes?: string;
	iconImage?: string;
}

export interface UpdateCardEntryInput {
	number: string | null;
	expiry: string | null;
	cvc: string | null;
	cardholder: string | null;
	notes: string | null;
	iconImage: string | null;
}

export type CardCopyField = 'number' | 'expiry' | 'cvc';

export function normalizeCardNumber(value: string): string {
	return value.replace(/[\s-]/g, '');
}

export function maskedCardNumber(lastFour: string): string {
	return `•••• •••• •••• ${lastFour || '••••'}`;
}

export function validCardNumber(value: string): boolean {
	const digits = normalizeCardNumber(value);
	return /^\d{12,19}$/.test(digits);
}

export function validCardExpiry(value: string): boolean {
	const match = /^(0[1-9]|1[0-2])\/(\d{2}|\d{4})$/.exec(value);
	return match !== null;
}

export function validCardCvc(value: string): boolean {
	return /^\d{3,4}$/.test(value);
}
