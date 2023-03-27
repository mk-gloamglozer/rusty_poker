<!-- src/routes/+page.svelte -->
<script lang="ts">
	import Tick from '$lib/images/tick.svelte';
	import Zombie from '$lib/images/zombie.svelte';
	import Rocket from '$lib/images/rocket.svelte';
	import PoliceCap from '$lib/images/police-cap.svelte';

	import Symbol from '$lib/components/user_symbol.svelte';
	import UserList from '$lib/components/user_list.svelte';

	import { env } from '$env/dynamic/public';

	import { onDestroy, onMount } from 'svelte';
	import type { PageData } from './$types';
	import Stats from '$lib/components/stats.svelte';

	const values = [1, 2, 3, 5, 8, 13];

	export let data: PageData;

	let board = data.board;

	let selected: number = -1;

	let socket: WebSocket;

	const vote = (i: number) => {
		console.log('voting', i);
		if (socket) {
			socket.send(JSON.stringify({ ParticipantVoted: { vote: i } }));
		}
	};

	$: {
		console.log('board', board.voting_complete);
	}

	onMount(() => {
		socket = new WebSocket(`ws://${env.PUBLIC_API_HOST}/${env.PUBLIC_API_URI}/ws/board/1?name=${data.name}`);

		// Connection opened
		socket.addEventListener('open', function (event) {
			console.log("It's open", event);
			console.log('and another thing');
		});

		// Listen for messages
		socket.addEventListener('message', function (event) {
			const data = JSON.parse(event.data);
			if (data.QueryUpdated) {
				board = data.QueryUpdated;
			}
			return true;
		});

		socket.addEventListener('close', function (event) {
			console.log("It's closed", event);
		});

		socket.addEventListener('ping', function (event) {
			console.log("It's pinged", event);
		});
	});

	onDestroy(() => {
		if (socket) {
			socket.close();
		}
	});

	const click = (i: number, value: number) => (e: any) => {
		if (selected === i) {
			selected = -1;
			vote(0);
		} else {
			selected = i;
			vote(value);
		}
	};
</script>

<div class="hero min-h-screen bg-gradient-to-br from-primary to-accent text-primary-content">
	<div class="hero-content max-w-5xl p-0 md:p-5">
		<div class="bg-base-100 grow md:rounded-lg text-base-content shadow-xl">
			<div class="m-4 mb-6">
				<div class="flex gap-1 flex-col justify-center items-start items-center ">
					<article class="prose">
						<h2 class="text-4xl">Your Vote Matters</h2>
					</article>
					<div class="divider grow"><p>probably</p></div>
					<div class="flex flex-wrap flex-row grow gap-5 sm:w-10/12">
						<div class="rounded-lg btn-group md:basis-full grow shadow-xl">
							<button class="btn btn-outline" on:click={click(selected, 0)}>Abstain</button>
							{#each values as value, i}
								<!-- <li data-content={value} class="step p-1 {i <= selected ? 'step-primary' : ''}"> -->
								<button
									class="btn grow {i <= selected ? 'btn-primary' : ''}"
									on:click={click(i, value)}>{value}</button
								>
								<div class="btn-divider" />
								<!-- </li> -->
							{/each}
							<button class="btn-accent btn">done</button>
						</div>
						<div class="bg-base-300 rounded-lg basis-1/3 shadow-lg">
							<UserList {board} />
						</div>
						<div class="flex flex-col grow">
							<Stats {board} />
						</div>
					</div>
				</div>
			</div>
		</div>
	</div>
</div>

<style>
	.step > .trigger {
		position: relative;
		display: grid;
		grid-column-start: 1;
		grid-row-start: 1;
		width: 2rem;
		height: 2rem;
		z-index: 2;
	}

	.overlay {
		position: absolute;
		top: 0;
		left: 0;
		width: 100%;
		height: 100%;
		background-color: hsl(var(--b1));
		opacity: 0.5;
		z-index: 1;
	}

	.tick > svg > path {
		fill: hsl(var(--su));
	}

	.btn-divider {
		position: relative;
		width: 1px;
		background-color: hsl(var(--b1));
	}

	.custom-step {
		grid-template-rows: 50px, 1fr;
		min-width: 5rem;
		min-height: 5rem;
	}

	.tag {
		overflow: visible;
		flex-direction: row;
	}
	.tag::before {
		content: '';
		display: inline-block;
		background-color: hsl(var(--p));
		width: 3px;
		@apply rounded-l-lg;
	}
	li.step::after {
		width: 3rem;
		height: 3rem;
	}

	li > button {
		position: relative;
		top: 0;
		left: 0;
		width: 100%;
		height: 100%;
		opacity: 0;
		z-index: 2;
	}
</style>
