<script lang="ts">
	import { onMount, tick } from 'svelte';
	import { PageHeader, Panel } from '@arcadia/ui';
	import {
		GripVertical,
		Pause,
		Pencil,
		Play,
		ShieldCheck,
	} from '@lucide/svelte';

	import AddAuthenticatorDialog from '$lib/components/AddAuthenticatorDialog.svelte';
	import CopyFeedback from '$lib/components/CopyFeedback.svelte';
	import ArcadiaToolbar from '$lib/components/ArcadiaToolbar.svelte';
	import EmptyState from '$lib/components/EmptyState.svelte';
	import ReorderableRecordList from '$lib/components/ReorderableRecordList.svelte';
	import RecordMark from '$lib/components/RecordMark.svelte';
	import type {
		AddAuthenticatorInput,
		AuthenticatorAccount,
		AuthenticatorCodePeriod,
	} from '$lib/authenticator';
	import { visualAuthenticatorProgress } from '$lib/authenticator-progress';
	import { CopyFeedbackController } from '$lib/copy-feedback-controller.svelte';
	import { requestError } from '$lib/request-error';
	import { matchesSearchQuery } from '$lib/search';
	import {
		getArcadiaNotifications,
		getArcadiaServices,
	} from '$lib/services/context';

	const authenticator = getArcadiaServices().authenticator;
	const notifications = getArcadiaNotifications();
	const copyFeedback = new CopyFeedbackController<string>();
	let clientNow = $state(0);
	let query = $state('');
	let editing = $state(false);
	let paused = $state(false);
	let snappingProgress = $state(false);
	let frozenCodes = $state<Map<string, AuthenticatorCodePeriod> | null>(null);
	let frozenProgress = $state(0);
	let rotationFill = $state<HTMLDivElement>();
	let liveCodes = $derived(
		authenticator.codeBatch
			? authenticator.codesAt(clientNow)
			: new Map<string, AuthenticatorCodePeriod>(),
	);
	let liveProgress = $derived(
		authenticator.codeBatch ? authenticator.progressAt(clientNow) : 0,
	);
	let displayedCodes = $derived(
		paused && frozenCodes ? frozenCodes : liveCodes,
	);
	let visualLiveProgress = $derived(
		authenticator.accounts[0]
			? visualAuthenticatorProgress(
					liveProgress,
					authenticator.accounts[0].periodSeconds * 1_000,
				)
			: 0,
	);
	let displayedProgress = $derived(
		paused ? frozenProgress : visualLiveProgress,
	);
	let visibleAccounts = $derived(
		authenticator.accounts.filter((account) =>
			matchesSearchQuery(`${account.issuer} ${account.account}`, query),
		),
	);

	$effect(() => {
		if (authenticator.accounts.length > 0) return;
		if (editing) {
			editing = false;
			copyFeedback.reset();
		}
		if (paused) {
			paused = false;
			frozenCodes = null;
			frozenProgress = 0;
			snappingProgress = false;
		}
	});

	onMount(() => {
		clientNow = Date.now();
		const interval = window.setInterval(() => {
			clientNow = Date.now();
		}, 100);
		return () => {
			window.clearInterval(interval);
			copyFeedback.destroy();
		};
	});

	async function addAuthenticator(input: AddAuthenticatorInput): Promise<void> {
		await authenticator.add(input);
		notifications.success('Authenticator added.');
	}

	function toggleEditing(): void {
		if (authenticator.accounts.length === 0) return;
		editing = !editing;
		query = '';
		copyFeedback.reset();
	}

	function reorderAuthenticators(items: AuthenticatorAccount[]): void {
		authenticator.reorder(items);
	}

	async function persistAuthenticatorOrder(
		items: AuthenticatorAccount[],
	): Promise<void> {
		try {
			await authenticator.persistOrder(items);
			notifications.success('Authenticator order saved.');
		} catch (error) {
			notifications.error(requestError(error));
		}
	}

	async function deleteAuthenticator(id: string): Promise<void> {
		const account = authenticator.accounts.find((item) => item.id === id);
		try {
			await authenticator.delete(id);
			if (account) notifications.success(`${account.issuer} deleted.`);
		} catch (error) {
			notifications.error(requestError(error));
		}
	}

	async function togglePaused(): Promise<void> {
		if (authenticator.accounts.length === 0) return;
		if (paused) {
			snappingProgress = true;
			paused = false;
			frozenCodes = null;
			await tick();
			if (!rotationFill)
				throw new Error('Rotation progress bar is not mounted.');
			rotationFill.getBoundingClientRect();
			snappingProgress = false;
			return;
		}
		frozenCodes = new Map(liveCodes);
		frozenProgress = visualLiveProgress;
		paused = true;
	}

	function handleKeydown(event: KeyboardEvent): void {
		if (
			authenticator.accounts.length === 0 ||
			event.code !== 'Space' ||
			event.repeat ||
			event.defaultPrevented ||
			document.querySelector('dialog[open]') ||
			shortcutTargetIsInteractive(event.target)
		)
			return;
		event.preventDefault();
		void togglePaused();
	}

	function shortcutTargetIsInteractive(target: EventTarget | null): boolean {
		return (
			target instanceof Element &&
			Boolean(
				target.closest(
					'input, textarea, select, button, a, [contenteditable="true"]',
				),
			)
		);
	}

	function displayCode(code?: string): string {
		if (!code) return '••• •••';
		return `${code.slice(0, 3)} ${code.slice(3)}`;
	}

	async function copy(
		authenticator: AuthenticatorAccount,
		code?: AuthenticatorCodePeriod,
	): Promise<void> {
		if (!code) return;
		const sequence = copyFeedback.begin(authenticator.id);
		try {
			await navigator.clipboard.writeText(code.code);
			if (!copyFeedback.isCurrent(sequence)) return;
			copyFeedback.finish(sequence);
		} catch (error) {
			if (!copyFeedback.isCurrent(sequence)) return;
			copyFeedback.cancel(sequence);
			notifications.error(requestError(error));
		}
	}
