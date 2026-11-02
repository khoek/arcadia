import { SvelteMap, SvelteSet } from 'svelte/reactivity';

import type { ArcadiaApi } from '$lib/arcadia';
import { requestError } from '$lib/request-error';
import { AsyncCommitQueue } from '$lib/services/async-commit-queue';
import { BackgroundRefresh } from '$lib/services/background-refresh';
import type {
	AddVaultEntryInput,
	UpdateVaultEntryInput,
	VaultEntry,
	VaultEntryDetail,
} from '$lib/vault';

const entryRefreshIntervalMs = 60_000;
const pendingAccent = '#246fca';

export class VaultService {
	entries = $state<VaultEntry[]>([]);
	loading = $state(true);

	private active = false;
	private readonly aliases = new SvelteMap<string, string>();
	private backgroundError = $state('');
	private readonly commits: AsyncCommitQueue;
	private readonly createdEntries = new SvelteMap<string, VaultEntry>();
	private readonly failedCreates = new SvelteSet<string>();
	private readonly refreshEntries: BackgroundRefresh;
	private generation = 0;
	private operationError = $state('');
	private revision = 0;

	constructor(private readonly api: ArcadiaApi) {
		this.commits = new AsyncCommitQueue((error) => {
			if (this.active) this.operationError = requestError(error);
		});
		this.refreshEntries = new BackgroundRefresh(
			() => this.loadEntries(),
			entryRefreshIntervalMs,
		);
	}

	get error(): string {
		return this.operationError || this.backgroundError;
	}

	start(): void {
		if (this.active) return;
		this.active = true;
		this.generation += 1;
		this.loading = true;
		this.operationError = '';
		this.backgroundError = '';
		this.refreshEntries.start();
	}

	stop(): void {
		this.active = false;
		this.generation += 1;
		this.refreshEntries.stop();
		this.commits.reset();
		this.aliases.clear();
		this.createdEntries.clear();
		this.failedCreates.clear();
		this.entries = [];
		this.loading = true;
		this.operationError = '';
		this.backgroundError = '';
	}

	refresh(): Promise<void> {
		return this.refreshEntries.refresh();
	}

	add(input: AddVaultEntryInput): Promise<void> {
		const id = `pending-${crypto.randomUUID()}`;
		const generation = this.generation;
		this.revision += 1;
		this.entries = [
			...this.entries,
			{
				id,
				name: input.name,
				...(input.username ? { username: input.username } : {}),
				hasPassword: Boolean(input.password),
				accent: pendingAccent,
				...(input.iconImage ? { iconImage: input.iconImage } : {}),
			},
		];
		this.operationError = '';
		return this.commits.enqueue(async () => {
			if (!this.active || generation !== this.generation) return;
			try {
				const entry = await this.api.addVaultEntry(input);
				this.aliases.set(id, entry.id);
				this.createdEntries.set(id, entry);
				if (!this.active || generation !== this.generation) return;
				this.entries = this.entries.map((current) =>
					current.id === id ? entry : current,
				);
				this.operationError = '';
			} catch (error) {
				this.failedCreates.add(id);
				if (this.active && generation === this.generation)
					this.entries = this.entries.filter((entry) => entry.id !== id);
				throw error;
			}
		});
	}

	update(id: string, input: UpdateVaultEntryInput): Promise<void> {
		const previous = this.entries;
		const generation = this.generation;
		const revision = ++this.revision;
		this.entries = this.entries.map((entry) =>
			entry.id === id ? optimisticUpdate(entry, input) : entry,
		);
		this.operationError = '';
		return this.commits.enqueue(async () => {
			if (!this.active || generation !== this.generation) return;
			const persistedId = this.persistedId(id);
			if (!persistedId) return;
			try {
				const detail = await this.api.updateVaultEntry(persistedId, input);
				if (!this.active || generation !== this.generation) return;
				this.entries = this.entries.map((entry) =>
					entry.id === id || entry.id === persistedId ? summary(detail) : entry,
				);
				this.operationError = '';
			} catch (error) {
				if (
					this.active &&
					generation === this.generation &&
					revision === this.revision
				)
					this.entries = this.resolveEntries(previous);
				throw error;
			}
		});
	}

	reorder(entries: VaultEntry[]): void {
		this.revision += 1;
		this.entries = entries;
	}

	persistOrder(entries: VaultEntry[]): Promise<void> {
		const generation = this.generation;
		const ids = entries.map(({ id }) => id);
		return this.commits.enqueue(async () => {
			if (!this.active || generation !== this.generation) return;
			await this.api.updateVaultEntryOrder(this.persistedIds(ids));
			if (this.active && generation === this.generation)
				this.operationError = '';
		});
	}

	delete(id: string): Promise<void> {
		const previous = this.entries;
		const generation = this.generation;
		const revision = ++this.revision;
		this.entries = this.entries.filter((entry) => entry.id !== id);
		return this.commits.enqueue(async () => {
			if (!this.active || generation !== this.generation) return;
			const persistedId = this.persistedId(id);
			if (!persistedId) return;
			try {
				await this.api.deleteVaultEntry(persistedId);
				if (this.active && generation === this.generation)
					this.operationError = '';
			} catch (error) {
				if (
					this.active &&
					generation === this.generation &&
					revision === this.revision
				)
					this.entries = this.resolveEntries(previous);
				throw error;
			}
		});
	}

	async getEntry(id: string): Promise<VaultEntryDetail> {
		const persistedId = await this.resolveForRead(id);
		return this.api.getVaultEntry(persistedId);
	}

	async getPassword(id: string): Promise<string> {
		const persistedId = await this.resolveForRead(id);
		return this.api.getVaultEntryPassword(persistedId);
	}

	private async loadEntries(force = false): Promise<void> {
		if (this.commits.busy && !force) return;
		const generation = this.generation;
		const revision = this.revision;
		try {
			const entries = await this.api.getVaultEntries();
			if (
				this.active &&
				generation === this.generation &&
				(force || revision === this.revision)
			) {
				this.entries = entries;
				this.backgroundError = '';
			}
		} catch (error) {
			if (this.active && generation === this.generation)
				this.backgroundError = requestError(error);
		} finally {
			if (this.active && generation === this.generation) this.loading = false;
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

	private async resolveForRead(id: string): Promise<string> {
		if (id.startsWith('pending-')) await this.commits.whenIdle();
		const persisted = this.persistedId(id);
		if (!persisted) throw new Error('This entry could not be saved.');
		return persisted;
	}

	private resolveEntries(entries: VaultEntry[]): VaultEntry[] {
		return entries.flatMap((entry) => {
			if (this.failedCreates.has(entry.id)) return [];
			return [
				this.createdEntries.get(entry.id) ??
					(this.aliases.has(entry.id)
						? { ...entry, id: this.aliases.get(entry.id)! }
						: entry),
			];
		});
	}
}

function optimisticUpdate(
	entry: VaultEntry,
	input: UpdateVaultEntryInput,
): VaultEntry {
	return {
		...entry,
		...(input.username
			? { username: input.username }
			: { username: undefined }),
		hasPassword: Boolean(input.password),
		...(input.iconImage
			? { iconImage: input.iconImage }
			: { iconImage: undefined }),
	};
}

function summary(detail: VaultEntryDetail): VaultEntry {
	return {
		id: detail.id,
		name: detail.name,
		...(detail.username ? { username: detail.username } : {}),
		hasPassword: Boolean(detail.password),
		accent: detail.accent,
		...(detail.iconImage ? { iconImage: detail.iconImage } : {}),
	};
}
