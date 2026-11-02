export interface VaultEntry {
	id: string;
	name: string;
	username?: string;
	hasPassword: boolean;
	accent: string;
	iconImage?: string;
}

export interface VaultEntryDetail extends Omit<VaultEntry, 'hasPassword'> {
	password?: string;
	notes?: string;
}

export interface AddVaultEntryInput {
	name: string;
	username?: string;
	password?: string;
	iconImage?: string;
}

export interface UpdateVaultEntryInput {
	username: string | null;
	password: string | null;
	notes: string | null;
	iconImage: string | null;
}

export type VaultCopyField = 'username' | 'password';
