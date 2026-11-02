<script lang="ts">
	import type { Snippet } from 'svelte';
	import { Plus, X } from '@lucide/svelte';

	interface Props {
		buttonLabel: string;
		kicker: string;
		title: string;
		titleId: string;
		submitLabel: string;
		submitting?: boolean;
		pendingLabel?: string;
		onopen?: () => void;
		onsubmit: (event: SubmitEvent, close: () => void) => void | Promise<void>;
		children?: Snippet;
		trigger?: Snippet<[() => void]>;
	}

	let {
		buttonLabel,
		kicker,
		title,
		titleId,
		submitLabel,
		submitting = false,
		pendingLabel = 'Adding…',
		onopen,
		onsubmit,
		children,
		trigger,
	}: Props = $props();
	let dialog: HTMLDialogElement;

	function open(): void {
		onopen?.();
		dialog.showModal();
	}

	function close(): void {
		dialog.close();
	}
</script>

{#if trigger}
	{@render trigger(open)}
{:else}
	<button
		class="arcadia-control-button"
		type="button"
		onclick={open}
		aria-label={buttonLabel}
		title={buttonLabel}
	>
		<Plus size={17} strokeWidth={2.4} />
	</button>
{/if}

<dialog bind:this={dialog} aria-labelledby={titleId}>
	<form onsubmit={(event) => onsubmit(event, close)}>
		<header>
			<div>
				<p>{kicker}</p>
				<h2 id={titleId}>{title}</h2>
			</div>
			<button
				class="close-button"
				type="button"
				onclick={close}
				aria-label="Close"
			>
				<X size={19} />
			</button>
		</header>

		{@render children?.()}

		<footer>
			<button
				class="arcadia-button arcadia-button-secondary"
				type="button"
				onclick={close}
			>
				Cancel
			</button>
			<button class="arcadia-button" type="submit" disabled={submitting}>
				{submitting ? pendingLabel : submitLabel}
			</button>
		</footer>
	</form>
</dialog>

<style>
	dialog {
		position: fixed;
		inset: 0 0 0 var(--arcadia-sidebar-width);
		width: min(32rem, calc(100vw - 2rem));
		max-height: calc(100dvh - 2rem);
		box-sizing: border-box;
		border: 1px solid var(--arcadia-line);
		border-radius: 1.35rem;
		background: #f8fbff;
		box-shadow: 0 30px 90px -32px rgb(5 24 58 / 0.72);
		color: var(--site-surface-fg);
		margin: auto;
		overflow-y: auto;
		overscroll-behavior: contain;
		padding: 0;
	}

	dialog::backdrop {
		background: rgb(5 24 58 / 0.48);
		backdrop-filter: blur(5px);
	}

	form {
		padding: 1.35rem;
	}

	header,
	footer {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 1rem;
	}

	header p {
		margin: 0 0 0.2rem;
		color: var(--site-link-fg);
		font-size: 0.68rem;
		font-weight: 750;
		letter-spacing: 0.1em;
		text-transform: uppercase;
	}

	header h2 {
		margin: 0;
		color: var(--site-heading-fg);
		font-size: 1.25rem;
		letter-spacing: -0.025em;
	}

	.close-button {
		display: grid;
		width: 2.25rem;
		height: 2.25rem;
		border: 1px solid var(--arcadia-line);
		border-radius: 0.72rem;
		background: white;
		color: var(--arcadia-muted);
		cursor: pointer;
		place-items: center;
	}

	footer {
		justify-content: flex-end;
		border-top: 1px solid var(--arcadia-line);
		margin-top: 1.2rem;
		padding-top: 1rem;
	}

	@media (max-width: 800px) {
		dialog {
			inset: 0;
		}
	}
</style>
