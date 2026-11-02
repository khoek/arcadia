<script lang="ts">
	import { onDestroy } from 'svelte';
	import { CreditCard, GripVertical, Pencil } from '@lucide/svelte';
	import { PageHeader, Panel } from '@arcadia/ui';

	import AddCardDialog from '$lib/components/AddCardDialog.svelte';
	import CopyFeedback from '$lib/components/CopyFeedback.svelte';
	import ArcadiaToolbar from '$lib/components/ArcadiaToolbar.svelte';
	import EditCardDialog from '$lib/components/EditCardDialog.svelte';
	import EmptyState from '$lib/components/EmptyState.svelte';
	import ReorderableRecordList from '$lib/components/ReorderableRecordList.svelte';
	import RecordMark from '$lib/components/RecordMark.svelte';
	import type {
		AddCardEntryInput,
		CardCopyField,
		CardEntry,
		UpdateCardEntryInput,
	} from '$lib/cards';
	import { maskedCardNumber } from '$lib/cards';
	import { CopyFeedbackController } from '$lib/copy-feedback-controller.svelte';
	import { requestError } from '$lib/request-error';
	import { matchesSearchQuery } from '$lib/search';
	import {
		getArcadiaNotifications,
		getArcadiaServices,
	} from '$lib/services/context';

	const cards = getArcadiaServices().cards;
	const notifications = getArcadiaNotifications();
	const copyFeedback = new CopyFeedbackController<string>();
	let query = $state('');
	let editing = $state(false);
	let visibleEntries = $derived(
		cards.entries.filter((entry) =>
			matchesSearchQuery(
				`${entry.name} ${entry.lastFour} ${entry.expiry ?? ''}`,
				query,
			),
		),
	);

	$effect(() => {
		if (cards.entries.length > 0 || !editing) return;
		editing = false;
		copyFeedback.reset();
	});

	onDestroy(() => {
		copyFeedback.destroy();
	});

	async function addEntry(input: AddCardEntryInput): Promise<void> {
		await cards.add(input);
		notifications.success('Card added.');
	}

	async function updateEntry(
		id: string,
		input: UpdateCardEntryInput,
	): Promise<void> {
		await cards.update(id, input);
		notifications.success('Card updated.');
	}

	function toggleEditing(): void {
		if (cards.entries.length === 0) return;
		editing = !editing;
		query = '';
		copyFeedback.reset();
	}

	function reorderEntries(reordered: CardEntry[]): void {
		cards.reorder(reordered);
	}

	async function persistEntryOrder(items: CardEntry[]): Promise<void> {
		try {
			await cards.persistOrder(items);
			notifications.success('Card order saved.');
		} catch (error) {
			notifications.error(requestError(error));
		}
	}

	async function deleteEntry(id: string): Promise<void> {
		const entry = cards.entries.find((item) => item.id === id);
		try {
			await cards.delete(id);
			if (entry) notifications.success(`${entry.name} deleted.`);
		} catch (error) {
			notifications.error(requestError(error));
		}
	}

	async function copy(entry: CardEntry, field: CardCopyField): Promise<void> {
		const sequence = copyFeedback.begin(copyTarget(entry, field));
		try {
			const value =
				field === 'number'
					? await cards.getNumber(entry.id)
					: field === 'cvc'
						? await cards.getCvc(entry.id)
						: entry.expiry;
			if (!value) {
				copyFeedback.cancel(sequence);
				return;
			}
			if (!copyFeedback.isCurrent(sequence)) return;
			await navigator.clipboard.writeText(value);
			if (!copyFeedback.isCurrent(sequence)) return;
			copyFeedback.finish(sequence);
		} catch (error) {
			if (!copyFeedback.isCurrent(sequence)) return;
			copyFeedback.cancel(sequence);
			notifications.error(requestError(error));
		}
	}

	function copyTarget(entry: CardEntry, field: CardCopyField): string {
		return entry.id + '\u0000' + field;
	}

	function fieldWasCopied(entry: CardEntry, field: CardCopyField): boolean {
		return copyFeedback.active === copyTarget(entry, field);
	}
</script>

<svelte:head>
	<title>Cards · Arcadia</title>
	<meta name="description" content="Credit card arcadia." />
</svelte:head>

