<script lang="ts">
	import { Check, Copy } from '@lucide/svelte';
	import { copyFeedbackFadeMs, copyFeedbackSolidMs } from '$lib/copy-feedback';

	interface Props {
		copied: boolean;
		message: string;
		sequence: number;
	}

	let { copied, message, sequence }: Props = $props();
</script>

{#key copied ? sequence : 0}
	<span
		class:copied
		class="copy-feedback"
		style={`--copy-feedback-solid: ${copyFeedbackSolidMs}ms; --copy-feedback-fade: ${copyFeedbackFadeMs}ms`}
		aria-live="polite"
	>
		{#if copied}
			<Check size={15} strokeWidth={2.4} aria-hidden="true" />
			<span class="status-copy">{message}</span>
		{:else}
			<Copy size={14} strokeWidth={2} aria-hidden="true" />
		{/if}
	</span>
{/key}

<style>
	.copy-feedback {
		position: relative;
		display: grid;
		width: 1.25rem;
		height: 1.25rem;
		flex: none;
		margin-left: auto;
		place-items: center;
		color: var(--arcadia-muted);
		opacity: 0;
		transition:
			color 120ms,
			opacity 120ms;
	}

	:global(.copy-cell:hover) .copy-feedback,
	:global(.copy-cell:focus-visible) .copy-feedback,
	:global(.copy-row:hover) .copy-feedback,
	:global(.copy-row:focus-visible) .copy-feedback,
	.copy-feedback.copied {
		opacity: 1;
	}

	.copy-feedback.copied {
		animation: fade-copy-feedback var(--copy-feedback-fade) linear
			var(--copy-feedback-solid) forwards;
		color: var(--arcadia-positive);
	}

	@keyframes fade-copy-feedback {
		from {
			opacity: 1;
		}

		to {
			opacity: 0;
		}
	}

	.status-copy {
		position: absolute;
		width: 1px;
		height: 1px;
		overflow: hidden;
		clip: rect(0 0 0 0);
		white-space: nowrap;
		clip-path: inset(50%);
	}
</style>
