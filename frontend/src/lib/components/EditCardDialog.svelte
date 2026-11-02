<script lang="ts">
	import { BookOpenText, Eye, EyeOff } from '@lucide/svelte';

	import ArcadiaDialog from '$lib/components/ArcadiaDialog.svelte';
	import IconImageField from '$lib/components/IconImageField.svelte';
	import { getArcadiaNotifications } from '$lib/services/context';
	import {
		normalizeCardNumber,
		validCardCvc,
		validCardExpiry,
		validCardNumber,
		type CardEntry,
		type CardEntryDetail,
		type UpdateCardEntryInput,
	} from '$lib/cards';

	interface Props {
		entry: CardEntry;
		onload: () => Promise<CardEntryDetail>;
		onsave: (input: UpdateCardEntryInput) => void | Promise<void>;
	}

	let { entry, onload, onsave }: Props = $props();
	const notifications = getArcadiaNotifications();
	let number = $state('');
	let expiry = $state('');
	let cvc = $state('');
	let cardholder = $state('');
	let notes = $state('');
	let iconImage = $state<string>();
	let showNumber = $state(false);
	let showCvc = $state(false);
	let error = $state('');
	let submitting = $state(false);
	let loading = $state(false);
	let ready = $state(false);

	async function openEditor(): Promise<void> {
		number = '';
		expiry = '';
		cvc = '';
		cardholder = '';
		notes = '';
		iconImage = entry.iconImage;
		showNumber = false;
		showCvc = false;
		error = '';
		loading = true;
		ready = false;
		try {
			const detail = await onload();
			number = detail.number ?? '';
			expiry = detail.expiry ?? '';
			cvc = detail.cvc ?? '';
			cardholder = detail.cardholder ?? '';
			notes = detail.notes ?? '';
			iconImage = detail.iconImage;
			ready = true;
		} catch (cause) {
			notifications.error(
				cause instanceof Error ? cause.message : 'Unable to load card.',
			);
		} finally {
			loading = false;
		}
	}

	async function submit(event: SubmitEvent, close: () => void): Promise<void> {
		event.preventDefault();
		if (!ready) return;
		number = normalizeCardNumber(number);
		expiry = normalizeExpiry(expiry);
		if (!validCardNumber(number)) {
			error = 'Enter a card number with 12 to 19 digits.';
			return;
		}
		if (!validCardExpiry(expiry)) {
			error = 'Enter an expiry in MM/YY or MM/YYYY format.';
			return;
		}
		if (!validCardCvc(cvc)) {
			error = 'Enter a 3 or 4 digit CVC.';
			return;
		}
		if (cardholder.length > 160) {
			error = 'Cardholder is too long.';
			return;
		}
		if (notes.length > 20_000) {
			error = 'Notes are too long.';
			return;
		}

		error = '';
		submitting = true;
		try {
			await onsave({
				number,
				expiry,
				cvc,
				cardholder: cardholder.trim() || null,
				notes: notes || null,
				iconImage: iconImage ?? null,
			});
			close();
		} catch (cause) {
			notifications.error(
				cause instanceof Error ? cause.message : 'Unable to save card.',
			);
		} finally {
			submitting = false;
		}
	}

	function normalizeExpiry(value: string): string {
		const compact = value.trim().replace(/\s/g, '');
		if (/^\d{4}$/.test(compact))
			return `${compact.slice(0, 2)}/${compact.slice(2)}`;
		return compact;
	}
</script>

