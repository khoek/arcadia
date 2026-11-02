<script lang="ts">
	import type { Snippet } from 'svelte';

	interface Props {
		title: string;
		description: string;
		icon: Snippet;
	}

	let { title, description, icon }: Props = $props();
</script>

<div class="empty-state">
	{@render icon()}
	<strong>{title}</strong>
	<span>{description}</span>
</div>

<style>
	.empty-state {
		display: grid;
		min-height: 14rem;
		place-items: center;
		align-content: center;
		color: var(--arcadia-muted);
		padding: 2rem;
		text-align: center;
	}

	.empty-state strong {
		margin-top: 0.7rem;
		color: var(--site-heading-fg);
		font-size: 0.86rem;
	}

	.empty-state span {
		margin-top: 0.2rem;
		font-size: 0.72rem;
	}
</style>
