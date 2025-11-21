import { defineConfig } from '@rsbuild/core';
import { pluginSass } from '@rsbuild/plugin-sass';
import { pluginSvelte } from '@rsbuild/plugin-svelte';

export default defineConfig({
  plugins: [pluginSvelte(), pluginSass()],
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
  performance: {
    chunkSplit: {
      override: {
        cacheGroups: {
          maplibre: {
            test: /maplibre-gl/,
            name: 'maplibre-shared',
            chunks: 'all',
          },
        },
      },
    },
  },
});
