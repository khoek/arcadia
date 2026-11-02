<script lang="ts">
	interface Props {
		name: string;
		accent: string;
		iconImage?: string;
	}

	let { name, accent, iconImage }: Props = $props();
</script>

<div
	class="record-mark"
	style={`--record-accent: ${accent}`}
	aria-hidden="true"
>
	{#if iconImage}
		<img src={iconImage} alt="" />
	{:else}
		{name.slice(0, 1).toUpperCase()}
	{/if}
</div>

<style>
	.record-mark {
		display: grid;
		width: 2.35rem;
		height: 2.35rem;
		flex: none;
		overflow: hidden;
		place-items: center;
		border-radius: 0.78rem;
		background: color-mix(in srgb, var(--record-accent) 12%, white);
		color: var(--record-accent);
		font-size: 0.83rem;
		font-weight: 850;
	}

	.record-mark img {
		width: 100%;
		height: 100%;
		object-fit: cover;
	}
</style>
