<script lang="ts">
	interface Props {
		label: string;
		tone?: 'neutral' | 'positive' | 'warning';
	}

	let { label, tone = 'neutral' }: Props = $props();
</script>

<span class={`ui-status ui-status-${tone}`}>
	<span class="ui-status-dot" aria-hidden="true"></span>
	{label}
</span>
