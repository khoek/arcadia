<script lang="ts">
	import { onDestroy } from 'svelte';
	import { PageHeader, Panel } from '@arcadia/ui';
	import { GripVertical, LockKeyhole, Pencil } from '@lucide/svelte';

	import AddVaultDialog from '$lib/components/AddVaultDialog.svelte';
	import CopyFeedback from '$lib/components/CopyFeedback.svelte';
	import ArcadiaToolbar from '$lib/components/ArcadiaToolbar.svelte';
	import EditVaultEntryDialog from '$lib/components/EditVaultEntryDialog.svelte';
	import EmptyState from '$lib/components/EmptyState.svelte';
	import ReorderableRecordList from '$lib/components/ReorderableRecordList.svelte';
	import RecordMark from '$lib/components/RecordMark.svelte';
	import { CopyFeedbackController } from '$lib/copy-feedback-controller.svelte';
	import { requestError } from '$lib/request-error';
	import { matchesSearchQuery } from '$lib/search';
	import {
		getArcadiaNotifications,
		getArcadiaServices,
	} from '$lib/services/context';
	import type {
		AddVaultEntryInput,
		UpdateVaultEntryInput,
		VaultCopyField,
		VaultEntry,
	} from '$lib/vault';

	const vault = getArcadiaServices().vault;
	const notifications = getArcadiaNotifications();
	const copyFeedback = new CopyFeedbackController<string>();
	let query = $state('');
	let editing = $state(false);
	let visibleEntries = $derived(
		vault.entries.filter((entry) =>
			matchesSearchQuery(`${entry.name} ${entry.username ?? ''}`, query),
		),
	);

	$effect(() => {
		if (vault.entries.length > 0 || !editing) return;
		editing = false;
		copyFeedback.reset();
	});

	onDestroy(() => {
		copyFeedback.destroy();
	});

	async function addEntry(input: AddVaultEntryInput): Promise<void> {
		await vault.add(input);
		notifications.success('Password added.');
	}

	async function updateEntry(
		id: string,
		input: UpdateVaultEntryInput,
	): Promise<void> {
		await vault.update(id, input);
		notifications.success('Password updated.');
	}

	function toggleEditing(): void {
		if (vault.entries.length === 0) return;
		editing = !editing;
		query = '';
		copyFeedback.reset();
	}

	function reorderEntries(reordered: VaultEntry[]): void {
		vault.reorder(reordered);
	}

	async function persistEntryOrder(items: VaultEntry[]): Promise<void> {
		try {
			await vault.persistOrder(items);
			notifications.success('Password order saved.');
		} catch (error) {
			notifications.error(requestError(error));
		}
	}

	async function deleteEntry(id: string): Promise<void> {
		const entry = vault.entries.find((item) => item.id === id);
		try {
			await vault.delete(id);
			if (entry) notifications.success(`${entry.name} deleted.`);
		} catch (error) {
			notifications.error(requestError(error));
		}
	}

	async function copy(entry: VaultEntry, field: VaultCopyField): Promise<void> {
		const sequence = copyFeedback.begin(copyTarget(entry, field));
		try {
			const value =
				field === 'username'
					? entry.username
					: await vault.getPassword(entry.id);
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

	function copyTarget(entry: VaultEntry, field: VaultCopyField): string {
		return entry.id + '\u0000' + field;
	}

	function fieldWasCopied(entry: VaultEntry, field: VaultCopyField): boolean {
		return copyFeedback.active === copyTarget(entry, field);
	}
</script>

<svelte:head>
	<title>Passwords · Arcadia</title>
	<meta name="description" content="Password vault arcadia." />
</svelte:head>

{#snippet vaultRow(entry: VaultEntry)}
	{#if editing}
		<div class="vault-row edit-row">
			<div class="entry-identity">
				<RecordMark
					name={entry.name}
					accent={entry.accent}
					iconImage={entry.iconImage}
				/>
				<strong class="entry-name">{entry.name}</strong>
			</div>
			<div class="vault-cell username-cell" data-label="Username">
				<span class:missing={!entry.username} class="edit-value">
					{entry.username ?? '—'}
				</span>
			</div>
			<div class="vault-cell password-cell" data-label="Password">
				<span
					class:missing={!entry.hasPassword}
					class="edit-value password-mask"
				>
					{entry.hasPassword ? '•••• … ••••' : '—'}
				</span>
			</div>
			<span class="drag-grip" aria-hidden="true">
				<GripVertical size={19} />
			</span>
		</div>
	{:else}
		<div class="vault-row">
			<div class="entry-identity">
				<RecordMark
					name={entry.name}
					accent={entry.accent}
					iconImage={entry.iconImage}
				/>
				<strong class="entry-name">{entry.name}</strong>
			</div>
			{#if entry.username}
				<button
					class="vault-cell copy-cell username-cell"
					data-label="Username"
					type="button"
					onclick={() => copy(entry, 'username')}
					aria-label={`Copy ${entry.name} username`}
					title="Copy username"
				>
					<div class="value-cluster">
						<span class="vault-value">{entry.username}</span>
						<CopyFeedback
							copied={fieldWasCopied(entry, 'username')}
							message={`${entry.name} username copied`}
							sequence={copyFeedback.sequence}
						/>
					</div>
				</button>
			{:else}
				<div class="vault-cell username-cell" data-label="Username">
					<span class="missing" aria-label="No username">—</span>
				</div>
			{/if}
			{#if entry.hasPassword}
				<button
					class="vault-cell copy-cell password-cell"
					data-label="Password"
					type="button"
					onclick={() => copy(entry, 'password')}
					aria-label={`Copy ${entry.name} password`}
					title="Copy password"
				>
					<div class="value-cluster">
						<span class="vault-value password-mask" aria-hidden="true"
							>•••• … ••••</span
						>
						<CopyFeedback
							copied={fieldWasCopied(entry, 'password')}
							message={`${entry.name} password copied`}
							sequence={copyFeedback.sequence}
						/>
					</div>
				</button>
			{:else}
				<div class="vault-cell password-cell" data-label="Password">
					<span class="missing" aria-label="No password">—</span>
				</div>
			{/if}
			<div class="entry-actions">
				<EditVaultEntryDialog
					{entry}
					onload={() => vault.getEntry(entry.id)}
					onsave={(input) => updateEntry(entry.id, input)}
				/>
			</div>
		</div>
	{/if}
{/snippet}

{#snippet vaultEmptyIcon()}
	<LockKeyhole size={26} />
{/snippet}

{#snippet emptyVaultList()}
	{#if query.trim()}
		<EmptyState
			title="No matching passwords"
			description="Try a different name or username."
			icon={vaultEmptyIcon}
		/>
	{:else}
		<EmptyState
			title="No passwords"
			description="To get started, click the + button."
			icon={vaultEmptyIcon}
		/>
	{/if}
{/snippet}

<PageHeader title="Passwords" />

<Panel class="vault-panel">
	<ArcadiaToolbar
		bind:query
		disabled={editing}
		placeholder="Search"
		searchLabel="Search passwords"
	>
		<AddVaultDialog onadd={addEntry} />
		<button
			class:pressed={editing}
			class="arcadia-control-button"
			type="button"
			onclick={toggleEditing}
			disabled={vault.entries.length === 0}
			aria-label={editing ? 'Finish editing' : 'Edit passwords'}
			aria-pressed={editing}
			title={editing ? 'Finish editing' : 'Edit'}
		>
			<Pencil size={15} />
		</button>
	</ArcadiaToolbar>

	<div class="column-headings" aria-hidden="true">
		<span>Entry</span>
		<span>Username</span>
		<span>Password</span>
		<span></span>
	</div>

	{#if vault.loading}
		<p class="loading-row">Loading…</p>
	{:else}
		<ReorderableRecordList
			items={visibleEntries}
			{editing}
			dndType="vault-entry"
			label="Password order"
			itemLabel={(entry) => entry.name}
			onreorder={reorderEntries}
			onfinalize={persistEntryOrder}
			ondelete={deleteEntry}
			row={vaultRow}
			empty={emptyVaultList}
		/>
	{/if}
</Panel>

<style>
	:global(.vault-panel) {
		--vault-grid-columns: minmax(0, 1.15fr) minmax(0, 1fr) minmax(8rem, 0.85fr)
			3rem;
		overflow: hidden;
		margin-top: 1.5rem;
	}

	.column-headings {
		display: grid;
		grid-template-columns: var(--vault-grid-columns);
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

	.vault-row {
		display: grid;
		width: 100%;
		min-height: 4.1rem;
		grid-template-columns: var(--vault-grid-columns);
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

	.vault-cell,
	.value-cluster,
	.vault-value {
		min-width: 0;
	}

	.vault-cell {
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

	.vault-value {
		overflow: hidden;
		color: var(--site-link-fg);
		font-size: 0.78rem;
		font-weight: 650;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.entry-actions {
		display: grid;
		place-items: center;
	}

	.password-mask {
		color: var(--site-heading-fg);
		font-family:
			ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
		font-size: 0.9rem;
		font-weight: 760;
		letter-spacing: 0.1em;
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

	.edit-value {
		display: block;
		overflow: hidden;
		color: var(--site-surface-fg);
		font-size: 0.78rem;
		font-weight: 620;
		text-overflow: ellipsis;
		white-space: nowrap;
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

	@media (max-width: 680px) {
		.column-headings {
			display: none;
		}

		.vault-row,
		.edit-row {
			grid-template-columns: minmax(0, 1fr) minmax(0, 1fr) 3rem;
		}

		.entry-identity {
			grid-column: 1 / 3;
			grid-row: 1;
		}

		.username-cell {
			grid-column: 1;
			grid-row: 2;
		}

		.password-cell {
			grid-column: 2;
			grid-row: 2;
		}

		.entry-actions {
			grid-column: 3;
			grid-row: 1 / span 2;
		}

		.vault-cell::before {
			display: block;
			margin-bottom: 0.16rem;
			color: var(--arcadia-muted);
			content: attr(data-label);
			font-size: 0.58rem;
			font-weight: 750;
			letter-spacing: 0.06em;
			text-transform: uppercase;
		}

		.vault-cell {
			align-items: flex-start;
			justify-content: center;
			flex-direction: column;
		}

		.edit-row .drag-grip {
			grid-column: 3;
			grid-row: 1 / span 2;
		}
	}
</style>
