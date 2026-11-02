<script lang="ts">
	import { page } from '$app/state';
	import { base } from '$app/paths';
	import { Power } from '@lucide/svelte';

	import { navigationItemIsActive, navigationItems } from '$lib/navigation';
	import type { ArcadiaSession } from '$lib/arcadia';

	interface Props {
		session: ArcadiaSession;
		onlogout: () => void | Promise<void>;
		children?: import('svelte').Snippet;
	}

	let { session, onlogout, children }: Props = $props();
	let loggingOut = $state(false);

	async function logout(): Promise<void> {
		loggingOut = true;
		try {
			await onlogout();
		} finally {
			loggingOut = false;
		}
	}
</script>

{#snippet identity()}
	<div class="identity">
		<a
			class="brand"
			href={`${base}/home`}
			aria-label={`Arcadia for ${session.user.email}`}
		>
			<strong>Arcadia</strong>
			<small>{session.user.email}</small>
		</a>
		<button
			class="logout-button"
			type="button"
			onclick={logout}
			disabled={loggingOut}
			aria-label={`Log out ${session.user.email}`}
			title="Log out"
		>
			<Power size={17} strokeWidth={2.2} />
		</button>
	</div>
{/snippet}

{#snippet navigation(compact = false)}
	<nav
		class:compact
		aria-label="Arcadia applications"
		data-sveltekit-preload-code="eager"
	>
		{#each navigationItems as item, index (item.href)}
			{#if !compact && item.section && (index === 0 || navigationItems[index - 1].section !== item.section)}
				<p class="section-label">{item.section}</p>
			{/if}
			{@const Icon = item.icon}
			<a
				class:active={navigationItemIsActive(page.url.pathname, item)}
				class="nav-item"
				href={item.href}
				aria-current={navigationItemIsActive(page.url.pathname, item)
					? 'page'
					: undefined}
			>
				<span class="nav-icon" aria-hidden="true">
					<Icon size={18} strokeWidth={2.2} />
				</span>
				<span class="nav-copy">
					<strong>{item.label}</strong>
				</span>
			</a>
		{/each}
	</nav>
{/snippet}

<div class="arcadia-shell">
	<aside class="desktop-sidebar">
		{@render identity()}
		{@render navigation()}
	</aside>

	<div class="arcadia-content">
		<header class="mobile-header">
			{@render identity()}
			{@render navigation(true)}
		</header>
		<main>{@render children?.()}</main>
	</div>
</div>

<style>
	.arcadia-shell {
		display: flex;
		min-height: 100vh;
	}

	.desktop-sidebar {
		position: sticky;
		top: 0;
		display: flex;
		width: var(--arcadia-sidebar-width);
		height: 100vh;
		box-sizing: border-box;
		flex: none;
		flex-direction: column;
		border-right: 1px solid var(--arcadia-sidebar-edge);
		background: linear-gradient(180deg, #104894, var(--arcadia-sidebar));
		box-shadow: 18px 0 50px -38px rgb(4 24 58 / 0.8);
		color: var(--site-chrome-fg);
		padding: 1.35rem 1rem;
	}

	.identity {
		display: flex;
		width: 100%;
		align-items: center;
		gap: 0.65rem;
	}

	.brand {
		display: block;
		min-width: 0;
		flex: 1;
		color: inherit;
		padding: 0.2rem 0.35rem;
		text-decoration: none;
	}

	.logout-button {
		display: grid;
		width: 2.25rem;
		height: 2.25rem;
		flex: none;
		place-items: center;
		border: 1px solid rgb(255 255 255 / 0.22);
		border-radius: 0.8rem;
		background: rgb(255 255 255 / 0.12);
		box-shadow: inset 0 1px rgb(255 255 255 / 0.15);
		color: white;
		cursor: pointer;
		transition: background 120ms;
	}

	.logout-button:hover {
		background: rgb(255 255 255 / 0.2);
	}

	.logout-button:disabled {
		cursor: wait;
		opacity: 0.65;
	}

	.brand strong,
	.brand small {
		display: block;
	}

	.brand strong {
		font-size: 1rem;
		letter-spacing: -0.02em;
	}

	.brand small {
		overflow: hidden;
		margin-top: 0.05rem;
		color: #a9c8ee;
		font-size: 0.72rem;
		font-weight: 700;
		letter-spacing: 0.02em;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.section-label {
		margin: 2rem 0.7rem 0.55rem;
		color: #8fafd7;
		font-size: 0.65rem;
		font-weight: 750;
		letter-spacing: 0.12em;
		text-transform: uppercase;
	}

	nav {
		display: grid;
		gap: 0.3rem;
	}

	.desktop-sidebar > nav {
		margin-top: 2.25rem;
	}

	.nav-item {
		display: flex;
		align-items: center;
		gap: 0.7rem;
		border: 1px solid transparent;
		border-radius: 0.8rem;
		color: #cfe2f9;
		padding: 0.62rem 0.7rem;
		text-decoration: none;
		transition:
			background 150ms,
			border-color 150ms,
			color 150ms;
	}

	.nav-item:hover {
		background: rgb(255 255 255 / 0.08);
		color: white;
	}

	.nav-item.active {
		border-color: rgb(143 247 255 / 0.16);
		background: rgb(143 247 255 / 0.12);
		color: white;
	}

	.nav-icon {
		display: grid;
		width: 1.75rem;
		height: 1.75rem;
		flex: none;
		place-items: center;
		border-radius: 0.55rem;
		background: rgb(255 255 255 / 0.08);
	}

	.nav-copy strong {
		display: block;
		font-size: 1rem;
		font-weight: 700;
	}

	.arcadia-content {
		min-width: 0;
		flex: 1;
	}

	.arcadia-content main {
		width: min(100%, 52rem);
		box-sizing: border-box;
		margin: 0 auto;
		padding: 2.35rem clamp(1rem, 4vw, 3.5rem) 4rem;
	}

	.mobile-header {
		display: none;
	}

	@media (max-width: 800px) {
		.arcadia-shell {
			display: block;
		}

		.desktop-sidebar {
			display: none;
		}

		.mobile-header {
			position: sticky;
			top: 0;
			z-index: 20;
			display: flex;
			align-items: center;
			justify-content: space-between;
			gap: 1rem;
			border-bottom: 1px solid var(--arcadia-sidebar-edge);
			background: rgb(11 50 107 / 0.96);
			box-shadow: 0 12px 30px -24px rgb(4 24 58 / 0.8);
			color: white;
			padding: 0.6rem 0.75rem;
			backdrop-filter: blur(18px);
		}

		.mobile-header .brand small {
			display: none;
		}

		.mobile-header .identity {
			min-width: 0;
			flex: 1;
		}

		.compact {
			min-width: 0;
			max-width: 100%;
			overflow-x: auto;
			display: flex;
			gap: 0.25rem;
			scrollbar-width: none;
		}

		.compact::-webkit-scrollbar {
			display: none;
		}

		.compact .nav-item {
			flex: none;
			gap: 0.4rem;
			padding: 0.42rem 0.55rem;
		}

		.compact .nav-icon {
			width: 1.5rem;
			height: 1.5rem;
			background: transparent;
		}

		.arcadia-content main {
			padding-top: 1.6rem;
		}
	}

	@media (max-width: 520px) {
		.compact .nav-copy {
			display: none;
		}
	}
</style>
