import { SvelteMap, SvelteSet } from 'svelte/reactivity';

import {
	normalizeCardNumber,
	type AddCardEntryInput,
	type CardEntry,
	type CardEntryDetail,
	type UpdateCardEntryInput,
} from '$lib/cards';
import type { ArcadiaApi } from '$lib/arcadia';
import { requestError } from '$lib/request-error';
import { AsyncCommitQueue } from '$lib/services/async-commit-queue';
import { BackgroundRefresh } from '$lib/services/background-refresh';

const entryRefreshIntervalMs = 60_000;
const pendingAccent = '#246fca';

export class CardService {
	entries = $state<CardEntry[]>([]);
	loading = $state(true);

	private active = false;
	private readonly aliases = new SvelteMap<string, string>();
	private backgroundError = $state('');
	private readonly commits: AsyncCommitQueue;
	private readonly createdEntries = new SvelteMap<string, CardEntry>();
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

	add(input: AddCardEntryInput): Promise<void> {
		const id = `pending-${crypto.randomUUID()}`;
		const generation = this.generation;
		const number = normalizeCardNumber(input.number);
		this.revision += 1;
		this.entries = [
			...this.entries,
			{
				id,
				name: input.name,
				lastFour: number.slice(-4),
				hasNumber: Boolean(number),
				hasCvc: Boolean(input.cvc),
				expiry: input.expiry,
				accent: pendingAccent,
				...(input.iconImage ? { iconImage: input.iconImage } : {}),
			},
		];
		this.operationError = '';
		return this.commits.enqueue(async () => {
			if (!this.active || generation !== this.generation) return;
			try {
				const entry = await this.api.addCardEntry({
					...input,
					number,
				});
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

	update(id: string, input: UpdateCardEntryInput): Promise<void> {
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
				const detail = await this.api.updateCardEntry(persistedId, {
					...input,
					number: input.number
						? normalizeCardNumber(input.number)
						: input.number,
				});
				if (!this.active || generation !== this.generation) return;
				this.entries = this.entries.map((entry) =>
					entry.id === id || entry.id === persistedId
						? summary(detail, entry)
						: entry,
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

	reorder(entries: CardEntry[]): void {
		this.revision += 1;
		this.entries = entries;
	}

	persistOrder(entries: CardEntry[]): Promise<void> {
		const generation = this.generation;
		const ids = entries.map(({ id }) => id);
		return this.commits.enqueue(async () => {
			if (!this.active || generation !== this.generation) return;
			await this.api.updateCardEntryOrder(this.persistedIds(ids));
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
				await this.api.deleteCardEntry(persistedId);
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

	async getEntry(id: string): Promise<CardEntryDetail> {
		const persistedId = await this.resolveForRead(id);
		return this.api.getCardEntry(persistedId);
	}

	async getNumber(id: string): Promise<string> {
		const persistedId = await this.resolveForRead(id);
		return this.api.getCardEntryNumber(persistedId);
	}

	async getCvc(id: string): Promise<string> {
		const persistedId = await this.resolveForRead(id);
		return this.api.getCardEntryCvc(persistedId);
	}

	private async loadEntries(force = false): Promise<void> {
		if (this.commits.busy && !force) return;
		const generation = this.generation;
		const revision = this.revision;
		try {
			const entries = await this.api.getCardEntries();
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
		if (!persisted) throw new Error('This card could not be saved.');
		return persisted;
	}

	private resolveEntries(entries: CardEntry[]): CardEntry[] {
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
	entry: CardEntry,
	input: UpdateCardEntryInput,
): CardEntry {
	const number = input.number ? normalizeCardNumber(input.number) : '';
	return {
		...entry,
		lastFour: number ? number.slice(-4) : '',
		hasNumber: Boolean(number),
		hasCvc: Boolean(input.cvc),
		...(input.expiry ? { expiry: input.expiry } : { expiry: undefined }),
		...(input.iconImage
			? { iconImage: input.iconImage }
			: { iconImage: undefined }),
	};
}

function summary(detail: CardEntryDetail, previous?: CardEntry): CardEntry {
	const number = detail.number ? normalizeCardNumber(detail.number) : '';
	return {
		id: detail.id,
		name: detail.name,
		lastFour: number.slice(-4) || detail.lastFour || previous?.lastFour || '',
		hasNumber:
			Boolean(number) ||
			Boolean(detail.lastFour) ||
			Boolean(previous?.hasNumber),
		hasCvc:
			detail.cvc !== undefined
				? Boolean(detail.cvc)
				: Boolean(previous?.hasCvc),
		...(detail.expiry ? { expiry: detail.expiry } : {}),
		accent: detail.accent,
		...(detail.iconImage ? { iconImage: detail.iconImage } : {}),
	};
}
