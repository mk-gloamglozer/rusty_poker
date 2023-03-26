<script lang="ts">
	import PoliceCap from '../images/police-cap.svelte';
	import Zombie from '../images/zombie.svelte';
	import Rocket from '../images/rocket.svelte';
	export let board: any;
</script>

<div class="stats shadow bg-base-300 stats-vertical relative">
	{#if !board.voting_complete}
		<div
			class="top-0 w-full h-full bg-base backdrop-blur-md absolute flex justify-center items-center"
		>
			<div class="bg-base-100 rounded-lg p-1 pl-3 pr-3">
				<h1 class="text-2xl">Vote Results</h1>
				<p class="text-base">Waiting for votes...</p>
			</div>
		</div>
	{/if}
	<div class="stat">
		<div class="stat-figure text-secondary">
			<PoliceCap class="w-10 h-10 fill-info" />
		</div>
		<div class="stat-title">Brutal Democracy</div>
		<div class="stat-value">{board.average || '?'}</div>
	</div>
	<div class="stat">
		<div class="stat-figure text-secondary">
			<Zombie class="w-10 h-10 fill-error" />
		</div>
		<div class="stat-title">Live in Fear</div>
		<div class="stat-value">{board.max || '?'}</div>
	</div>
	<div class="stat">
		<div class="stat-figure text-secondary">
			<Rocket class="w-10 h-10 fill-success" />
		</div>
		<div class="stat-title">Yeet That Mofo</div>
		<div class="stat-value">{board.min || '?'}</div>
	</div>
</div>
