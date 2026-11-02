import type {
	AddAuthenticatorInput,
	AuthenticatorAccount,
	AuthenticatorCodeBatch,
} from '$lib/authenticator';
import type {
	AddCardEntryInput,
	CardEntry,
	CardEntryDetail,
	UpdateCardEntryInput,
} from '$lib/cards';
import type {
	AddVaultEntryInput,
	UpdateVaultEntryInput,
	VaultEntry,
	VaultEntryDetail,
} from '$lib/vault';

const arcadiaApiBase = '/v2/arcadia';

export const arcadiaSessionExpiredEvent = 'arcadia-session-expired';

export interface ArcadiaSession {
	user: {
		id: string;
		email: string;
	};
	absoluteExpiresAt: number;
}

interface VaultPassword {
	password: string;
}

interface CardNumberResponse {
	number: string;
}

interface CardCvcResponse {
	cvc: string;
}

interface ErrorResponse {
	error?: string;
}

export class ArcadiaApi {
	async getSession(): Promise<ArcadiaSession | null> {
		const response = await this.fetch('/session');
		if (response.status === 401) return null;
		return this.readJson<ArcadiaSession>(response);
	}

	getAuthenticatorAccounts(): Promise<AuthenticatorAccount[]> {
		return this.request('/authenticators');
	}

	getAuthenticatorCodeBatch(): Promise<AuthenticatorCodeBatch> {
		return this.request('/authenticator-codes');
	}

	async addAuthenticator(
		input: AddAuthenticatorInput,
	): Promise<AuthenticatorAccount> {
		const body = new FormData();
		body.set('mode', input.mode);
		if (input.iconImage) body.set('iconImage', input.iconImage);
		if (input.mode === 'qr') body.set('qr', input.qr);
		else {
			body.set('issuer', input.issuer);
			body.set('account', input.account);
			body.set('secret', input.secret);
		}
		return this.request('/authenticators', { method: 'POST', body });
	}

	async updateAuthenticatorOrder(ids: string[]): Promise<void> {
		await this.request('/authenticators/order', {
			method: 'PUT',
			body: JSON.stringify({ ids }),
		});
	}

	async deleteAuthenticator(id: string): Promise<void> {
		await this.request(`/authenticators/${encodeURIComponent(id)}`, {
			method: 'DELETE',
		});
	}

	getVaultEntries(): Promise<VaultEntry[]> {
		return this.request('/vault/entries');
	}

	getVaultEntry(id: string): Promise<VaultEntryDetail> {
		return this.request(`/vault/entries/${encodeURIComponent(id)}`);
	}

	async getVaultEntryPassword(id: string): Promise<string> {
		return (
			await this.request<VaultPassword>(
				`/vault/entries/${encodeURIComponent(id)}/password`,
			)
		).password;
	}

	addVaultEntry(input: AddVaultEntryInput): Promise<VaultEntry> {
		return this.request('/vault/entries', {
			method: 'POST',
			body: JSON.stringify(input),
		});
	}

	updateVaultEntry(
		id: string,
		input: UpdateVaultEntryInput,
	): Promise<VaultEntryDetail> {
		return this.request(`/vault/entries/${encodeURIComponent(id)}`, {
			method: 'PATCH',
			body: JSON.stringify(input),
		});
	}

	async updateVaultEntryOrder(ids: string[]): Promise<void> {
		await this.request('/vault/entries/order', {
			method: 'PUT',
			body: JSON.stringify({ ids }),
		});
	}

	async deleteVaultEntry(id: string): Promise<void> {
		await this.request(`/vault/entries/${encodeURIComponent(id)}`, {
			method: 'DELETE',
		});
	}

	getCardEntries(): Promise<CardEntry[]> {
		return this.request('/cards/entries');
	}

	getCardEntry(id: string): Promise<CardEntryDetail> {
		return this.request(`/cards/entries/${encodeURIComponent(id)}`);
	}

	async getCardEntryNumber(id: string): Promise<string> {
		return (
			await this.request<CardNumberResponse>(
				`/cards/entries/${encodeURIComponent(id)}/number`,
			)
		).number;
	}

	async getCardEntryCvc(id: string): Promise<string> {
		return (
			await this.request<CardCvcResponse>(
				`/cards/entries/${encodeURIComponent(id)}/cvc`,
			)
		).cvc;
	}

	addCardEntry(input: AddCardEntryInput): Promise<CardEntry> {
		return this.request('/cards/entries', {
			method: 'POST',
			body: JSON.stringify(input),
		});
	}

	updateCardEntry(
		id: string,
		input: UpdateCardEntryInput,
	): Promise<CardEntryDetail> {
		return this.request(`/cards/entries/${encodeURIComponent(id)}`, {
			method: 'PATCH',
			body: JSON.stringify(input),
		});
	}

	async updateCardEntryOrder(ids: string[]): Promise<void> {
		await this.request('/cards/entries/order', {
			method: 'PUT',
			body: JSON.stringify({ ids }),
		});
	}

	async deleteCardEntry(id: string): Promise<void> {
		await this.request(`/cards/entries/${encodeURIComponent(id)}`, {
			method: 'DELETE',
		});
	}

	private async request<T>(
		path: string,
		init: RequestInit = {},
		notifyUnauthorized = true,
	): Promise<T> {
		const response = await this.fetch(path, init);
		if (response.status === 401 && notifyUnauthorized)
			window.dispatchEvent(new Event(arcadiaSessionExpiredEvent));
		if (!response.ok) throw await ArcadiaApiError.fromResponse(response);
		if (response.status === 204) return undefined as T;
		return this.readJson<T>(response);
	}

	private fetch(path: string, init: RequestInit = {}): Promise<Response> {
		const headers = new Headers(init.headers);
		headers.set('Accept', 'application/json');
		if (typeof init.body === 'string')
			headers.set('Content-Type', 'application/json');
		return fetch(`${arcadiaApiBase}${path}`, {
			...init,
			headers,
			credentials: 'include',
		});
	}

	private async readJson<T>(response: Response): Promise<T> {
		if (!response.ok) throw await ArcadiaApiError.fromResponse(response);
		return response.json() as Promise<T>;
	}
}

export class ArcadiaApiError extends Error {
	constructor(
		message: string,
		readonly status: number,
	) {
		super(message);
		this.name = 'ArcadiaApiError';
	}

	static async fromResponse(response: Response): Promise<ArcadiaApiError> {
		let body: ErrorResponse = {};
		try {
			body = (await response.json()) as ErrorResponse;
		} catch {
			// The status text remains the safe error when an intermediary returned HTML.
		}
		return new ArcadiaApiError(
			body.error || response.statusText || 'Request failed',
			response.status,
		);
	}
}

export const arcadiaApi = new ArcadiaApi();
