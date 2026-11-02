<script lang="ts">
	import { onDestroy } from 'svelte';
	import { KeyRound, Upload } from '@lucide/svelte';

	import type { AddAuthenticatorInput } from '$lib/authenticator';
	import ArcadiaDialog from '$lib/components/ArcadiaDialog.svelte';
	import IconImageField from '$lib/components/IconImageField.svelte';
	import { getArcadiaNotifications } from '$lib/services/context';

	interface Props {
		onadd: (input: AddAuthenticatorInput) => void | Promise<void>;
	}

	const qrImageTypes = new Set([
		'image/png',
		'image/jpeg',
		'image/webp',
		'image/gif',
	]);

	let { onadd }: Props = $props();
	const notifications = getArcadiaNotifications();
	let mode: 'qr' | 'manual' = $state('qr');
	let qrPreview: string | null = $state(null);
	let qrFileName = $state('');
	let error = $state('');
	let submitting = $state(false);
	let iconImage = $state<string>();

	onDestroy(() => {
		if (qrPreview) URL.revokeObjectURL(qrPreview);
	});

	function selectQr(event: Event): void {
		const file = (event.currentTarget as HTMLInputElement).files?.[0];
		if (!file) return;
		if (qrPreview) URL.revokeObjectURL(qrPreview);
		qrPreview = URL.createObjectURL(file);
		qrFileName = file.name;
	}

	async function submit(event: SubmitEvent, close: () => void): Promise<void> {
		event.preventDefault();
		const formElement = event.currentTarget as HTMLFormElement;
		const form = new FormData(formElement);
		let input: AddAuthenticatorInput;

		if (mode === 'qr') {
			const qr = form.get('qr');
			if (!(qr instanceof File) || qr.size === 0) {
				fail('Choose a QR code image.');
				return;
			}
			if (!qrImageTypes.has(qr.type)) {
				fail('Choose a PNG, JPEG, WebP or GIF image.');
				return;
			}
			if (qr.size > 8 * 1024 * 1024) {
				fail('The QR image must be smaller than 8 MB.');
				return;
			}
			input = {
				mode: 'qr',
				qr,
				...(iconImage ? { iconImage } : {}),
			};
		} else {
			const issuer = requiredText(form, 'issuer', 80);
			const account = requiredText(form, 'account', 160);
			if (!issuer || !account) return;
			const secret = requiredText(form, 'secret', 256);
			if (!secret) return;
			if (secret.replaceAll(' ', '').length < 8) {
				fail('Enter a valid setup key.');
				return;
			}
			input = {
				mode: 'manual',
				issuer,
				account,
				secret,
				...(iconImage ? { iconImage } : {}),
			};
		}

		error = '';
		submitting = true;
		try {
			await onadd(input);
			formElement.reset();
			resetQr();
			iconImage = undefined;
			close();
		} catch (cause) {
			notifications.error(
				cause instanceof Error ? cause.message : 'Unable to add authenticator.',
			);
		} finally {
			submitting = false;
		}
	}

	function requiredText(
		form: FormData,
		field: string,
		maxLength: number,
	): string | null {
		const value = form.get(field);
		if (typeof value !== 'string' || !value.trim())
			return fail(`${field} is required.`);
		if (value.length > maxLength) return fail(`${field} is too long.`);
		return value.trim();
	}

	function fail(message: string): null {
		error = message;
		return null;
	}

	function resetQr(): void {
		if (qrPreview) URL.revokeObjectURL(qrPreview);
		qrPreview = null;
		qrFileName = '';
	}
</script>

<ArcadiaDialog
	buttonLabel="Add authenticator"
	kicker="Add account"
	title="New authenticator"
	titleId="add-authenticator-title"
	submitLabel="Add account"
	{submitting}
	onopen={() => (error = '')}
	onsubmit={submit}
