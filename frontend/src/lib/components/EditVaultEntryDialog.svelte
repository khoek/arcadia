<script lang="ts">
	import { BookOpenText, Eye, EyeOff } from '@lucide/svelte';

	import ArcadiaDialog from '$lib/components/ArcadiaDialog.svelte';
	import IconImageField from '$lib/components/IconImageField.svelte';
	import { getArcadiaNotifications } from '$lib/services/context';
	import type {
		UpdateVaultEntryInput,
		VaultEntry,
		VaultEntryDetail,
	} from '$lib/vault';

	interface Props {
		entry: VaultEntry;
		onload: () => Promise<VaultEntryDetail>;
		onsave: (input: UpdateVaultEntryInput) => void | Promise<void>;
	}

	let { entry, onload, onsave }: Props = $props();
	const notifications = getArcadiaNotifications();
	let username = $state('');
	let password = $state('');
	let notes = $state('');
	let iconImage = $state<string>();
	let showPassword = $state(false);
	let error = $state('');
	let submitting = $state(false);
	let loading = $state(false);
	let ready = $state(false);

	async function openEditor(): Promise<void> {
		username = '';
		password = '';
		notes = '';
		iconImage = entry.iconImage;
		showPassword = false;
		error = '';
		loading = true;
		ready = false;
		try {
			const detail = await onload();
			username = detail.username ?? '';
			password = detail.password ?? '';
			notes = detail.notes ?? '';
			iconImage = detail.iconImage;
			ready = true;
		} catch (cause) {
			notifications.error(
				cause instanceof Error ? cause.message : 'Unable to load entry.',
			);
		} finally {
			loading = false;
		}
	}

	async function submit(event: SubmitEvent, close: () => void): Promise<void> {
		event.preventDefault();
		if (!ready) return;
		if (username.length > 320) {
			error = 'Username is too long.';
			return;
		}
		if (password.length > 1_024) {
			error = 'Password is too long.';
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
				username: username.trim() || null,
				password: password || null,
				notes: notes || null,
				iconImage: iconImage ?? null,
			});
			close();
		} catch (cause) {
			notifications.error(
				cause instanceof Error ? cause.message : 'Unable to save entry.',
			);
		} finally {
			submitting = false;
		}
	}
</script>

{#snippet trigger(open: () => void)}
	<button
		class="entry-editor-button"
		type="button"
		onclick={open}
		aria-label={`Edit ${entry.name} password and notes`}
		title="Password and notes"
	>
		<BookOpenText size={17} aria-hidden="true" />
	</button>
{/snippet}

<ArcadiaDialog
	buttonLabel={`Edit ${entry.name}`}
	kicker="Password entry"
	title={entry.name}
	titleId={`edit-vault-${entry.id}-title`}
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
			<div class="credential-fields">
				<label class="field-label">
					<span>Username</span>
					<input
						bind:value={username}
						class="arcadia-field"
						name="username"
						placeholder="optional"
						maxlength="320"
						autocomplete="off"
					/>
				</label>
				<label class="field-label">
					<span>Password</span>
					<div class="password-input">
						<input
							bind:value={password}
							class="arcadia-field"
							name="password"
							type={showPassword ? 'text' : 'password'}
							placeholder="optional"
							maxlength="1024"
							autocomplete="new-password"
						/>
						<button
							type="button"
							onclick={() => (showPassword = !showPassword)}
							aria-label={showPassword ? 'Hide password' : 'Show password'}
							aria-pressed={showPassword}
							title={showPassword ? 'Hide password' : 'Show password'}
						>
							{#if showPassword}<EyeOff size={17} />{:else}<Eye
									size={17}
								/>{/if}
						</button>
					</div>
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
					placeholder="Notes about this entry"
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

	.credential-fields {
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

	.password-input {
		position: relative;
	}

	.password-input input {
		padding-right: 2.8rem;
	}

	.password-input button {
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

	.password-input button:hover {
		background: rgb(36 111 202 / 0.07);
		color: var(--site-link-fg);
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
		.credential-fields {
			grid-template-columns: 1fr;
		}
	}
</style>
