<script>
	import Hero from '$lib/components/hero.svelte';
	import HeroComponent from '$lib/components/hero_component.svelte';
	import Content from '$lib/components/content.svelte';
</script>

<div class="bg-base-100 w-full pt-6">
	<Content>
		<div slot="name" class="text-error text-4xl p-3 rounded-lg">Error</div>
		<div slot="main" class="w-64 flex gap-2 items-center flex-col">
			<p class="text-xl">Something went wrong.</p>
			<p class="text-lg">This is your fault isn't it</p>
			<div class="pt-4">
				<button class="btn btn-outline btn-error">
					<div class="grow flex flex-col gap-1 text-base-content hover:text-inherit">
						<span>Accept</span><span>Responsibility</span>
					</div>
				</button>
			</div>
		</div>
	</Content>
</div>
