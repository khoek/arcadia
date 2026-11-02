import adapter from '@sveltejs/adapter-static';
import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';

/** @type {import('@sveltejs/kit').Config} */
const config = {
	preprocess: vitePreprocess(),
	kit: {
		adapter: adapter(),
		csp: {
			mode: 'hash',
			directives: {
				'default-src': ['self'],
				'connect-src': ['self', 'self'],
				'img-src': ['self', 'data:', 'blob:'],
				'font-src': ['self'],
				'object-src': ['none'],
				'base-uri': ['none'],
				'form-action': ['self'],
				'style-src': ['self', 'unsafe-inline'],
				'upgrade-insecure-requests': true,
			},
		},
	},
};

export default config;
