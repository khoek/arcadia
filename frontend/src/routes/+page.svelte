<script lang="ts">
	import { base } from '$app/paths';
	import { onMount } from 'svelte';

	onMount(() => {
		window.location.replace(`${base}/home`);
	});
</script>

<svelte:head>
	<meta http-equiv="refresh" content={`0;url=${base}/home`} />
	<title>Arcadia</title>
</svelte:head>

<main>
	<p>Opening <a href={`${base}/home`}>Arcadia</a>…</p>
</main>
