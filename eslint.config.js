import js from '@eslint/js';
import globals from 'globals';
import tseslint from 'typescript-eslint';
import svelte from 'eslint-plugin-svelte';
import svelteParser from 'svelte-eslint-parser';
import prettierConfig from 'eslint-config-prettier';
import json from '@eslint/json';
import css from '@eslint/css';
import { defineConfig } from 'eslint/config';

export default defineConfig([
	// Global Ignores (Must be first without other keys)
	{
		ignores: ['build/', '.svelte-kit/', 'dist/', 'src-tauri/target/']
	},

	// Core JS/TS
	{
		files: ['**/*.{js,mjs,cjs,ts,mts,cts}'],
		plugins: { js },
		extends: ['js/recommended'],
		languageOptions: { globals: { ...globals.browser, ...globals.node } }
	},
	...tseslint.configs.recommended,

	// Svelte support (Scoped specifically to .svelte files)
	...svelte.configs['flat/recommended'].map((config) => ({
		...config,
		files: ['**/*.svelte']
	})),
	{
		files: ['**/*.svelte'],
		languageOptions: {
			parser: svelteParser,
			parserOptions: {
				parser: tseslint.parser,
				extraFileExtensions: ['.svelte']
			}
		}
	},

	// JSON & CSS
	{
		files: ['**/*.json'],
		plugins: { json },
		language: 'json/jsonc',
		extends: ['json/recommended']
	},
	{ files: ['**/*.css'], plugins: { css }, language: 'css/css', extends: ['css/recommended'] },

	// Turn off rules that conflict with Prettier (must be near the end)
	prettierConfig
]);
