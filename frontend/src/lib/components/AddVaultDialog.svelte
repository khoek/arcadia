<script lang="ts">
	import ArcadiaDialog from '$lib/components/ArcadiaDialog.svelte';
	import IconImageField from '$lib/components/IconImageField.svelte';
	import { getArcadiaNotifications } from '$lib/services/context';
	import type { AddVaultEntryInput } from '$lib/vault';

	interface Props {
		onadd: (input: AddVaultEntryInput) => void | Promise<void>;
	}

	let { onadd }: Props = $props();
	const notifications = getArcadiaNotifications();
	let error = $state('');
	let submitting = $state(false);
	let iconImage = $state<string>();

	async function submit(event: SubmitEvent, close: () => void): Promise<void> {
		event.preventDefault();
		const formElement = event.currentTarget as HTMLFormElement;
		const form = new FormData(formElement);
		const name = text(form, 'name', 'Name', 80);
		if (name === null) return;
		const username = text(form, 'username', 'Username', 320);
		if (username === null) return;
		const password = text(form, 'password', 'Password', 1_024, false);
		if (password === null) return;
		if (!name) {
			error = 'Name is required.';
			return;
		}

		error = '';
		submitting = true;
		try {
			await onadd({
				name,
				...(username ? { username } : {}),
				...(password ? { password } : {}),
				...(iconImage ? { iconImage } : {}),
			});
			formElement.reset();
			iconImage = undefined;
			close();
		} catch (cause) {
			notifications.error(
				cause instanceof Error ? cause.message : 'Unable to add entry.',
			);
		} finally {
			submitting = false;
		}
	}

	function text(
		form: FormData,
		field: string,
		label: string,
		maxLength: number,
		trim = true,
	): string | null {
		const value = form.get(field);
		if (typeof value !== 'string')
			throw new Error(`${field} must be a text field.`);
		if (value.length > maxLength) {
			error = `${label} is too long.`;
			return null;
		}
		return trim ? value.trim() : value;
	}
</script>

<ArcadiaDialog
	buttonLabel="Add password"
	kicker="Add password"
	title="New password"
	titleId="add-password-title"
	submitLabel="Add password"
	{submitting}
	onopen={() => (error = '')}
	onsubmit={submit}
>
	<div class="vault-fields">
		<div class="identity-fields">
			<div class="icon-field">
				<span>Icon</span>
				<IconImageField bind:value={iconImage} />
			</div>
			<label class="field-label">
				<span>Name</span>
				<input
					class="arcadia-field"
					name="name"
					placeholder="Example"
					maxlength="80"
					autocomplete="off"
					required
				/>
			</label>
		</div>
		<div class="credential-fields">
			<label class="field-label">
				<span>Username</span>
				<input
					class="arcadia-field"
					name="username"
					placeholder="optional"
					maxlength="320"
					autocomplete="off"
				/>
			</label>
			<label class="field-label">
				<span>Password</span>
				<input
					class="arcadia-field password-field"
					name="password"
					type="password"
					placeholder="optional"
					maxlength="1024"
					autocomplete="new-password"
				/>
			</label>
		</div>
	</div>

	{#if error}<p class="form-error" role="alert">{error}</p>{/if}
</ArcadiaDialog>

<style>
	.vault-fields {
		display: grid;
		gap: 0.9rem;
		margin-top: 1.2rem;
	}

	.identity-fields {
		display: grid;
		grid-template-columns: 3.25rem minmax(0, 1fr);
		gap: 0.75rem;
	}

	.credential-fields {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 0.75rem;
	}

	.field-label > span,
	.icon-field > span {
		display: block;
		margin: 0 0 0.35rem 0.1rem;
		color: var(--arcadia-muted);
		font-size: 0.7rem;
		font-weight: 700;
	}

	.vault-fields .arcadia-field {
		box-sizing: border-box;
		height: 3.25rem;
	}

	.icon-field :global(.icon-image-control) {
		margin-top: 0;
	}

	.password-field {
		font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
	}

	.form-error {
		border-radius: 0.75rem;
		background: #fff0f0;
		color: #a52a37;
		font-size: 0.75rem;
		margin: 0.8rem 0 0;
		padding: 0.65rem 0.75rem;
	}

	@media (max-width: 520px) {
		.credential-fields {
			grid-template-columns: 1fr;
		}
	}
</style>
