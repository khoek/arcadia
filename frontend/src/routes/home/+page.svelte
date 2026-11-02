<script lang="ts">
	import { PageHeader, Panel } from '@arcadia/ui';
</script>

<svelte:head>
	<title>Home · Arcadia</title>
	<meta name="description" content="Arcadia home." />
</svelte:head>

<PageHeader title="Welcome" />

<Panel class="welcome-panel">
	<div class="welcome-card">
		<p class="welcome-kicker">Private workspace</p>
		<h2>Everything important, in one place.</h2>
		<p>
			Use the sidebar to manage your passwords, cards, and authenticator codes.
		</p>
	</div>
</Panel>

<style>
	:global(.welcome-panel) {
		display: grid;
		min-height: 18rem;
		box-sizing: border-box;
		place-items: center;
		margin-top: 1.5rem;
		padding: clamp(2rem, 7vw, 4rem);
		text-align: center;
	}

	.welcome-card {
		width: min(100%, 32rem);
	}

	.welcome-kicker {
		margin: 0 0 0.65rem;
		color: var(--site-link-fg);
		font-size: 0.68rem;
		font-weight: 750;
		letter-spacing: 0.14em;
		text-transform: uppercase;
	}

	.welcome-card h2 {
		margin: 0;
		color: var(--site-heading-fg);
		font-size: clamp(1.4rem, 3vw, 1.85rem);
		font-weight: 750;
		letter-spacing: -0.03em;
		line-height: 1.15;
	}

	.welcome-card p:last-child {
		max-width: 28rem;
		margin: 0.8rem auto 0;
		color: var(--arcadia-muted);
		font-size: 0.9rem;
		line-height: 1.6;
	}
</style>
