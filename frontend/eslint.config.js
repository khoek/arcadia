import js from '@eslint/js';
import tsPlugin from '@typescript-eslint/eslint-plugin';
import tsParser from '@typescript-eslint/parser';
import svelte from 'eslint-plugin-svelte';
import globals from 'globals';

import svelteConfig from './svelte.config.js';

export default [
	{ ignores: ['build/', '.svelte-kit/', 'node_modules/', 'static/'] },
	js.configs.recommended,
	...tsPlugin.configs['flat/recommended'],
	...svelte.configs['flat/recommended'],
	...svelte.configs['flat/prettier'],
	{
		languageOptions: {
			globals: {
				...globals.browser,
				...globals.node,
			},
		},
	},
	{
		files: ['**/*.svelte', '**/*.svelte.ts', '**/*.svelte.js'],
		languageOptions: {
			parserOptions: {
				parser: tsParser,
				extraFileExtensions: ['.svelte'],
				svelteConfig,
			},
		},
	},
	{
		rules: {
			'svelte/no-navigation-without-resolve': 'off',
		},
	},
];
