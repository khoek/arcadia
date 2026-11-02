<script lang="ts">
	interface Props {
		kicker?: string;
		title: string;
		description?: string;
		actions?: import('svelte').Snippet;
	}

	let { kicker, title, description, actions }: Props = $props();
</script>

<header class="ui-page-header">
	<div class="ui-page-heading">
		{#if kicker}<p class="ui-kicker">{kicker}</p>{/if}
		<h1>{title}</h1>
		{#if description}<p class="ui-page-description">{description}</p>{/if}
	</div>
	{#if actions}<div class="ui-page-actions">{@render actions()}</div>{/if}
</header>
