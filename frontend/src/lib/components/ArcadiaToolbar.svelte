<script lang="ts">
	import type { Snippet } from 'svelte';
	import { Search } from '@lucide/svelte';

	interface Props {
		query?: string;
		disabled?: boolean;
		placeholder?: string;
		searchLabel?: string;
		children?: Snippet;
	}

	let {
		query = $bindable(''),
		disabled = false,
		placeholder = 'Search',
		searchLabel = 'Search',
		children,
	}: Props = $props();
</script>

<div class="toolbar">
	<label class:disabled class="search-field">
		<Search size={17} aria-hidden="true" />
		<span class="sr-only">{searchLabel}</span>
		<input bind:value={query} type="search" {placeholder} {disabled} />
	</label>
	{#if children}
		<div class="toolbar-controls">{@render children()}</div>
	{/if}
</div>

<style>
	.toolbar {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 1rem;
		padding: 0.8rem;
	}

	.toolbar-controls {
		display: flex;
		align-items: center;
		gap: 0.45rem;
	}

	.search-field {
		display: flex;
		min-width: 0;
		width: min(20rem, 100%);
		align-items: center;
		gap: 0.55rem;
		border: 1px solid var(--arcadia-line);
		border-radius: 0.75rem;
		background: rgb(255 255 255 / 0.72);
		color: var(--arcadia-muted);
		padding: 0 0.7rem;
	}

	.search-field:focus-within {
		border-color: color-mix(in srgb, var(--arcadia-focus) 55%, white);
		box-shadow: 0 0 0 3px rgb(37 134 232 / 0.09);
	}

	.search-field input {
		width: 100%;
		border: 0;
		outline: 0;
		background: transparent;
		color: var(--site-surface-fg);
		font-size: 0.78rem;
		padding: 0.6rem 0;
	}

	.search-field.disabled {
		opacity: 0.48;
		pointer-events: none;
	}

	.sr-only {
		position: absolute;
		width: 1px;
		height: 1px;
		overflow: hidden;
		clip: rect(0 0 0 0);
		white-space: nowrap;
		clip-path: inset(50%);
	}

	@media (max-width: 500px) {
		.toolbar {
			gap: 0.55rem;
		}

		.search-field {
			width: 100%;
		}

		.toolbar-controls {
			gap: 0.3rem;
		}
	}
</style>
