<script lang="ts">
	import ArcadiaDialog from '$lib/components/ArcadiaDialog.svelte';
	import IconImageField from '$lib/components/IconImageField.svelte';
	import { getArcadiaNotifications } from '$lib/services/context';
	import {
		normalizeCardNumber,
		validCardCvc,
		validCardExpiry,
		validCardNumber,
		type AddCardEntryInput,
	} from '$lib/cards';

	interface Props {
		onadd: (input: AddCardEntryInput) => void | Promise<void>;
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
		const numberValue = text(form, 'number', 'Card number', 32);
		if (numberValue === null) return;
		const expiryValue = text(form, 'expiry', 'Expiry', 7);
		if (expiryValue === null) return;
		const cvc = text(form, 'cvc', 'CVC', 4);
		if (cvc === null) return;
		const cardholder = text(form, 'cardholder', 'Cardholder', 160);
		if (cardholder === null) return;
		const notes = text(form, 'notes', 'Notes', 20_000, false);
		if (notes === null) return;
		const number = normalizeCardNumber(numberValue);
		const expiry = normalizeExpiry(expiryValue);

		if (!name) {
			error = 'Name is required.';
			return;
		}
		if (!validCardNumber(number)) {
			error = 'Enter a card number with 12 to 19 digits.';
			return;
		}
		if (!expiry || !validCardExpiry(expiry)) {
			error = 'Enter an expiry in MM/YY or MM/YYYY format.';
			return;
		}
		if (!validCardCvc(cvc)) {
			error = 'Enter a 3 or 4 digit CVC.';
			return;
		}

		error = '';
		submitting = true;
		try {
			await onadd({
				name,
				number,
				expiry,
				cvc,
				...(cardholder ? { cardholder } : {}),
				...(notes ? { notes } : {}),
				...(iconImage ? { iconImage } : {}),
			});
			formElement.reset();
			iconImage = undefined;
			close();
		} catch (cause) {
			notifications.error(
				cause instanceof Error ? cause.message : 'Unable to add card.',
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

	function normalizeExpiry(value: string): string {
		const compact = value.replace(/\s/g, '');
		if (/^\d{4}$/.test(compact))
			return `${compact.slice(0, 2)}/${compact.slice(2)}`;
		return compact;
	}
</script>

<ArcadiaDialog
	buttonLabel="Add card"
	kicker="Add card"
	title="New card"
	titleId="add-card-title"
	submitLabel="Add card"
	{submitting}
	onopen={() => (error = '')}
	onsubmit={submit}
>
	<div class="card-fields">
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
					placeholder="Example card"
					maxlength="80"
					autocomplete="off"
					required
				/>
			</label>
		</div>

		<label class="field-label">
			<span>Card number</span>
			<input
				class="arcadia-field number-field"
				name="number"
				type="text"
				inputmode="numeric"
				placeholder="1234 5678 9012 3456"
				maxlength="32"
				autocomplete="cc-number"
				required
			/>
		</label>

		<div class="detail-fields">
			<label class="field-label">
				<span>Expiry</span>
				<input
					class="arcadia-field number-field"
					name="expiry"
					type="text"
					inputmode="numeric"
					placeholder="MM/YY"
					maxlength="7"
					autocomplete="cc-exp"
					required
				/>
			</label>
			<label class="field-label">
				<span>CVC</span>
				<input
					class="arcadia-field number-field"
					name="cvc"
					type="password"
					inputmode="numeric"
					placeholder="•••"
					maxlength="4"
					autocomplete="cc-csc"
					required
				/>
			</label>
		</div>

		<label class="field-label">
			<span>Cardholder <small>optional</small></span>
			<input
				class="arcadia-field"
				name="cardholder"
				placeholder="Name on card"
				maxlength="160"
				autocomplete="cc-name"
			/>
		</label>

		<label class="field-label notes-field">
			<span>Notes <small>optional</small></span>
			<textarea
				class="arcadia-field"
				name="notes"
				rows="4"
				maxlength="20000"
				placeholder="Notes about this card"
			></textarea>
		</label>
	</div>

	{#if error}<p class="form-error" role="alert">{error}</p>{/if}
</ArcadiaDialog>

<style>
	.card-fields {
		display: grid;
		gap: 0.9rem;
		margin-top: 1.2rem;
	}

	.identity-fields,
	.detail-fields {
		display: grid;
		gap: 0.75rem;
	}

	.identity-fields {
		grid-template-columns: 3.25rem minmax(0, 1fr);
	}

	.detail-fields {
		grid-template-columns: 1fr 1fr;
	}

	.field-label > span,
	.icon-field > span {
		display: block;
		margin: 0 0 0.35rem 0.1rem;
		color: var(--arcadia-muted);
		font-size: 0.7rem;
		font-weight: 700;
	}

	.field-label small {
		font-size: 0.65rem;
		font-weight: 600;
	}

	.card-fields .arcadia-field {
		box-sizing: border-box;
		min-height: 3.25rem;
	}

	.icon-field :global(.icon-image-control) {
		margin-top: 0;
	}

	.number-field {
		font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
		font-variant-numeric: tabular-nums;
		letter-spacing: 0.06em;
	}

	.notes-field textarea {
		min-height: 6rem;
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

	@media (max-width: 520px) {
		.detail-fields {
			grid-template-columns: 1fr;
		}
	}
</style>
