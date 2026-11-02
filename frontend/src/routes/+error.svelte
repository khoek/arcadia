<script lang="ts">
	import { page } from '$app/state';
	import { base } from '$app/paths';
	import { Panel } from '@arcadia/ui';
</script>

<svelte:head><title>{page.status} · Arcadia</title></svelte:head>

<Panel class="error-panel">
	<p class="error-code">{page.status}</p>
	<h1>Something went wrong</h1>
	<p>{page.error?.message ?? 'The requested page could not be loaded.'}</p>
	<a class="arcadia-button" href={`${base}/home/`}>Return to Home</a>
</Panel>

<style>
	:global(.error-panel) {
		max-width: 34rem;
		padding: 2rem;
	}

	.error-code {
		margin: 0;
		color: var(--site-link-fg);
		font-weight: 800;
	}

	h1 {
		margin: 0.4rem 0;
	}

	:global(.error-panel) > a {
		margin-top: 1rem;
	}
</style>
