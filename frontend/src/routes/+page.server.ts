import type { PageServerLoad } from './$types';
import { redirect } from '@sveltejs/kit';

export const load = (async ({ cookies }) => {
	let board;
	board = await fetch('http://localhost:8080/board/1');
	board = await board.json();

	const name = cookies.get('session-user');
	if (!name) {
		throw redirect(307, '/register');
	}

	return {
		name: cookies.get('session-user') || '',
		board
	};
}) satisfies PageServerLoad;
