import { defineConfig } from '@rsbuild/core';
import { pluginSvelte } from '@rsbuild/plugin-svelte';

export default defineConfig({
  plugins: [pluginSvelte()],
  dev: {
    // See: https://github.com/sveltejs/svelte-loader?tab=readme-ov-file#hot-reload
    hmr: false,
  },
  tools: {
    rspack: {
      module: {
        rules: [
          {
            test: /\.(glsl)$/,
            type: 'asset/source',
          },
        ],
      },
    },
  },
});
