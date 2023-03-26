<script lang="ts">
	import Tick from '$lib/images/tick.svelte';
	export let user: any;
	export let complete: boolean;

	$: {
		console.log('user', user);
	}
</script>

{#if complete && user.vote !== undefined}
	<div>{user.vote}</div>
{:else if user.vote !== undefined}<Tick class="w-5 h-5 fill-accent" />
{/if}