{#snippet trigger(open: () => void)}
	<button
		class="entry-editor-button"
		type="button"
		onclick={open}
		aria-label={`Edit ${entry.name} card and notes`}
		title="Card and notes"
	>
		<BookOpenText size={17} aria-hidden="true" />
	</button>
{/snippet}

<ArcadiaDialog
	buttonLabel={`Edit ${entry.name}`}
	kicker="Card entry"
	title={entry.name}
	titleId={`edit-card-${entry.id}-title`}
	submitLabel="Save"
	pendingLabel="Saving…"
	submitting={submitting || loading || !ready}
	onopen={openEditor}
	onsubmit={submit}
	{trigger}
>
	{#if loading}
		<p class="loading-entry">Loading…</p>
	{:else}
		<div class="editor-fields">
			<div class="secret-fields">
				<label class="field-label">
					<span>Card number</span>
					<div class="secret-input">
						<input
							bind:value={number}
							class="arcadia-field number-field"
							name="number"
							type={showNumber ? 'text' : 'password'}
							inputmode="numeric"
							maxlength="32"
							autocomplete="cc-number"
						/>
						<button
							type="button"
							onclick={() => (showNumber = !showNumber)}
							aria-label={showNumber ? 'Hide card number' : 'Show card number'}
							aria-pressed={showNumber}
							title={showNumber ? 'Hide card number' : 'Show card number'}
						>
							{#if showNumber}<EyeOff size={17} />{:else}<Eye size={17} />{/if}
						</button>
					</div>
				</label>
				<label class="field-label">
					<span>CVC</span>
					<div class="secret-input">
						<input
							bind:value={cvc}
							class="arcadia-field number-field"
							name="cvc"
							type={showCvc ? 'text' : 'password'}
							inputmode="numeric"
							maxlength="4"
							autocomplete="cc-csc"
						/>
						<button
							type="button"
							onclick={() => (showCvc = !showCvc)}
							aria-label={showCvc ? 'Hide CVC' : 'Show CVC'}
							aria-pressed={showCvc}
							title={showCvc ? 'Hide CVC' : 'Show CVC'}
						>
							{#if showCvc}<EyeOff size={17} />{:else}<Eye size={17} />{/if}
						</button>
					</div>
				</label>
			</div>

			<div class="detail-fields">
				<label class="field-label">
					<span>Expiry</span>
					<input
						bind:value={expiry}
						class="arcadia-field number-field"
						name="expiry"
						type="text"
						inputmode="numeric"
						maxlength="7"
						autocomplete="cc-exp"
					/>
				</label>
				<label class="field-label">
					<span>Cardholder</span>
					<input
						bind:value={cardholder}
						class="arcadia-field"
						name="cardholder"
						maxlength="160"
						autocomplete="cc-name"
					/>
				</label>
			</div>

			<IconImageField bind:value={iconImage} />

			<label class="field-label notes-field">
				<span>Notes</span>
				<textarea
					bind:value={notes}
					class="arcadia-field"
					name="notes"
					rows="8"
					maxlength="20000"
					placeholder="Notes about this card"
				></textarea>
			</label>
		</div>
	{/if}

	{#if error}<p class="form-error" role="alert">{error}</p>{/if}
</ArcadiaDialog>

<style>
	.entry-editor-button {
		display: grid;
		width: 2.2rem;
		height: 2.2rem;
		place-items: center;
		border: 1px solid transparent;
		border-radius: 0.65rem;
		background: transparent;
		color: var(--site-heading-fg);
		cursor: pointer;
		font-size: 1rem;
		transition:
			background 120ms,
			border-color 120ms;
	}

	.entry-editor-button:hover {
		border-color: var(--arcadia-line);
		background: rgb(36 111 202 / 0.07);
	}

	.editor-fields {
		display: grid;
		gap: 1rem;
		margin-top: 1.2rem;
	}

	.secret-fields,
	.detail-fields {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 0.75rem;
	}

	.field-label > span {
		display: flex;
		align-items: baseline;
		justify-content: space-between;
		margin: 0 0 0.35rem 0.1rem;
		color: var(--arcadia-muted);
		font-size: 0.7rem;
		font-weight: 700;
	}

	.secret-input {
		position: relative;
	}

	.secret-input input {
		padding-right: 2.8rem;
	}

	.secret-input button {
		position: absolute;
		top: 50%;
		right: 0.45rem;
		display: grid;
		width: 2rem;
		height: 2rem;
		place-items: center;
		border: 0;
		border-radius: 0.55rem;
		background: transparent;
		color: var(--arcadia-muted);
		cursor: pointer;
		transform: translateY(-50%);
	}

	.secret-input button:hover {
		background: rgb(36 111 202 / 0.07);
		color: var(--site-link-fg);
	}

	.number-field {
		font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
		font-variant-numeric: tabular-nums;
		letter-spacing: 0.06em;
	}

	.notes-field textarea {
		min-height: 9rem;
		box-sizing: border-box;
		resize: vertical;
		font: inherit;
		line-height: 1.45;
	}

	.form-error {
		border-radius: 0.75rem;
		background: #fff0f0;
		color: #a52a37;
		font-size: 0.75rem;
		margin: 0.8rem 0 0;
		padding: 0.65rem 0.75rem;
	}

	.loading-entry {
		margin: 1.25rem 0;
		color: var(--arcadia-muted);
		font-size: 0.78rem;
	}

	@media (max-width: 520px) {
		.secret-fields,
		.detail-fields {
			grid-template-columns: 1fr;
		}
	}
</style>
