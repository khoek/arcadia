<script lang="ts">
	import { onMount } from 'svelte';
	import { CheckCircle2, CircleAlert, Info, X } from '@lucide/svelte';

	import type {
		ArcadiaNoticeKind,
		ArcadiaNotifications,
	} from '$lib/arcadia-notifications.svelte';

	interface Props {
		notifications: ArcadiaNotifications;
	}

	let { notifications }: Props = $props();
	let host = $state<HTMLDivElement>();
	let container = $state<Element>();

	$effect(() => {
		if (!host || !container) return;
		if (host.parentElement !== container) {
			host.hidePopover();
			container.append(host);
		}
		host.togglePopover(notifications.notices.length > 0);
	});

	onMount(() => {
		const home = host!.parentElement!;
		const syncContainer = (): void => {
			// Modal descendants remain interactive and exposed to assistive technology.
			container = document.querySelector('dialog:modal') ?? home;
		};
		const observer = new MutationObserver(syncContainer);
		observer.observe(document.body, {
			attributes: true,
			attributeFilter: ['open'],
			childList: true,
			subtree: true,
		});
		syncContainer();
		return () => {
			observer.disconnect();
			host!.hidePopover();
			home.append(host!);
			notifications.destroy();
		};
	});

	function iconFor(kind: ArcadiaNoticeKind) {
		return kind === 'success'
			? CheckCircle2
			: kind === 'error'
				? CircleAlert
				: Info;
	}
</script>

<div
	bind:this={host}
	class="notice-host"
	popover="manual"
	role="region"
	aria-label="Notifications"
>
	{#if notifications.notices.length > 0}
		{#each notifications.notices as notice (notice.id)}
			{@const Icon = iconFor(notice.kind)}
			<div
				class:success={notice.kind === 'success'}
				class:error={notice.kind === 'error'}
				class:info={notice.kind === 'info'}
				class="arcadia-notice"
				role={notice.kind === 'error' ? 'alert' : 'status'}
				aria-atomic="true"
			>
				<Icon
					class="notice-icon"
					size={18}
					strokeWidth={2.2}
					aria-hidden="true"
				/>
				<span>{notice.message}</span>
				<button
					class="notice-dismiss"
					type="button"
					onclick={() => notifications.dismiss(notice.id)}
					aria-label="Dismiss notification"
					title="Dismiss"
				>
					<X size={16} strokeWidth={2.2} aria-hidden="true" />
				</button>
			</div>
		{/each}
	{/if}
</div>

<style>
	.notice-host {
		position: fixed;
		inset: auto;
		bottom: calc(1.25rem + env(safe-area-inset-bottom));
		left: 50%;
		width: min(28rem, calc(100vw - 2rem));
		max-width: none;
		gap: 0.6rem;
		margin: 0;
		border: 0;
		background: transparent;
		color: inherit;
		overflow: visible;
		padding: 0;
		transform: translateX(-50%);
		pointer-events: none;
	}

	.notice-host:popover-open {
		display: grid;
	}

	.notice-host::backdrop {
		background: transparent;
	}

	.arcadia-notice {
		display: grid;
		grid-template-columns: auto minmax(0, 1fr) auto;
		align-items: center;
		gap: 0.65rem;
		min-height: 3.05rem;
		box-sizing: border-box;
		border: 1px solid;
		border-radius: 0.9rem;
		box-shadow: 0 14px 38px -18px rgb(5 24 58 / 0.55);
		font-size: 0.8rem;
		font-weight: 680;
		line-height: 1.35;
		padding: 0.7rem 0.7rem 0.7rem 0.85rem;
		pointer-events: auto;
		animation: arcadia-notice-in 180ms ease-out both;
		backdrop-filter: blur(16px);
	}

	.arcadia-notice.success {
		border-color: rgb(21 150 106 / 0.28);
		background: rgb(235 252 244 / 0.96);
		color: #0b704e;
	}

	.arcadia-notice.error {
		border-color: rgb(165 42 55 / 0.28);
		background: rgb(255 242 243 / 0.97);
		color: #8f2634;
	}

	.arcadia-notice.info {
		border-color: rgb(36 111 202 / 0.28);
		background: rgb(239 247 255 / 0.97);
		color: #18599a;
	}

	.notice-dismiss {
		display: grid;
		width: 1.9rem;
		height: 1.9rem;
		place-items: center;
		border: 0;
		border-radius: 0.55rem;
		background: transparent;
		color: currentColor;
		cursor: pointer;
		opacity: 0.68;
	}

	.notice-dismiss:hover {
		background: rgb(5 24 58 / 0.08);
		opacity: 1;
	}

	@keyframes arcadia-notice-in {
		from {
			opacity: 0;
			transform: translateY(0.5rem) scale(0.98);
		}
		to {
			opacity: 1;
			transform: translateY(0) scale(1);
		}
	}

	@media (prefers-reduced-motion: reduce) {
		.arcadia-notice {
			animation: none;
		}
	}

	@media (max-width: 520px) {
		.notice-host {
			bottom: calc(0.75rem + env(safe-area-inset-bottom));
			width: min(100% - 1rem, 28rem);
		}
	}
</style>
