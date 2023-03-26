const config = {
	content: ['./src/**/*.{html,js,svelte,ts}'],

	theme: {
		extend: {
			gridTemplateColumns: {
				main: 'minmax(0, 1fr) 1fr minmax(0, 1fr)'
			}
		}
	},

	plugins: [require('@tailwindcss/typography'), require('daisyui')]
};

module.exports = config;