{#snippet cardRow(entry: CardEntry)}
	{#if editing}
		<div class="card-row edit-row">
			<div class="entry-identity">
				<RecordMark
					name={entry.name}
					accent={entry.accent}
					iconImage={entry.iconImage}
				/>
				<strong class="entry-name">{entry.name}</strong>
			</div>
			<div class="card-cell number-cell" data-label="Number">
				<span class:missing={!entry.hasNumber} class="card-value number-mask">
					{entry.hasNumber ? maskedCardNumber(entry.lastFour) : '—'}
				</span>
			</div>
			<div class="card-cell expiry-cell" data-label="Expiry">
				<span class:missing={!entry.expiry} class="card-value">
					{entry.expiry ?? '—'}
				</span>
			</div>
			<div class="card-cell cvc-cell" data-label="CVC">
				<span class:missing={!entry.hasCvc} class="card-value number-mask">
					{entry.hasCvc ? '•••' : '—'}
				</span>
			</div>
			<span class="drag-grip" aria-hidden="true">
				<GripVertical size={19} />
			</span>
		</div>
	{:else}
		<div class="card-row">
			<div class="entry-identity">
				<RecordMark
					name={entry.name}
					accent={entry.accent}
					iconImage={entry.iconImage}
				/>
				<strong class="entry-name">{entry.name}</strong>
			</div>
			{#if entry.hasNumber}
				<button
					class="card-cell copy-cell number-cell"
					data-label="Number"
					type="button"
					onclick={() => copy(entry, 'number')}
					aria-label={`Copy ${entry.name} card number`}
					title="Copy card number"
				>
					<div class="value-cluster">
						<span class="card-value number-mask" aria-hidden="true"
							>{maskedCardNumber(entry.lastFour)}</span
						>
						<CopyFeedback
							copied={fieldWasCopied(entry, 'number')}
							message={`${entry.name} card number copied`}
							sequence={copyFeedback.sequence}
						/>
					</div>
				</button>
			{:else}
				<div class="card-cell number-cell" data-label="Number">
					<span class="missing" aria-label="No card number">—</span>
				</div>
			{/if}
			{#if entry.expiry}
				<button
					class="card-cell copy-cell expiry-cell"
					data-label="Expiry"
					type="button"
					onclick={() => copy(entry, 'expiry')}
					aria-label={`Copy ${entry.name} expiry`}
					title="Copy expiry"
				>
					<div class="value-cluster">
						<span class="card-value">{entry.expiry}</span>
						<CopyFeedback
							copied={fieldWasCopied(entry, 'expiry')}
							message={`${entry.name} expiry copied`}
							sequence={copyFeedback.sequence}
						/>
					</div>
				</button>
			{:else}
				<div class="card-cell expiry-cell" data-label="Expiry">
					<span class="missing" aria-label="No expiry">—</span>
				</div>
			{/if}
			{#if entry.hasCvc}
				<button
					class="card-cell copy-cell cvc-cell"
					data-label="CVC"
					type="button"
					onclick={() => copy(entry, 'cvc')}
					aria-label={`Copy ${entry.name} CVC`}
					title="Copy CVC"
				>
					<div class="value-cluster">
						<span class="card-value number-mask" aria-hidden="true">•••</span>
						<CopyFeedback
							copied={fieldWasCopied(entry, 'cvc')}
							message={`${entry.name} CVC copied`}
							sequence={copyFeedback.sequence}
						/>
					</div>
				</button>
			{:else}
				<div class="card-cell cvc-cell" data-label="CVC">
					<span class="missing" aria-label="No CVC">—</span>
				</div>
			{/if}
			<div class="entry-actions">
				<EditCardDialog
					{entry}
					onload={() => cards.getEntry(entry.id)}
					onsave={(input) => updateEntry(entry.id, input)}
				/>
			</div>
		</div>
	{/if}
{/snippet}

{#snippet cardsEmptyIcon()}
	<CreditCard size={26} />
{/snippet}

{#snippet emptyCardsList()}
	{#if query.trim()}
		<EmptyState
			title="No matching cards"
			description="Try a different name or last four digits."
			icon={cardsEmptyIcon}
		/>
	{:else}
		<EmptyState
			title="No cards"
			description="To get started, click the + button."
			icon={cardsEmptyIcon}
		/>
	{/if}
{/snippet}

<PageHeader title="Cards" />

<Panel class="cards-panel">
	<ArcadiaToolbar
		bind:query
		disabled={editing}
		placeholder="Search"
		searchLabel="Search cards"
	>
		<AddCardDialog onadd={addEntry} />
		<button
			class:pressed={editing}
			class="arcadia-control-button"
			type="button"
			onclick={toggleEditing}
			disabled={cards.entries.length === 0}
			aria-label={editing ? 'Finish editing' : 'Edit cards'}
			aria-pressed={editing}
			title={editing ? 'Finish editing' : 'Edit'}
		>
			<Pencil size={15} />
		</button>
	</ArcadiaToolbar>

	<div class="column-headings" aria-hidden="true">
		<span>Entry</span>
		<span>Number</span>
		<span>Expiry</span>
		<span>CVC</span>
		<span></span>
	</div>

	{#if cards.loading}
		<p class="loading-row">Loading…</p>
	{:else}
		<ReorderableRecordList
			items={visibleEntries}
			{editing}
			dndType="card-entry"
			label="Card order"
			itemLabel={(entry) => entry.name}
			onreorder={reorderEntries}
			onfinalize={persistEntryOrder}
			ondelete={deleteEntry}
			row={cardRow}
			empty={emptyCardsList}
		/>
	{/if}
</Panel>

<style>
	:global(.cards-panel) {
		--cards-grid-columns: minmax(0, 1.25fr) minmax(10rem, 1.15fr)
			minmax(5rem, 0.62fr) minmax(4.5rem, 0.55fr) 3rem;
		overflow: hidden;
		margin-top: 1.5rem;
	}

	.column-headings {
		display: grid;
		grid-template-columns: var(--cards-grid-columns);
		border-block: 1px solid var(--arcadia-line);
		background: rgb(36 111 202 / 0.035);
		color: var(--arcadia-muted);
		font-size: 0.62rem;
		font-weight: 750;
		letter-spacing: 0.09em;
		text-transform: uppercase;
	}

	.column-headings span {
		padding: 0.45rem 0.8rem;
	}

	.loading-row {
		margin: 0;
		padding: 1rem;
		color: var(--arcadia-muted);
		font-size: 0.78rem;
	}

	.card-row {
		display: grid;
		width: 100%;
		min-height: 4.1rem;
		grid-template-columns: var(--cards-grid-columns);
		align-items: stretch;
		box-sizing: border-box;
	}

	.entry-identity {
		display: flex;
		min-width: 0;
		align-items: center;
		gap: 0.75rem;
		padding: 0.7rem 0.8rem;
	}

	.entry-name {
		min-width: 0;
		overflow: hidden;
		color: var(--site-heading-fg);
		font-size: 1rem;
		font-weight: 740;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.card-cell,
	.value-cluster,
	.card-value {
		min-width: 0;
	}

	.card-cell {
		display: flex;
		align-items: center;
		box-sizing: border-box;
		border: 0;
		background: transparent;
		color: inherit;
		padding: 0.7rem 0.8rem;
		text-align: left;
	}

	.value-cluster {
		display: flex;
		width: 100%;
		align-items: center;
		gap: 0.45rem;
	}

	.copy-cell {
		cursor: copy;
		transition:
			background 120ms,
			box-shadow 120ms;
	}

	.copy-cell:hover {
		background: rgb(36 111 202 / 0.045);
		box-shadow: inset 0 0 0 1px rgb(36 111 202 / 0.035);
	}

	.copy-cell:active {
		background: rgb(36 111 202 / 0.09);
	}

	.card-value {
		overflow: hidden;
		color: var(--site-link-fg);
		font-size: 0.78rem;
		font-weight: 650;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.number-mask {
		color: var(--site-heading-fg);
		font-family:
			ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
		font-size: 0.78rem;
		font-variant-numeric: tabular-nums;
		font-weight: 760;
		letter-spacing: 0.07em;
	}

	.entry-actions {
		display: grid;
		place-items: center;
	}

	.missing {
		color: #a8b3c2;
		font-size: 0.85rem;
	}

	.edit-row {
		cursor: grab;
		user-select: none;
	}

	.edit-row:active {
		cursor: grabbing;
	}

	.drag-grip {
		display: grid;
		place-items: center;
		color: var(--arcadia-muted);
		transition:
			background 120ms,
			color 120ms;
	}

	.edit-row:hover .drag-grip {
		background: rgb(36 111 202 / 0.07);
		color: var(--site-link-fg);
	}

	@media (max-width: 760px) {
		:global(.cards-panel) {
			--cards-grid-columns: minmax(0, 1.15fr) minmax(0, 1fr) 3rem;
		}

		.column-headings {
			display: none;
		}

		.card-row,
		.edit-row {
			grid-template-columns: var(--cards-grid-columns);
		}

		.entry-identity {
			grid-column: 1 / 3;
			grid-row: 1;
		}

		.number-cell {
			grid-column: 1 / 3;
			grid-row: 2;
		}

		.expiry-cell {
			grid-column: 1;
			grid-row: 3;
		}

		.cvc-cell {
			grid-column: 2;
			grid-row: 3;
		}

		.entry-actions,
		.edit-row .drag-grip {
			grid-column: 3;
			grid-row: 1 / span 3;
		}

		.card-cell::before {
			display: block;
			margin-bottom: 0.16rem;
			color: var(--arcadia-muted);
			content: attr(data-label);
			font-size: 0.58rem;
			font-weight: 750;
			letter-spacing: 0.06em;
			text-transform: uppercase;
		}

		.card-cell {
			align-items: flex-start;
			justify-content: center;
			flex-direction: column;
		}
	}
</style>
