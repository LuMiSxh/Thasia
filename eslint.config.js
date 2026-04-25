import js from '@eslint/js';
import ts from 'typescript-eslint';
import svelte from 'eslint-plugin-svelte';
import svelteParser from 'svelte-eslint-parser';
import globals from 'globals';

export default [
    js.configs.recommended,
    ...ts.configs.recommended,
    ...svelte.configs['flat/recommended'],
    {
        languageOptions: {
            globals: { ...globals.browser, ...globals.node },
        },
    },
    {
        files: ['**/*.svelte', '**/*.svelte.ts', '**/*.svelte.js'],
        languageOptions: {
            parser: svelteParser,
            parserOptions: {
                parser: ts.parser,
            },
        },
    },
    {
        rules: {
            '@typescript-eslint/no-unused-vars': ['warn', { argsIgnorePattern: '^_' }],
            '@typescript-eslint/no-explicit-any': 'off',
            'no-console': 'warn',
            'svelte/no-navigation-without-resolve': 'off',
            'no-empty': ['error', { allowEmptyCatch: true }],
        },
    },
    {
        ignores: [
            'src-tauri/',
            'build/',
            '.svelte-kit/',
            'node_modules/',
            '**/bindings.ts',
            'target',
        ],
    },
];
