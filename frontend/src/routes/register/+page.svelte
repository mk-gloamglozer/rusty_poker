<script>
	import Hero from '$lib/components/hero.svelte';
	import HeroComponent from '$lib/components/hero_component.svelte';
	import Content from '$lib/components/content.svelte';

	import { page } from '$app/stores';
	import { enhance } from '$app/forms';

	const redirect = $page.url.searchParams.get('redirect');
</script>

<Hero>
	<HeroComponent>
		<Content>
			<span slot="name">Identify Yourself</span>
			<p slot="divider-label">rebel scum</p>

			<div slot="main" class="w-64 md:rounded-lg text-base-content">
				<div class="p-4">
					<form class="flex flex-col gap-5" method="POST" action="?/register" use:enhance>
						<input type="text" name="name" placeholder="Name" class="w-full input bg-base-200" />
						<button type="submit" class="btn btn-primary">Submit</button>
					</form>
				</div>
			</div>
		</Content>
	</HeroComponent>
</Hero>
