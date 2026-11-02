<script lang="ts">
	interface Props {
		class?: string;
		children?: import('svelte').Snippet;
	}

	let { class: className = '', children }: Props = $props();
</script>

<section class={`ui-panel ${className}`}>
	{@render children?.()}
</section>