>
	<div class="mode-switch" aria-label="Import method">
		<button
			class:active={mode === 'qr'}
			type="button"
			onclick={() => (mode = 'qr')}
		>
			<Upload size={16} /> QR image
		</button>
		<button
			class:active={mode === 'manual'}
			type="button"
			onclick={() => (mode = 'manual')}
		>
			<KeyRound size={16} /> Setup key
		</button>
	</div>
	<input type="hidden" name="mode" value={mode} />

	<div class="credential-import" class:qr-mode={mode === 'qr'}>
		{#if mode === 'qr'}
			<label class="qr-upload">
				<input
					type="file"
					name="qr"
					accept="image/png,image/jpeg,image/webp,image/gif"
					required
					onchange={selectQr}
				/>
				{#if qrPreview}
					<img src={qrPreview} alt="Selected QR code preview" />
					<span
						><strong>{qrFileName}</strong><small>Ready to import</small></span
					>
				{:else}
					<span class="upload-icon"><Upload size={21} /></span>
					<span
						><strong>Choose a QR code image</strong><small
							>PNG, JPEG, WebP or GIF</small
						></span
					>
				{/if}
			</label>
		{:else}
			<label class="field-label">
				<span>Setup key</span>
				<input
					class="arcadia-field code-field"
					name="secret"
					type="password"
					autocomplete="off"
					placeholder="JBSW Y3DP EHPK 3PXP JBSW Y3DP EHPK 3PXP"
					required
				/>
			</label>
		{/if}
	</div>

	<div class="account-fields">
		<div class="icon-field">
			<span>Icon</span>
			<IconImageField bind:value={iconImage} />
		</div>
		{#if mode === 'manual'}
			<label class="field-label">
				<span>Issuer</span>
				<input
					class="arcadia-field"
					name="issuer"
					placeholder="Example"
					maxlength="80"
					required
				/>
			</label>
			<label class="field-label account-name-field">
				<span>Account</span>
				<input
					class="arcadia-field"
					name="account"
					placeholder="name@example.com"
					maxlength="160"
					required
				/>
			</label>
		{/if}
	</div>

	{#if error}<p class="form-error" role="alert">{error}</p>{/if}
</ArcadiaDialog>

<style>
	.mode-switch,
	.qr-upload,
	.qr-upload span {
		display: flex;
		align-items: center;
	}

	.mode-switch {
		gap: 0.25rem;
		border-radius: 0.85rem;
		background: rgb(36 111 202 / 0.07);
		margin: 1.25rem 0 1rem;
		padding: 0.25rem;
	}

	.mode-switch button {
		display: inline-flex;
		flex: 1;
		align-items: center;
		justify-content: center;
		gap: 0.45rem;
		border: 0;
		border-radius: 0.65rem;
		background: transparent;
		color: var(--arcadia-muted);
		cursor: pointer;
		font-size: 0.78rem;
		font-weight: 700;
		padding: 0.58rem;
	}

	.mode-switch button.active {
		background: white;
		box-shadow: 0 5px 18px -14px rgb(5 24 58 / 0.8);
		color: var(--site-link-fg);
	}

	.credential-import {
		min-height: 7rem;
	}

	.credential-import.qr-mode,
	.credential-import.qr-mode .qr-upload {
		min-height: 12.2rem;
	}

	.credential-import > .field-label {
		display: block;
	}

	.credential-import .arcadia-field {
		box-sizing: border-box;
		height: 3.25rem;
	}

	.qr-upload {
		min-height: 7rem;
		justify-content: center;
		gap: 0.9rem;
		border: 1px dashed #8aa9ce;
		border-radius: 1rem;
		background: rgb(255 255 255 / 0.65);
		cursor: pointer;
		padding: 1rem;
		text-align: left;
	}

	.qr-upload:hover {
		border-color: var(--arcadia-focus);
		background: white;
	}

	.qr-upload input {
		position: absolute;
		width: 1px;
		height: 1px;
		overflow: hidden;
		clip: rect(0 0 0 0);
		white-space: nowrap;
		clip-path: inset(50%);
	}

	.qr-upload img {
		width: 4.5rem;
		height: 4.5rem;
		border: 0.35rem solid white;
		border-radius: 0.6rem;
		box-shadow: 0 8px 25px -18px rgb(5 24 58 / 0.8);
		object-fit: contain;
	}

	.qr-upload span {
		align-items: flex-start;
		flex-direction: column;
		gap: 0.15rem;
	}

	.qr-upload strong {
		max-width: 100%;
		overflow: hidden;
		font-size: 0.82rem;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.qr-upload small {
		color: var(--arcadia-muted);
		font-size: 0.7rem;
	}

	.upload-icon {
		display: grid !important;
		width: 2.7rem;
		height: 2.7rem;
		border-radius: 0.8rem;
		background: rgb(36 111 202 / 0.1);
		color: var(--site-link-fg);
		place-items: center;
	}

	.account-fields {
		display: grid;
		grid-template-columns: 3.25rem minmax(0, 1fr);
		gap: 0.75rem;
		margin-top: 1rem;
	}

	.field-label > span,
	.icon-field > span {
		display: block;
		margin: 0 0 0.35rem 0.1rem;
		color: var(--arcadia-muted);
		font-size: 0.7rem;
		font-weight: 700;
	}

	.account-name-field {
		grid-column: 1 / -1;
	}

	.account-fields .arcadia-field {
		height: 3.25rem;
		box-sizing: border-box;
	}

	.icon-field :global(.icon-image-control) {
		margin-top: 0;
	}

	.code-field {
		font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
		letter-spacing: 0.08em;
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
		.qr-upload {
			justify-content: flex-start;
		}
	}
</style>
