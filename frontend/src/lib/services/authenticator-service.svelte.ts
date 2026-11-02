import { SvelteMap, SvelteSet } from 'svelte/reactivity';

import type {
	AddAuthenticatorInput,
	AuthenticatorAccount,
	AuthenticatorCodeBatch,
	AuthenticatorCodePeriod,
} from '$lib/authenticator';
import { AuthenticatorCodeManager } from '$lib/authenticator-code-manager';
import type { ArcadiaApi } from '$lib/arcadia';
import { requestError } from '$lib/request-error';
import { AsyncCommitQueue } from '$lib/services/async-commit-queue';
import { BackgroundRefresh } from '$lib/services/background-refresh';

const accountRefreshIntervalMs = 60_000;
const pendingAccent = '#246fca';

export class AuthenticatorService {
	accounts = $state<AuthenticatorAccount[]>([]);
	codeBatch = $state<AuthenticatorCodeBatch>();
	loading = $state(true);

	private accountError = $state('');
	private active = false;
	private readonly aliases = new SvelteMap<string, string>();
	private codeError = $state('');
	private readonly codeManager: AuthenticatorCodeManager;
	private readonly commits: AsyncCommitQueue;
	private readonly createdAccounts = new SvelteMap<
		string,
		AuthenticatorAccount
	>();
	private readonly failedCreates = new SvelteSet<string>();
	private generation = 0;
	private operationError = $state('');
	private readonly refreshAccounts: BackgroundRefresh;
	private revision = 0;

	constructor(private readonly api: ArcadiaApi) {
		this.codeManager = new AuthenticatorCodeManager(api);
		this.commits = new AsyncCommitQueue((error) => {
			if (this.active) this.operationError = requestError(error);
		});
		this.refreshAccounts = new BackgroundRefresh(
			() => this.loadAccounts(),
			accountRefreshIntervalMs,
		);
	}

	get error(): string {
		return this.operationError || this.accountError || this.codeError;
	}

	start(): void {
		if (this.active) return;
		this.active = true;
		this.generation += 1;
		this.loading = true;
		this.operationError = '';
		this.accountError = '';
		this.codeError = '';
		this.refreshAccounts.start();
		void this.startCodes(this.generation);
	}

	stop(): void {
		this.active = false;
		this.generation += 1;
		this.refreshAccounts.stop();
		this.codeManager.stop();
		this.commits.reset();
		this.aliases.clear();
		this.createdAccounts.clear();
		this.failedCreates.clear();
		this.accounts = [];
		this.codeBatch = undefined;
		this.loading = true;
		this.operationError = '';
		this.accountError = '';
		this.codeError = '';
	}

	async refresh(): Promise<void> {
		await Promise.all([this.refreshAccounts.refresh(), this.refreshCodes()]);
	}

	codesAt(clientTime: number): Map<string, AuthenticatorCodePeriod> {
		return this.codeManager.codesAt(clientTime);
	}

	progressAt(clientTime: number): number {
		return this.codeManager.progressAt(clientTime);
	}

	add(input: AddAuthenticatorInput): Promise<void> {
		const id = `pending-${crypto.randomUUID()}`;
		const generation = this.generation;
		this.revision += 1;
		this.accounts = [
			...this.accounts,
			{
				id,
				issuer: input.mode === 'manual' ? input.issuer : 'Importing…',
				account: input.mode === 'manual' ? input.account : '',
				accent: pendingAccent,
				...(input.iconImage ? { iconImage: input.iconImage } : {}),
				periodSeconds: 30,
			},
		];
		this.operationError = '';
		return this.commits.enqueue(async () => {
			if (!this.active || generation !== this.generation) return;
			try {
				const account = await this.api.addAuthenticator(input);
				this.aliases.set(id, account.id);
				this.createdAccounts.set(id, account);
				if (!this.active || generation !== this.generation) return;
				this.accounts = this.accounts.map((current) =>
					current.id === id ? account : current,
				);
				this.operationError = '';
				void this.refreshCodes();
			} catch (error) {
				this.failedCreates.add(id);
				if (this.active && generation === this.generation)
					this.accounts = this.accounts.filter((account) => account.id !== id);
				throw error;
			}
		});
	}

	reorder(accounts: AuthenticatorAccount[]): void {
		this.revision += 1;
		this.accounts = accounts;
	}

	persistOrder(accounts: AuthenticatorAccount[]): Promise<void> {
		const generation = this.generation;
		const ids = accounts.map(({ id }) => id);
		return this.commits.enqueue(async () => {
			if (!this.active || generation !== this.generation) return;
			await this.api.updateAuthenticatorOrder(this.persistedIds(ids));
			if (this.active && generation === this.generation)
				this.operationError = '';
		});
	}

	delete(id: string): Promise<void> {
		const previous = this.accounts;
		const generation = this.generation;
		const revision = ++this.revision;
		this.accounts = this.accounts.filter((account) => account.id !== id);
		return this.commits.enqueue(async () => {
			if (!this.active || generation !== this.generation) return;
			const persistedId = this.persistedId(id);
			if (!persistedId) return;
			try {
				await this.api.deleteAuthenticator(persistedId);
				if (!this.active || generation !== this.generation) return;
				this.operationError = '';
				void this.refreshCodes();
			} catch (error) {
				if (
					this.active &&
					generation === this.generation &&
					revision === this.revision
				)
					this.accounts = this.resolveAccounts(previous);
				throw error;
			}
		});
	}

	private async loadAccounts(force = false): Promise<void> {
		if (this.commits.busy && !force) return;
		const generation = this.generation;
		const revision = this.revision;
		try {
			const accounts = await this.api.getAuthenticatorAccounts();
			if (
				this.active &&
				generation === this.generation &&
				(force || revision === this.revision)
			) {
				this.accounts = accounts;
				this.accountError = '';
			}
		} catch (error) {
			if (this.active && generation === this.generation)
				this.accountError = requestError(error);
		} finally {
			if (this.active && generation === this.generation) this.loading = false;
		}
	}

	private async startCodes(generation: number): Promise<void> {
		try {
			await this.codeManager.start(
				(batch) => {
					if (!this.active || generation !== this.generation) return;
					this.codeBatch = batch;
					this.codeError = '';
				},
				(error) => {
					if (this.active && generation === this.generation)
						this.codeError = requestError(error);
				},
			);
		} catch (error) {
			if (this.active && generation === this.generation)
				this.codeError = requestError(error);
		}
	}

	private async refreshCodes(): Promise<void> {
		const generation = this.generation;
		try {
			await this.codeManager.refresh();
			if (this.active && generation === this.generation) this.codeError = '';
		} catch (error) {
			if (this.active && generation === this.generation)
				this.codeError = requestError(error);
		}
	}

	private persistedId(id: string): string | undefined {
		if (this.failedCreates.has(id)) return undefined;
		return this.aliases.get(id) ?? id;
	}

	private persistedIds(ids: string[]): string[] {
		return ids.flatMap((id) => {
			const persisted = this.persistedId(id);
			return persisted ? [persisted] : [];
		});
	}

	private resolveAccounts(
		accounts: AuthenticatorAccount[],
	): AuthenticatorAccount[] {
		return accounts.flatMap((account) => {
			if (this.failedCreates.has(account.id)) return [];
			return [
				this.createdAccounts.get(account.id) ??
					(this.aliases.has(account.id)
						? { ...account, id: this.aliases.get(account.id)! }
						: account),
			];
		});
	}
}
