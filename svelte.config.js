import adapter from '@sveltejs/adapter-static';
import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';

const config = {
    preprocess: vitePreprocess(),
    kit: {
        adapter: adapter(),
        alias: {
            $components: './src/components',
            $types: './src/types',
            $lib: './src/lib',
            $assets: './src/assets',
        },
    },
};

export default config;
