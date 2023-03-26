<script lang="ts">
	import UserSymbol from './user_symbol.svelte';
	export let board: any;
</script>

<div class="flex m-4 flex-col gap-1 justify-center items-center">
	{#each board?.participants as user}
		<div class="tag w-full flex grow">
			<div class="grow rounded-r-sm bg-base-100">
				<div class="flex items-center justify-between m-1 ml-2 mr-2">
					{user.name}
					<UserSymbol {user} complete={board.voting_complete} />
				</div>
			</div>
		</div>
	{/each}
</div>

<style>
	.tag::before {
		content: '';
		display: inline-block;
		background-color: hsl(var(--p));
		width: 3px;
		@apply rounded-l-lg;
	}
</style>