</script>

<svelte:head>
	<title>Authenticator · Arcadia</title>
	<meta name="description" content="Live authenticator arcadia." />
</svelte:head>

<svelte:window onkeydown={handleKeydown} />

{#snippet authenticatorRow(account: AuthenticatorAccount)}
	{@const code = displayedCodes.get(account.id)}
	{#if editing}
		<div class="authenticator-row edit-row">
			<div class="mark-cell">
				<RecordMark
					name={account.issuer}
					accent={account.accent}
					iconImage={account.iconImage}
				/>
			</div>
			<strong class="account-name">{account.issuer}</strong>
			<code
				class="authenticator-code"
				aria-label={code
					? `${code.code} for ${account.issuer}`
					: `Code loading for ${account.issuer}`}
			>
				{displayCode(code?.code)}
			</code>
			<span class="drag-grip" aria-hidden="true">
				<GripVertical size={19} />
			</span>
		</div>
	{:else}
		<button
			class="authenticator-row copy-row"
			type="button"
			onclick={() => copy(account, code)}
			aria-label={code
				? `Copy ${account.issuer} code`
				: `Code loading for ${account.issuer}`}
			disabled={!code}
		>
			<div class="mark-cell">
				<RecordMark
					name={account.issuer}
					accent={account.accent}
					iconImage={account.iconImage}
				/>
			</div>
			<div class="account-label">
				<strong class="account-name">{account.issuer}</strong>
			</div>
			<div class="code-cluster">
				<code
					class="authenticator-code"
					aria-label={code
						? `${code.code} for ${account.issuer}`
						: `Code loading for ${account.issuer}`}
				>
					{displayCode(code?.code)}
				</code>
				<CopyFeedback
					copied={copyFeedback.active === account.id}
					message={`${account.issuer} code copied`}
					sequence={copyFeedback.sequence}
				/>
			</div>
		</button>
	{/if}
{/snippet}

{#snippet authenticatorEmptyIcon()}
	<ShieldCheck size={26} />
{/snippet}

{#snippet emptyAuthenticatorList()}
	{#if query.trim()}
		<EmptyState
			title="No matching authenticators"
			description="Try a different issuer or account name."
			icon={authenticatorEmptyIcon}
		/>
	{:else}
		<EmptyState
			title="No authenticators"
			description="To get started, click the + button."
			icon={authenticatorEmptyIcon}
		/>
	{/if}
{/snippet}

<PageHeader title="Authenticator" />

<Panel class="authenticator-panel">
	<ArcadiaToolbar
		bind:query
		disabled={editing}
		placeholder="Search"
		searchLabel="Search authenticators"
	>
		<button
			class:pressed={paused}
			class="arcadia-control-button"
			type="button"
			onclick={togglePaused}
			disabled={authenticator.accounts.length === 0}
			aria-label={paused ? 'Resume code updates' : 'Pause code updates'}
			aria-keyshortcuts="Space"
			aria-pressed={paused}
			title={paused ? 'Resume' : 'Pause'}
		>
			{#if paused}<Play size={16} />{:else}<Pause size={16} />{/if}
		</button>
		<AddAuthenticatorDialog onadd={addAuthenticator} />
		<button
			class:pressed={editing}
			class="arcadia-control-button"
			type="button"
			onclick={toggleEditing}
			disabled={authenticator.accounts.length === 0}
			aria-label={editing ? 'Finish editing' : 'Edit authenticators'}
			aria-pressed={editing}
			title={editing ? 'Finish editing' : 'Edit'}
		>
			<Pencil size={15} />
		</button>
	</ArcadiaToolbar>
	<div
		class:at-start={displayedProgress === 0}
		class:paused
		class:snapping={snappingProgress}
		class="rotation-progress"
		role="progressbar"
		aria-label="Code rotation"
		aria-valuemin="0"
		aria-valuemax="100"
		aria-valuenow={Math.round(displayedProgress * 100)}
		aria-valuetext={paused
			? 'Paused'
			: `${Math.round(displayedProgress * 100)} percent`}
	>
		<div
			bind:this={rotationFill}
			class="rotation-fill"
			style={`--rotation-progress: ${displayedProgress}`}
		></div>
	</div>

	{#if authenticator.loading}
		<p class="loading-row">Loading…</p>
	{:else}
		<ReorderableRecordList
			items={visibleAccounts}
			{editing}
			dndType="authenticator"
			label="Authenticator order"
			itemLabel={(account) => account.issuer}
			onreorder={reorderAuthenticators}
			onfinalize={persistAuthenticatorOrder}
			ondelete={deleteAuthenticator}
			row={authenticatorRow}
			empty={emptyAuthenticatorList}
		/>
	{/if}
</Panel>

<style>
	:global(.authenticator-panel) {
		overflow: hidden;
		margin-top: 1.5rem;
	}

	.rotation-progress {
		height: 0.4rem;
		overflow: hidden;
		border-block: 1px solid rgb(36 111 202 / 0.08);
		background: rgb(36 111 202 / 0.08);
		box-shadow: inset 0 1px 2px rgb(10 42 83 / 0.05);
	}

	.loading-row {
		margin: 0;
		padding: 1rem;
		color: var(--arcadia-muted);
		font-size: 0.78rem;
	}

	.rotation-fill {
		width: 100%;
		height: 100%;
		background: linear-gradient(90deg, #1769c2, #2586e8 62%, #55c9db);
		box-shadow: 0 0 12px rgb(37 134 232 / 0.32);
		transform: scaleX(var(--rotation-progress));
		transform-origin: left;
		transition: transform 100ms linear;
	}

	.rotation-progress.paused .rotation-fill {
		background: linear-gradient(90deg, #1a65b7, #2b7cca 55%, #4aa4cf);
		box-shadow: inset 0 2px 3px rgb(5 24 58 / 0.16);
		transition: none;
	}

	.rotation-progress.snapping .rotation-fill {
		transition: none;
	}

	.rotation-progress.at-start .rotation-fill {
		transition: none;
	}

	.copy-row:focus-visible {
		outline: 3px solid rgb(37 134 232 / 0.2);
		outline-offset: 2px;
	}

	.authenticator-row {
		display: grid;
		width: 100%;
		grid-template-columns: auto minmax(10rem, 1fr) auto;
		align-items: center;
		gap: 0.9rem;
		box-sizing: border-box;
		border: 0;
		background: transparent;
		color: inherit;
		padding: 0.85rem 0.95rem;
		text-align: left;
	}

	.copy-row {
		cursor: copy;
		transition: background 120ms;
	}

	.copy-row:hover {
		background: rgb(36 111 202 / 0.035);
	}

	.copy-row:active {
		background: rgb(36 111 202 / 0.07);
	}

	.copy-row:disabled {
		cursor: wait;
	}

	.edit-row {
		grid-template-columns: auto minmax(10rem, 1fr) auto auto;
		cursor: grab;
		user-select: none;
	}

	.edit-row:active {
		cursor: grabbing;
	}

	.account-name {
		min-width: 0;
		overflow: hidden;
		color: var(--site-heading-fg);
		font-size: 1.1rem;
		font-weight: 740;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.account-label {
		display: flex;
		min-width: 0;
		align-items: center;
	}

	.code-cluster {
		display: flex;
		align-items: center;
		justify-content: flex-end;
		gap: 0.55rem;
	}

	.authenticator-code {
		color: var(--site-heading-fg);
		font-family:
			ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
		font-size: clamp(1.15rem, 2.5vw, 1.42rem);
		font-variant-numeric: tabular-nums;
		font-weight: 720;
		letter-spacing: 0.085em;
		white-space: nowrap;
	}

	.drag-grip {
		display: grid;
		width: 2.2rem;
		height: 2.2rem;
		place-items: center;
	}

	.drag-grip {
		border-radius: 0.65rem;
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
		.authenticator-row,
		.edit-row {
			grid-template-columns: auto minmax(0, 1fr) auto;
			gap: 0.65rem;
		}

		.mark-cell {
			grid-row: 1 / span 2;
		}

		.authenticator-code {
			grid-column: 2;
			grid-row: 2;
			justify-self: start;
		}

		.code-cluster {
			grid-column: 2;
			grid-row: 2;
			justify-self: start;
		}

		.drag-grip {
			grid-column: 3;
			grid-row: 1 / span 2;
		}
	}
</style>
