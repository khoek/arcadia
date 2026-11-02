<script lang="ts">
	import { onMount } from 'svelte';
	import { SvelteMap } from 'svelte/reactivity';

	import ArcadiaNoticeHost from '$lib/components/ArcadiaNoticeHost.svelte';
	import ArcadiaShell from '$lib/components/ArcadiaShell.svelte';
	import { ArcadiaNotifications } from '$lib/arcadia-notifications.svelte';
	import {
		arcadiaSessionExpiredEvent,
		arcadiaApi,
		type ArcadiaSession,
	} from '$lib/arcadia';
	import { ArcadiaServices } from '$lib/services/arcadia-services';
	import {
		provideArcadiaNotifications,
		provideArcadiaServices,
	} from '$lib/services/context';

	import './app.css';

	interface Props {
		children?: import('svelte').Snippet;
	}

	let { children }: Props = $props();
	let checking = $state(true);
	let session = $state<ArcadiaSession>();
	const services = provideArcadiaServices(new ArcadiaServices(arcadiaApi));
	const notifications = provideArcadiaNotifications(new ArcadiaNotifications());
	const reportedServiceErrors = new SvelteMap<string, string>();

	$effect(() => {
		if (!session) {
			reportedServiceErrors.clear();
			return;
		}

		const serviceErrors = [
			['authenticator', services.authenticator.error],
			['cards', services.cards.error],
			['vault', services.vault.error],
		] as const;
		for (const [source, message] of serviceErrors) {
			if (!message) {
				reportedServiceErrors.delete(source);
				continue;
			}
			if (reportedServiceErrors.get(source) === message) continue;
			reportedServiceErrors.set(source, message);
			notifications.error(message);
		}
	});

	onMount(() => {
		const expireSession = (): void => {
			services.stop();
			session = undefined;
			checking = false;
			notifications.error(
				'Your IAP session expired. Please reload the arcadia.',
			);
		};
		window.addEventListener(arcadiaSessionExpiredEvent, expireSession);
		void loadSession();
		return () => {
			window.removeEventListener(arcadiaSessionExpiredEvent, expireSession);
			services.stop();
		};
	});

	async function loadSession(): Promise<void> {
		checking = true;
		try {
			session = (await arcadiaApi.getSession()) ?? undefined;
			if (session) services.start();
		} catch {
			notifications.error('The arcadia API is unavailable. Please try again.');
			session = undefined;
		} finally {
			checking = false;
		}
	}

	function logout(): void {
		window.location.assign('/_gcp_iap/clear_login_cookie');
	}
</script>

<svelte:head>
	{#if !checking && !session}<title>Private arcadia · Arcadia</title>{/if}
	<meta name="robots" content="noindex, nofollow, noarchive" />
	<meta name="theme-color" content="#0b326b" />
</svelte:head>

{#if checking}
	<main class="session-check" aria-label="Checking arcadia session">
		<span></span>
	</main>
{:else if session}
	<ArcadiaShell {session} onlogout={logout}>
		{@render children?.()}
	</ArcadiaShell>
{:else}
	<main class="access-required" aria-labelledby="access-required-title">
		<section>
			<p class="eyebrow">Arcadia</p>
			<h1 id="access-required-title">Private arcadia</h1>
			<p>
				This workspace is protected by Google Identity-Aware Proxy. Use the
				authorized Google account to continue.
			</p>
		</section>
	</main>
{/if}

<ArcadiaNoticeHost {notifications} />

<style>
	.session-check,
	.access-required {
		display: grid;
		min-height: 100dvh;
		place-items: center;
	}

	.access-required {
		padding: 2rem;
	}

	.access-required section {
		width: min(100%, 30rem);
		border: 1px solid var(--arcadia-line);
		border-radius: 0.3rem;
		background: rgb(255 255 255 / 0.82);
		box-shadow: 0 24px 70px -52px rgb(7 35 76 / 0.8);
		padding: clamp(2rem, 6vw, 3.6rem);
		text-align: center;
	}

	.access-required .eyebrow {
		margin: 0 0 1.1rem;
		color: var(--site-link-fg);
		font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
		font-size: 0.66rem;
		font-weight: 700;
		letter-spacing: 0.16em;
		text-transform: uppercase;
	}

	.access-required h1 {
		margin: 0;
		color: var(--site-heading-fg);
		font-family: Georgia, 'Times New Roman', serif;
		font-size: clamp(2rem, 7vw, 3rem);
		font-weight: 500;
		letter-spacing: -0.045em;
	}

	.access-required p:last-child {
		margin: 0.9rem auto 0;
		color: var(--arcadia-muted);
		font-size: 0.9rem;
		line-height: 1.6;
	}

	.session-check span {
		width: 1.25rem;
		height: 1.25rem;
		border: 2px solid rgb(36 111 202 / 0.16);
		border-top-color: var(--site-link-fg);
		border-radius: 50%;
		animation: session-spin 700ms linear infinite;
	}

	@keyframes session-spin {
		to {
			transform: rotate(360deg);
		}
	}
</style>
