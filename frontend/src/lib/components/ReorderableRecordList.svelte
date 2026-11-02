<script lang="ts" generics="Item extends { id: string }">
	import type { Snippet } from 'svelte';
	import { flip } from 'svelte/animate';
	import { tick } from 'svelte';
	import { Trash2 } from '@lucide/svelte';
	import { dndzone, type DndEvent } from 'svelte-dnd-action';

	interface Props {
		items: Item[];
		editing: boolean;
		dndType: string;
		label: string;
		itemLabel: (item: Item) => string;
		onreorder: (items: Item[]) => void;
		onfinalize?: (items: Item[]) => void | Promise<void>;
		ondelete: (id: string) => void | Promise<void>;
		row: Snippet<[Item]>;
		empty: Snippet;
	}

	let {
		items,
		editing,
		dndType,
		label,
		itemLabel,
		onreorder,
		onfinalize,
		ondelete,
		row,
		empty,
	}: Props = $props();
	const flipDurationMs = 180;
	let trash = $state<Item[]>([]);
	let reservedHeight = $state<number>();
	let listElement = $state<HTMLUListElement>();
	let wasEditing = false;
	let previousItemCount = 0;

	$effect(() => {
		const itemCount = items.length;
		if (!editing) {
			reservedHeight = undefined;
			trash = [];
		} else if (!wasEditing || itemCount > previousItemCount) {
			void reserveHeightAfterRender();
		}
		wasEditing = editing;
		previousItemCount = itemCount;
	});

	async function reserveHeightAfterRender(): Promise<void> {
		await tick();
		if (!editing) return;
		if (!listElement) throw new Error('Record list is not mounted.');
		reservedHeight = Math.max(
			reservedHeight ?? 0,
			listElement.getBoundingClientRect().height,
		);
	}

	function consider(event: CustomEvent<DndEvent<Item>>): void {
		onreorder(event.detail.items);
	}

	function finalize(event: CustomEvent<DndEvent<Item>>): void {
		onreorder(event.detail.items);
		void onfinalize?.(event.detail.items);
	}

	function updateTrash(event: CustomEvent<DndEvent<Item>>): void {
		trash = event.detail.items;
	}

	function deleteDropped(event: CustomEvent<DndEvent<Item>>): void {
		void ondelete(event.detail.info.id);
		trash = [];
	}
</script>

{#if items.length || editing}
	<ul
		bind:this={listElement}
		class:editing
		class="record-list"
		style:min-height={editing && reservedHeight !== undefined
			? `${reservedHeight}px`
			: undefined}
		use:dndzone={{
			items,
			type: dndType,
			flipDurationMs,
			dragDisabled: !editing,
			delayTouchStart: 100,
			useCursorForDetection: true,
			dropTargetStyle: {},
		}}
		onconsider={consider}
		onfinalize={finalize}
		aria-label={label}
	>
		{#each items as item (item.id)}
			<li
				animate:flip={{ duration: flipDurationMs }}
				aria-label={itemLabel(item)}
			>
				{@render row(item)}
			</li>
		{/each}
	</ul>
{:else}
	{@render empty()}
{/if}

{#if editing}
	<div class:active={trash.length > 0} class="delete-row">
		<div
			class="delete-dropzone"
			use:dndzone={{
				items: trash,
				type: dndType,
				flipDurationMs,
				morphDisabled: true,
				useCursorForDetection: true,
				dropTargetStyle: {},
			}}
			onconsider={updateTrash}
			onfinalize={deleteDropped}
			aria-label="Delete entry"
		>
			{#each trash as item (item.id)}
				<div
					class="delete-placeholder"
					aria-label={`Delete ${itemLabel(item)}`}
				></div>
			{/each}
		</div>
		<Trash2 size={18} aria-hidden="true" />
		<strong>{trash.length ? 'Release to delete' : 'Drag here to delete'}</strong
		>
	</div>
{/if}

<style>
	.record-list {
		margin: 0;
		padding: 0;
		list-style: none;
	}

	.record-list li {
		border-bottom: 1px solid var(--arcadia-line);
		background: white;
		padding: 0;
	}

	.record-list li:last-child {
		border-bottom: 0;
	}

	.record-list.editing li:focus-visible {
		outline: 3px solid rgb(37 134 232 / 0.2);
		outline-offset: -3px;
	}

	:global(#dnd-action-dragged-el) {
		overflow: hidden;
		border: 1px solid rgb(36 111 202 / 0.16);
		border-radius: 0.85rem;
		background: white;
		box-shadow: 0 14px 35px rgb(10 42 83 / 0.18);
	}

	.delete-row {
		position: relative;
		display: flex;
		min-height: 5rem;
		align-items: center;
		justify-content: center;
		gap: 0.5rem;
		border-top: 1px dashed rgb(190 52 52 / 0.24);
		background: rgb(190 52 52 / 0.025);
		color: #a33b3b;
		transition:
			background 140ms,
			box-shadow 140ms,
			color 140ms;
	}

	.delete-row strong {
		font-size: 0.76rem;
		font-weight: 700;
	}

	.delete-row.active {
		background: rgb(190 52 52 / 0.1);
		box-shadow: inset 0 0 0 2px rgb(190 52 52 / 0.22);
		color: #8f2929;
	}

	.delete-dropzone {
		position: absolute;
		z-index: 1;
		inset: 0;
	}

	.delete-dropzone:focus-visible {
		outline: 3px solid rgb(190 52 52 / 0.22);
		outline-offset: -3px;
	}

	.delete-placeholder {
		position: absolute;
		inset: 0;
		opacity: 0;
	}
</style>
