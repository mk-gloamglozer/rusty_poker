import type { Actions, RequestEvent } from './$types';
import { redirect } from '@sveltejs/kit';

export const actions = {
	register: async (ctx: RequestEvent) => {
		const data = await ctx.request.formData();
		console.log(...data);
		ctx.cookies.set('session-user', data.get('name')?.toString() || '');

		console.log('searchParams', ctx.url.searchParams);
		throw redirect(307, '/');
	}
} satisfies Actions;
